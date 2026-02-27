use serde::{Deserialize, Serialize};

use crate::error::LockpickError;
use crate::{DependencyGraph, Severity, VulnReport, Vulnerability};

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const OSV_BATCH_SIZE: usize = 1000;

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

/// Default cache TTL: 1 hour
const DEFAULT_CACHE_TTL: u64 = 3600;

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
        if use_cache {
            if let Some(vulns) = crate::cache::osv::get(name, version, ttl) {
                if !vulns.is_empty() {
                    cached_reports.push(VulnReport {
                        package: name.clone(),
                        version: version.clone(),
                        vulns,
                    });
                }
                continue;
            }
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

    // Send queries in batches to avoid API limits
    let mut all_results: Vec<OsvResultEntry> = Vec::new();
    for chunk in queries.chunks(OSV_BATCH_SIZE) {
        let request = OsvBatchRequest {
            queries: chunk.to_vec(),
        };

        let resp = client
            .post(OSV_BATCH_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| LockpickError::Network(format!("OSV API request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(LockpickError::Network(format!(
                "OSV API returned status: {}",
                resp.status()
            )));
        }

        let batch: OsvBatchResponse = resp
            .json()
            .await
            .map_err(|e| LockpickError::Network(format!("Failed to parse OSV response: {e}")))?;

        all_results.extend(batch.results);
    }

    // Map fresh results back to packages and write to cache
    let mut fresh_reports: Vec<VulnReport> = Vec::new();
    for (i, entry) in all_results.iter().enumerate() {
        let Some((name, version)) = pkg_order.get(i) else {
            continue;
        };

        let vulns: Vec<Vulnerability> = entry.vulns.iter().map(convert_vuln).collect();

        // Write to cache (even empty results, so we don't re-query clean packages)
        if use_cache {
            let _ = crate::cache::osv::set(name, version, &vulns);
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
    // CVSS vector format: "CVSS:3.1/AV:N/AC:L/..." — no embedded score,
    // but some APIs put the score in a separate `score` field.
    // Try extracting trailing float after last '/' if present
    if score.starts_with("CVSS:") {
        // Vector string itself doesn't contain a numeric score
        return None;
    }
    None
}
