mod cli;
mod evidence;
mod git;
mod report;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), TrailError> {
    let repo = cli.resolve_repo()?;

    match &cli.command {
        Command::Log { limit, since, until } => {
            let commits = git::get_commits(
                &repo,
                *limit,
                since.as_deref(),
                until.as_deref(),
            )?;
            let ev: Vec<_> = commits.iter()
                .map(|c| gather_evidence(&repo, c))
                .collect();
            report::print_log(&commits, &ev, cli.is_json())
        }
        Command::Summary { limit } => {
            let commits = git::get_commits(&repo, *limit, None, None)?;
            let ev: Vec<_> = commits.iter()
                .map(|c| gather_evidence(&repo, c))
                .collect();
            report::print_summary(&commits, &ev, cli.is_json())
        }
        Command::Show { commit } => {
            let commit_info = git::get_commit(&repo, commit)?;
            let ev = gather_evidence(&repo, &commit_info);
            report::print_commit_detail(&commit_info, &ev, cli.is_json())
        }
        Command::Sources => {
            let sources = evidence::detect_sources(&repo);
            report::print_sources(&sources, cli.is_json())
        }
    }
}

fn gather_evidence(repo: &std::path::Path, commit: &git::CommitInfo) -> evidence::CommitEvidence {
    evidence::CommitEvidence {
        witness_runs: evidence::find_witness_evidence(repo, &commit.date),
        latch_context: evidence::find_latch_context(repo),
        probe_snapshot: repo.join(".agent-probe").exists(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TrailError {
    #[error("{0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl TrailError {
    pub fn exit_code(&self) -> i32 {
        match self {
            TrailError::Validation(_) => 1,
            TrailError::NotFound(_) => 3,
            TrailError::Io(_) => 2,
            TrailError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            TrailError::Validation(_) => "validation_error",
            TrailError::NotFound(_) => "not_found",
            TrailError::Io(_) => "io_error",
            TrailError::Json(_) => "json_error",
        }
    }
}
