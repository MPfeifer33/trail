use crate::evidence::{CommitEvidence, EvidenceSources};
use crate::git::CommitInfo;
use crate::TrailError;

pub fn print_log(
    commits: &[CommitInfo],
    evidence: &[CommitEvidence],
    is_json: bool,
) -> Result<(), TrailError> {
    if is_json {
        let entries: Vec<serde_json::Value> = commits.iter().zip(evidence.iter())
            .map(|(c, e)| serde_json::json!({
                "sha": c.sha,
                "short_sha": c.short_sha,
                "author": c.author,
                "date": c.date,
                "subject": c.subject,
                "files_changed": c.files_changed.len(),
                "insertions": c.insertions,
                "deletions": c.deletions,
                "evidence": e,
            }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "commits": entries,
        }))?);
    } else {
        for (commit, ev) in commits.iter().zip(evidence.iter()) {
            let stats = format!("+{} -{} ({} files)",
                commit.insertions, commit.deletions, commit.files_changed.len());

            println!("{} {} ({})", commit.short_sha, commit.subject, commit.author);
            println!("  {} | {}", &commit.date[..16], stats);

            if !ev.witness_runs.is_empty() {
                for w in &ev.witness_runs {
                    let icon = if w.exit_code == 0 { "✓" } else { "✗" };
                    let tag_str = w.tag.as_deref().map(|t| format!(" [{t}]")).unwrap_or_default();
                    println!("  {icon} witness: `{}`{}", truncate(&w.command, 50), tag_str);
                }
            }

            println!();
        }
    }
    Ok(())
}

pub fn print_summary(commits: &[CommitInfo], evidence: &[CommitEvidence], is_json: bool) -> Result<(), TrailError> {
    let total_files: usize = commits.iter().map(|c| c.files_changed.len()).sum();
    let total_insertions: usize = commits.iter().map(|c| c.insertions).sum();
    let total_deletions: usize = commits.iter().map(|c| c.deletions).sum();
    let witness_count: usize = evidence.iter().map(|e| e.witness_runs.len()).sum();

    let authors: Vec<String> = {
        let mut a: Vec<String> = commits.iter().map(|c| c.author.clone()).collect();
        a.sort();
        a.dedup();
        a
    };

    if is_json {
        let recent: Vec<serde_json::Value> = commits.iter().take(5)
            .map(|c| serde_json::json!({
                "sha": c.sha,
                "short_sha": c.short_sha,
                "author": c.author,
                "date": c.date,
                "subject": c.subject,
            }))
            .collect();

        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "summary": {
                "commits": commits.len(),
                "authors": authors,
                "files_changed": total_files,
                "insertions": total_insertions,
                "deletions": total_deletions,
                "witness_evidence": witness_count,
                "date_range": {
                    "oldest": commits.last().map(|c| &c.date),
                    "newest": commits.first().map(|c| &c.date),
                },
                "recent": recent,
            }
        }))?);
    } else {
        println!("trail summary: {} commits by {} author(s)", commits.len(), authors.len());
        println!();

        if let (Some(newest), Some(oldest)) = (commits.first(), commits.last()) {
            println!("  Period: {} to {}", &oldest.date[..16], &newest.date[..16]);
        }

        println!("  Authors: {}", authors.join(", "));
        println!("  Changes: +{total_insertions} -{total_deletions} across {total_files} file edits");

        if witness_count > 0 {
            println!("  Evidence: {witness_count} witness recording(s)");
        }

        println!();
        println!("  Recent:");
        for commit in commits.iter().take(5) {
            println!("    {} {}", commit.short_sha, commit.subject);
        }
        if commits.len() > 5 {
            println!("    ... and {} more", commits.len() - 5);
        }
    }
    Ok(())
}

pub fn print_commit_detail(
    commit: &CommitInfo,
    evidence: &CommitEvidence,
    is_json: bool,
) -> Result<(), TrailError> {
    if is_json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "commit": {
                "sha": commit.sha,
                "short_sha": commit.short_sha,
                "author": commit.author,
                "date": commit.date,
                "subject": commit.subject,
                "body": commit.body,
                "files_changed": commit.files_changed,
                "insertions": commit.insertions,
                "deletions": commit.deletions,
            },
            "evidence": evidence,
        }))?);
    } else {
        println!("trail show: {}", commit.short_sha);
        println!();
        println!("  Subject: {}", commit.subject);
        println!("  Author: {}", commit.author);
        println!("  Date: {}", commit.date);
        println!("  Stats: +{} -{} ({} files)", commit.insertions, commit.deletions, commit.files_changed.len());

        if !commit.body.trim().is_empty() {
            println!();
            println!("  Message:");
            for line in commit.body.trim().lines() {
                println!("    {line}");
            }
        }

        if !commit.files_changed.is_empty() {
            println!();
            println!("  Files:");
            for f in &commit.files_changed {
                println!("    +{:<4} -{:<4} {}", f.additions, f.deletions, f.path);
            }
        }

        if !evidence.witness_runs.is_empty() {
            println!();
            println!("  Witness evidence:");
            for w in &evidence.witness_runs {
                let icon = if w.exit_code == 0 { "✓" } else { "✗" };
                let tag_str = w.tag.as_deref().map(|t| format!(" [{t}]")).unwrap_or_default();
                println!("    {icon} {} `{}`{}", w.id, w.command, tag_str);
            }
        }

        if let Some(ref ctx) = evidence.latch_context {
            println!();
            println!("  Latch: {ctx}");
        }
    }
    Ok(())
}

pub fn print_sources(sources: &EvidenceSources, is_json: bool) -> Result<(), TrailError> {
    if is_json {
        let source_details = serde_json::json!([
            { "name": "witness", "available": sources.witness, "path": ".agent-witness/evidence/" },
            { "name": "latch", "available": sources.latch, "path": ".latch.db" },
            { "name": "probe", "available": sources.probe, "path": ".agent-probe/" },
            { "name": "atlas", "available": sources.atlas, "path": ".agent-atlas/" }
        ]);

        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "sources": sources,
            "source_details": source_details,
        }))?);
    } else {
        println!("trail sources:");
        println!();
        println!("  {} witness — .agent-witness/evidence/", if sources.witness { "✓" } else { "·" });
        println!("  {} latch   — .latch.db", if sources.latch { "✓" } else { "·" });
        println!("  {} probe   — .agent-probe/", if sources.probe { "✓" } else { "·" });
        println!("  {} atlas   — .agent-atlas/", if sources.atlas { "✓" } else { "·" });
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
