use serde::Serialize;
use std::path::Path;
use std::process::Command;

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
        "--format=%H\x1f%h\x1f%an\x1f%ai\x1f%s\x1f%b%x00".to_string(),
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
        if is_empty_history_error(&String::from_utf8_lossy(&output.stderr)) {
            return Ok(Vec::new());
        }
        return Err(TrailError::Validation("Failed to read git log".into()));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();

    for entry in text.split('\0') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let parts: Vec<&str> = entry.splitn(6, '\x1f').collect();
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
        .args(["diff-tree", "--root", "--no-commit-id", "-r", "--numstat", sha])
        .current_dir(repo)
        .output()?;

    if output.status.success() {
        parse_numstat(&String::from_utf8_lossy(&output.stdout))
    } else {
        Ok(Vec::new())
    }
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

fn is_empty_history_error(stderr: &str) -> bool {
    stderr.contains("does not have any commits yet")
        || stderr.contains("your current branch")
        || stderr.contains("No commits yet")
}

/// Get a single commit by SHA.
pub fn get_commit(repo: &Path, sha: &str) -> Result<CommitInfo, TrailError> {
    let commits = get_commits_by_sha(repo, sha)?;
    commits.into_iter().next()
        .ok_or_else(|| TrailError::NotFound(format!("Commit {sha} not found")))
}

fn get_commits_by_sha(repo: &Path, sha: &str) -> Result<Vec<CommitInfo>, TrailError> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%H\x1f%h\x1f%an\x1f%ai\x1f%s\x1f%b%x00", sha])
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

        let parts: Vec<&str> = entry.splitn(6, '\x1f').collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_empty_history_errors() {
        assert!(is_empty_history_error(
            "fatal: your current branch 'master' does not have any commits yet"
        ));
        assert!(!is_empty_history_error("fatal: bad revision 'HEAD'"));
    }

    #[test]
    fn root_commit_stats_ignore_worktree_changes() {
        let workspace = tempfile::tempdir().unwrap();
        init_repo(workspace.path());

        write_file(workspace.path().join("README.md"), "hello\n");
        write_file(workspace.path().join("src/lib.rs"), "pub fn value() {}\n");
        git(workspace.path(), &["add", "."]);
        git(workspace.path(), &["commit", "-m", "initial"]);

        write_file(workspace.path().join("src/lib.rs"), "pub fn value() {}\n// dirty\n");

        let commits = get_commits(workspace.path(), 1, None, None).unwrap();

        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].files_changed.len(), 2);
        assert!(commits[0].files_changed.iter().any(|f| f.path == "README.md"));
        assert!(commits[0].files_changed.iter().any(|f| f.path == "src/lib.rs"));
    }

    fn init_repo(path: &std::path::Path) {
        git(path, &["init"]);
        git(path, &["config", "user.email", "trail@example.test"]);
        git(path, &["config", "user.name", "Trail Test"]);
    }

    fn git(path: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn empty_repo_returns_empty_commits() {
        let workspace = tempfile::tempdir().unwrap();
        init_repo(workspace.path());
        let commits = get_commits(workspace.path(), 10, None, None).unwrap();
        assert!(commits.is_empty());
    }

    #[test]
    fn get_commit_by_sha() {
        let workspace = tempfile::tempdir().unwrap();
        init_repo(workspace.path());
        write_file(workspace.path().join("test.txt"), "hello");
        git(workspace.path(), &["add", "."]);
        git(workspace.path(), &["commit", "-m", "first commit"]);

        let commits = get_commits(workspace.path(), 1, None, None).unwrap();
        let sha = &commits[0].sha;
        let commit = get_commit(workspace.path(), sha).unwrap();
        assert_eq!(commit.subject, "first commit");
    }

    #[test]
    fn parse_numstat_handles_empty() {
        let result = parse_numstat("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_numstat_handles_binary() {
        // Binary files show - instead of numbers
        let result = parse_numstat("-\t-\timage.png").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].additions, 0);
        assert_eq!(result[0].path, "image.png");
    }

    fn write_file(path: std::path::PathBuf, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }
}
