use serde::{Deserialize, Serialize};

use crate::{DependencyGraph, Severity, VulnReport, Vulnerability};

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";

/// OSV batch query request
#[derive(Serialize)]
struct OsvBatchRequest {
    queries: Vec<OsvQuery>,
}

#[derive(Serialize)]
struct OsvQuery {
    package: OsvPackage,
    version: String,
}

#[derive(Serialize)]
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

/// Scan dependencies for known vulnerabilities via OSV.dev API
pub async fn scan_vulnerabilities(graph: &DependencyGraph) -> Result<Vec<VulnReport>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    // Build batch query from all deps
    let mut queries = Vec::new();
    let mut pkg_order = Vec::new();

    for (name, info) in &graph.dependencies {
        queries.push(OsvQuery {
            package: OsvPackage {
                name: name.clone(),
                ecosystem: "npm".into(),
            },
            version: info.version.clone(),
        });
        pkg_order.push((name.clone(), info.version.clone()));
    }

    for (name, info) in &graph.dev_dependencies {
        queries.push(OsvQuery {
            package: OsvPackage {
                name: name.clone(),
                ecosystem: "npm".into(),
            },
            version: info.version.clone(),
        });
        pkg_order.push((name.clone(), info.version.clone()));
    }

    if queries.is_empty() {
        return Ok(Vec::new());
    }

    let request = OsvBatchRequest { queries };

    let resp = client
        .post(OSV_BATCH_URL)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("OSV API request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("OSV API returned status: {}", resp.status()));
    }

    let batch: OsvBatchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse OSV response: {e}"))?;

    // Map results back to packages
    let mut reports = Vec::new();
    for (i, entry) in batch.results.iter().enumerate() {
        if entry.vulns.is_empty() {
            continue;
        }
        let (name, version) = &pkg_order[i];
        let vulns = entry.vulns.iter().map(convert_vuln).collect();
        reports.push(VulnReport {
            package: name.clone(),
            version: version.clone(),
            vulns,
        });
    }

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

/// Extract numeric score from CVSS vector string
fn extract_cvss_score(score: &str) -> Option<f64> {
    // CVSS vector format: "CVSS:3.1/AV:N/AC:L/..." or just a number
    score.parse::<f64>().ok()
}
