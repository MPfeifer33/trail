# PROJECT.md — trail

**What:** Evidence-backed changelog generator. Aggregates git history with witness recordings, latch context, and probe snapshots into work narratives.

**Status:** MVP complete. Log, summary, show, sources all working. Witness correlation by date.

**Tech:** Rust 2021, clap 4, serde/serde_json, chrono, regex, thiserror.

## Module Ownership

| Module | Owner | Status |
|--------|-------|--------|
| cli.rs | Nix | Done |
| main.rs | Nix | Done |
| git.rs | Nix | Done |
| evidence.rs | Nix | Done |
| report.rs | Nix | Done (Bjarn enhancing) |

## Usage

```sh
trail log                           # changelog with evidence annotations
trail log --limit 5                 # last 5 commits
trail log --since "2 days ago"      # time-filtered
trail summary                       # compact work summary
trail show <sha>                    # full commit detail with evidence
trail sources                       # list available evidence sources
```

## Evidence Sources

| Source | Data | Detection |
|--------|------|-----------|
| witness | Command recordings (stdout, exit code, hash) | .agent-witness/evidence/*.json |
| latch | Coordination context (decisions, claims, notes) | .latch.db |
| probe | Project snapshots | .agent-probe/ |
| atlas | Dependency graph | .agent-atlas/ |

## Last Updated

2026-06-22 — Initial skeleton with log/summary/show/sources working.
