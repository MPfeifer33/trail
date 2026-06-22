use std::path::Path;
use serde::Serialize;

/// Evidence sources found in the project.
#[derive(Debug, Serialize)]
pub struct EvidenceSources {
    pub witness: bool,
    pub latch: bool,
    pub probe: bool,
    pub atlas: bool,
}

/// Evidence correlated to a specific commit.
#[derive(Debug, Default, Serialize)]
pub struct CommitEvidence {
    /// Witness evidence bundles recorded near this commit
    pub witness_runs: Vec<WitnessRef>,
    /// Latch decisions/notes active at this commit
    pub latch_context: Option<String>,
    /// Probe snapshot near this commit
    pub probe_snapshot: bool,
}

#[derive(Debug, Serialize)]
pub struct WitnessRef {
    pub id: String,
    pub command: String,
    pub exit_code: i32,
    pub tag: Option<String>,
}

pub fn detect_sources(repo: &Path) -> EvidenceSources {
    EvidenceSources {
        witness: repo.join(".agent-witness").join("evidence").exists(),
        latch: repo.join(".latch.db").exists(),
        probe: repo.join(".agent-probe").exists(),
        atlas: repo.join(".agent-atlas").exists(),
    }
}

/// Find witness evidence bundles that were recorded near the commit timestamp.
pub fn find_witness_evidence(repo: &Path, commit_date: &str) -> Vec<WitnessRef> {
    let evidence_dir = repo.join(".agent-witness").join("evidence");
    if !evidence_dir.exists() {
        return Vec::new();
    }

    let mut refs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&evidence_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(evidence) = serde_json::from_str::<serde_json::Value>(&content) {
                    let ev_timestamp = evidence["timestamp"].as_str().unwrap_or("");

                    // Simple date proximity check — same day
                    let commit_day = &commit_date[..10]; // "2026-06-22"
                    let ev_day = if ev_timestamp.len() >= 10 { &ev_timestamp[..10] } else { "" };

                    if commit_day == ev_day {
                        refs.push(WitnessRef {
                            id: evidence["id"].as_str().unwrap_or("").to_string(),
                            command: evidence["command"].as_str().unwrap_or("").to_string(),
                            exit_code: evidence["exit_code"].as_i64().unwrap_or(-1) as i32,
                            tag: evidence["tag"].as_str().map(|s| s.to_string()),
                        });
                    }
                }
            }
        }
    }

    refs
}

/// Check for latch context near a commit.
pub fn find_latch_context(repo: &Path) -> Option<String> {
    let db_path = repo.join(".latch.db");
    if !db_path.exists() {
        return None;
    }
    // For MVP, just note that latch is present. Full integration would query the DB.
    Some("Latch coordination ledger is active in this project.".to_string())
}
