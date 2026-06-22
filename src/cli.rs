use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::TrailError;

#[derive(Parser, Debug)]
#[command(name = "trail", version, about = "Evidence-backed changelog generator")]
pub struct Cli {
    /// Project root override
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn resolve_repo(&self) -> Result<PathBuf, TrailError> {
        if let Some(ref repo) = self.repo {
            return Ok(repo.clone());
        }
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
        std::env::current_dir().map_err(TrailError::Io)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Generate a changelog from git history + evidence sources
    Log {
        /// How many commits back to include
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Only include commits since this ref/date
        #[arg(long)]
        since: Option<String>,
        /// Only include commits until this ref/date
        #[arg(long)]
        until: Option<String>,
    },
    /// Show a summary of what happened (agent-friendly brief)
    Summary {
        /// How many commits to summarize
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Show evidence trail for a specific commit
    Show {
        /// Commit SHA (short or full)
        commit: String,
    },
    /// List available evidence sources
    Sources,
}
