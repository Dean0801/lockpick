use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::error::LockpickError;
use crate::{DependencyGraph, Severity, VulnReport, Vulnerability};

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const OSV_BATCH_SIZE: usize = 1000;
const MAX_RETRIES: u32 = 3;

/// OSV batch query request
#[derive(Serialize)]
struct OsvBatchRequest {
    queries: Vec<OsvQuery>,
}

#[derive(Serialize, Clone)]
struct OsvQuery {
    package: OsvPackage,
    version: String,
}

#[derive(Serialize, Clone)]
struct OsvPackage {
    name: String,
    ecosystem: String,
}

/// OSV batch query response
#[derive(Deserialize)]
struct OsvBatchResponse {
    results: Vec<OsvResultEntry>,
}

#[derive(Deserialize)]
struct OsvResultEntry {
    #[serde(default)]
    vulns: Vec<OsvVuln>,
}

#[derive(Deserialize)]
struct OsvVuln {
    id: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    severity: Vec<OsvSeverity>,
    #[serde(default)]
    affected: Vec<OsvAffected>,
}

#[derive(Deserialize)]
struct OsvSeverity {
    #[serde(rename = "type")]
    severity_type: String,
    score: String,
}

#[derive(Deserialize)]
struct OsvAffected {
    #[serde(default)]
    ranges: Vec<OsvRange>,
}

#[derive(Deserialize)]
struct OsvRange {
    #[serde(default)]
    events: Vec<OsvEvent>,
}

#[derive(Deserialize)]
struct OsvEvent {
    #[serde(default)]
    fixed: Option<String>,
}

/// Default cache TTL: 24 hours (matches config documentation)
const DEFAULT_CACHE_TTL: u64 = 86400;

/// Scan dependencies for known vulnerabilities via OSV.dev API
pub async fn scan_vulnerabilities(
    graph: &DependencyGraph,
    cache_ttl: Option<u64>,
    no_cache: bool,
) -> Result<Vec<VulnReport>, LockpickError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| LockpickError::Network(format!("HTTP client error: {e}")))?;

    let use_cache = !no_cache;
    let ttl = cache_ttl.unwrap_or(DEFAULT_CACHE_TTL);

    // Collect all packages to scan
    let all_pkgs: Vec<(String, String)> = graph
        .dependencies
        .iter()
        .chain(graph.dev_dependencies.iter())
        .map(|(name, info)| (name.clone(), info.version.clone()))
        .collect();

    if all_pkgs.is_empty() {
        return Ok(Vec::new());
    }

    // Check cache per-package, separate into cached hits and uncached misses
    let mut cached_reports: Vec<VulnReport> = Vec::new();
    let mut uncached: Vec<(String, String)> = Vec::new();

    for (name, version) in &all_pkgs {
        if use_cache && let Some(vulns) = crate::cache::osv::get(name, version, ttl) {
            if !vulns.is_empty() {
                cached_reports.push(VulnReport {
                    package: name.clone(),
                    version: version.clone(),
                    vulns,
                });
            }
            continue;
        }
        uncached.push((name.clone(), version.clone()));
    }

    // Build batch queries only for uncached packages
    let mut queries = Vec::new();
    let mut pkg_order = Vec::new();

    for (name, version) in &uncached {
        queries.push(OsvQuery {
            package: OsvPackage {
                name: name.clone(),
                ecosystem: "npm".into(),
            },
            version: version.clone(),
        });
        pkg_order.push((name.clone(), version.clone()));
    }

    // Send queries in batches to avoid API limits.
    // Failed batches are skipped with a warning so that successful results are preserved.
    let mut all_results: Vec<OsvResultEntry> = Vec::new();
    let batches: Vec<_> = queries.chunks(OSV_BATCH_SIZE).collect();
    let pb = ProgressBar::new(batches.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{bar:30}] {pos}/{len} batches ({eta})")
            .unwrap_or_else(|_| ProgressStyle::default_bar()),
    );
    for (batch_idx, chunk) in batches.iter().enumerate() {
        let request = OsvBatchRequest {
            queries: chunk.to_vec(),
        };

        match send_batch_with_retry(&client, &request).await {
            Ok(batch) => {
                all_results.extend(batch.results);
            }
            Err(e) => {
                eprintln!(
                    "warning: OSV batch {} failed after retries, skipping ({e})",
                    batch_idx + 1
                );
                // Fill placeholder empty entries so pkg_order indices stay aligned
                all_results.extend((0..chunk.len()).map(|_| OsvResultEntry { vulns: Vec::new() }));
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    // Map fresh results back to packages and write to cache
    let mut fresh_reports: Vec<VulnReport> = Vec::new();
    for (i, entry) in all_results.iter().enumerate() {
        let Some((name, version)) = pkg_order.get(i) else {
            continue;
        };

        let vulns: Vec<Vulnerability> = entry.vulns.iter().map(convert_vuln).collect();

        // Write to cache (even empty results, so we don't re-query clean packages)
        if use_cache && let Err(e) = crate::cache::osv::set(name, version, &vulns) {
            eprintln!("warning: failed to write OSV cache for {name}@{version}: {e}");
        }

        if !vulns.is_empty() {
            fresh_reports.push(VulnReport {
                package: name.clone(),
                version: version.clone(),
                vulns,
            });
        }
    }

    // Merge cached + fresh results
    let mut reports = cached_reports;
    reports.extend(fresh_reports);
    Ok(reports)
}

/// Send a single OSV batch request with exponential backoff retry.
/// Retries on network errors and 5xx responses; 4xx errors fail immediately.
async fn send_batch_with_retry(
    client: &reqwest::Client,
    request: &OsvBatchRequest,
) -> Result<OsvBatchResponse, LockpickError> {
    let mut last_err = LockpickError::Network("unknown error".into());
    let total_attempts = MAX_RETRIES + 1; // 1 initial + 3 retries

    for attempt in 0..total_attempts {
        if attempt > 0 {
            let delay = Duration::from_secs(1 << (attempt - 1)); // 1s, 2s, 4s
            eprintln!(
                "warning: OSV API request failed, retrying in {}s (attempt {}/{})",
                delay.as_secs(),
                attempt,
                MAX_RETRIES
            );
            sleep(delay).await;
        }

        let resp = match client.post(OSV_BATCH_URL).json(request).send().await {
            Ok(r) => r,
            Err(e) => {
                // Network-level error (timeout, DNS, connection reset) — retryable
                last_err = LockpickError::Network(format!("OSV API request failed: {e}"));
                continue;
            }
        };

        let status = resp.status();

        // 4xx client errors are not retryable
        if status.is_client_error() {
            return Err(LockpickError::Network(format!(
                "OSV API returned client error: {status}"
            )));
        }

        // 5xx server errors are retryable
        if status.is_server_error() {
            last_err = LockpickError::Network(format!("OSV API returned server error: {status}"));
            continue;
        }

        // Success — parse the response body
        let batch: OsvBatchResponse = resp
            .json()
            .await
            .map_err(|e| LockpickError::Network(format!("Failed to parse OSV response: {e}")))?;

        return Ok(batch);
    }

    Err(last_err)
}

fn convert_vuln(v: &OsvVuln) -> Vulnerability {
    let severity = parse_severity(v);
    let fixed_version = v
        .affected
        .iter()
        .flat_map(|a| &a.ranges)
        .flat_map(|r| &r.events)
        .filter_map(|e| e.fixed.clone())
        .next();

    Vulnerability {
        id: v.id.clone(),
        summary: v.summary.clone(),
        severity,
        fixed_version,
    }
}

fn parse_severity(v: &OsvVuln) -> Severity {
    // Try to extract CVSS score
    for s in &v.severity {
        if s.severity_type == "CVSS_V3"
            && let Some(score) = extract_cvss_score(&s.score)
        {
            return match score {
                s if s >= 9.0 => Severity::Critical,
                s if s >= 7.0 => Severity::High,
                s if s >= 4.0 => Severity::Medium,
                _ => Severity::Low,
            };
        }
    }
    // Default to Medium if no CVSS info
    Severity::Medium
}

/// Extract numeric score from CVSS vector string or plain number
fn extract_cvss_score(score: &str) -> Option<f64> {
    // Try plain number first: "7.5"
    if let Ok(v) = score.parse::<f64>() {
        return Some(v);
    }
    // CVSS 3.x vector format: compute base score from metrics
    if score.starts_with("CVSS:") {
        return compute_cvss3_base_score(score);
    }
    None
}

/// Compute CVSS 3.x Base Score from a vector string.
/// Implements the standard formula from https://www.first.org/cvss/v3.1/specification-document
fn compute_cvss3_base_score(vector: &str) -> Option<f64> {
    use std::collections::HashMap;

    // Parse metrics from vector like "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
    let mut metrics: HashMap<&str, &str> = HashMap::new();
    for part in vector.split('/') {
        if let Some((key, val)) = part.split_once(':') {
            metrics.insert(key, val);
        }
    }

    // All 8 base metrics are required
    let av = metrics.get("AV")?;
    let ac = metrics.get("AC")?;
    let pr = metrics.get("PR")?;
    let ui = metrics.get("UI")?;
    let s = metrics.get("S")?;
    let c = metrics.get("C")?;
    let i = metrics.get("I")?;
    let a = metrics.get("A")?;

    let scope_changed = *s == "C";

    // Attack Vector
    let av_score = match *av {
        "N" => 0.85,
        "A" => 0.62,
        "L" => 0.55,
        "P" => 0.20,
        _ => return None,
    };

    // Attack Complexity
    let ac_score = match *ac {
        "L" => 0.77,
        "H" => 0.44,
        _ => return None,
    };

    // Privileges Required (depends on Scope)
    let pr_score = match (*pr, scope_changed) {
        ("N", _) => 0.85,
        ("L", false) => 0.62,
        ("L", true) => 0.68,
        ("H", false) => 0.27,
        ("H", true) => 0.50,
        _ => return None,
    };

    // User Interaction
    let ui_score = match *ui {
        "N" => 0.85,
        "R" => 0.62,
        _ => return None,
    };

    // Impact metrics (C/I/A)
    let c_score = match *c {
        "H" => 0.56,
        "L" => 0.22,
        "N" => 0.0,
        _ => return None,
    };
    let i_score = match *i {
        "H" => 0.56,
        "L" => 0.22,
        "N" => 0.0,
        _ => return None,
    };
    let a_score = match *a {
        "H" => 0.56,
        "L" => 0.22,
        "N" => 0.0,
        _ => return None,
    };

    // Exploitability sub-score
    let exploitability = 8.22 * av_score * ac_score * pr_score * ui_score;

    // Impact sub-score
    let iss = 1.0 - (1.0 - c_score) * (1.0 - i_score) * (1.0 - a_score);

    let impact: f64 = if scope_changed {
        7.52 * (iss - 0.029) - 3.25 * (iss - 0.02_f64).powf(15.0)
    } else {
        6.42 * iss
    };

    if impact <= 0.0 {
        return Some(0.0);
    }

    let score: f64 = if scope_changed {
        1.08 * (impact + exploitability)
    } else {
        impact + exploitability
    };

    // Cap at 10.0 and apply roundup
    let capped = score.min(10.0);
    Some(roundup(capped))
}

/// CVSS roundup: smallest number >= input with one decimal place
fn roundup(input: f64) -> f64 {
    let int_input = (input * 100_000.0).round() as i64;
    if int_input % 10000 == 0 {
        (int_input as f64) / 100_000.0
    } else {
        ((int_input / 10000 + 1) as f64) / 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cvss_plain_number() {
        assert_eq!(extract_cvss_score("7.5"), Some(7.5));
        assert_eq!(extract_cvss_score("9.8"), Some(9.8));
        assert_eq!(extract_cvss_score("0.0"), Some(0.0));
    }

    #[test]
    fn test_cvss3_critical_vector() {
        // AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H → 9.8
        let score =
            compute_cvss3_base_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").unwrap();
        assert!((score - 9.8).abs() < 0.1, "expected ~9.8, got {score}");
    }

    #[test]
    fn test_cvss3_medium_vector() {
        // AV:N/AC:L/PR:L/UI:N/S:U/C:L/I:L/A:N → 5.4
        let score =
            compute_cvss3_base_score("CVSS:3.1/AV:N/AC:L/PR:L/UI:N/S:U/C:L/I:L/A:N").unwrap();
        assert!((score - 5.4).abs() < 0.1, "expected ~5.4, got {score}");
    }

    #[test]
    fn test_cvss3_scope_changed() {
        // AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H → 10.0
        let score =
            compute_cvss3_base_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H").unwrap();
        assert!((score - 10.0).abs() < 0.1, "expected ~10.0, got {score}");
    }

    #[test]
    fn test_cvss3_low_vector() {
        // AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N → 1.8
        let score =
            compute_cvss3_base_score("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N").unwrap();
        assert!((score - 1.8).abs() < 0.2, "expected ~1.8, got {score}");
    }

    #[test]
    fn test_cvss3_zero_impact() {
        // All impact metrics None → 0.0
        let score =
            compute_cvss3_base_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N").unwrap();
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_cvss3_incomplete_vector() {
        // Missing required metrics → None
        assert!(compute_cvss3_base_score("CVSS:3.1/AV:N/AC:L").is_none());
    }

    #[test]
    fn test_cvss3_invalid_metric_value() {
        assert!(compute_cvss3_base_score("CVSS:3.1/AV:X/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H").is_none());
    }

    #[test]
    fn test_extract_cvss_score_with_vector() {
        // Integration: extract_cvss_score should now handle vectors
        let score = extract_cvss_score("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H");
        assert!(score.is_some());
        assert!((score.unwrap() - 9.8).abs() < 0.1);
    }

    #[test]
    fn test_parse_severity_with_cvss_vector() {
        let vuln = OsvVuln {
            id: "TEST-001".into(),
            summary: "test".into(),
            severity: vec![OsvSeverity {
                severity_type: "CVSS_V3".into(),
                score: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H".into(),
            }],
            affected: vec![],
        };
        assert_eq!(parse_severity(&vuln), Severity::Critical);
    }

    #[test]
    fn test_roundup() {
        assert_eq!(roundup(0.0), 0.0);
        assert_eq!(roundup(4.0), 4.0);
        assert_eq!(roundup(4.02), 4.1);
        assert_eq!(roundup(4.1), 4.1);
    }
}
