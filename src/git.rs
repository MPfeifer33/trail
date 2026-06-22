use std::path::Path;
use std::process::Command;
use serde::Serialize;

use crate::TrailError;

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub body: String,
    pub files_changed: Vec<FileChange>,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Get commit log with optional since/until filters.
pub fn get_commits(
    repo: &Path,
    limit: usize,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<CommitInfo>, TrailError> {
    let mut args = vec![
        "log".to_string(),
        format!("-{}", limit),
        "--format=%H|%h|%an|%ai|%s|%b%x00".to_string(),
    ];

    if let Some(s) = since {
        args.push(format!("--since={}", s));
    }
    if let Some(u) = until {
        args.push(format!("--until={}", u));
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo)
        .output()?;

    if !output.status.success() {
        return Err(TrailError::Validation("Failed to read git log".into()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for entry in text.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let parts: Vec<&str> = entry.splitn(6, '|').collect();
        if parts.len() < 5 {
            continue;
        }

        let sha = parts[0].to_string();
        let short_sha = parts[1].to_string();
        let author = parts[2].to_string();
        let date = parts[3].to_string();
        let subject = parts[4].to_string();
        let body = parts.get(5).unwrap_or(&"").to_string();

        let files_changed = get_file_changes(repo, &sha)?;
        let insertions = files_changed.iter().map(|f| f.additions).sum();
        let deletions = files_changed.iter().map(|f| f.deletions).sum();

        commits.push(CommitInfo {
            sha,
            short_sha,
            author,
            date,
            subject,
            body,
            files_changed,
            insertions,
            deletions,
        });
    }

    Ok(commits)
}

fn get_file_changes(repo: &Path, sha: &str) -> Result<Vec<FileChange>, TrailError> {
    let output = Command::new("git")
        .args(["diff", "--numstat", &format!("{}~1..{}", sha, sha)])
        .current_dir(repo)
        .output()?;

    if !output.status.success() {
        // First commit won't have a parent
        let output = Command::new("git")
            .args(["diff", "--numstat", "--root", sha])
            .current_dir(repo)
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }
        return parse_numstat(&String::from_utf8_lossy(&output.stdout));
    }

    parse_numstat(&String::from_utf8_lossy(&output.stdout))
}

fn parse_numstat(text: &str) -> Result<Vec<FileChange>, TrailError> {
    let mut changes = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            changes.push(FileChange {
                additions: parts[0].parse().unwrap_or(0),
                deletions: parts[1].parse().unwrap_or(0),
                path: parts[2].to_string(),
            });
        }
    }
    Ok(changes)
}

/// Get a single commit by SHA.
pub fn get_commit(repo: &Path, sha: &str) -> Result<CommitInfo, TrailError> {
    let commits = get_commits_by_sha(repo, sha)?;
    commits.into_iter().next()
        .ok_or_else(|| TrailError::NotFound(format!("Commit {sha} not found")))
}

fn get_commits_by_sha(repo: &Path, sha: &str) -> Result<Vec<CommitInfo>, TrailError> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H|%h|%an|%ai|%s|%b%x00", sha])
        .current_dir(repo)
        .output()?;

    if !output.status.success() {
        return Err(TrailError::NotFound(format!("Commit {sha} not found")));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for entry in text.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let parts: Vec<&str> = entry.splitn(6, '|').collect();
        if parts.len() < 5 {
            continue;
        }

        let sha_full = parts[0].to_string();
        let files_changed = get_file_changes(repo, &sha_full)?;
        let insertions = files_changed.iter().map(|f| f.additions).sum();
        let deletions = files_changed.iter().map(|f| f.deletions).sum();

        commits.push(CommitInfo {
            sha: sha_full,
            short_sha: parts[1].to_string(),
            author: parts[2].to_string(),
            date: parts[3].to_string(),
            subject: parts[4].to_string(),
            body: parts.get(5).unwrap_or(&"").to_string(),
            files_changed,
            insertions,
            deletions,
        });
    }

    Ok(commits)
}
