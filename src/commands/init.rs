use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    if State::exists(&ctx.repo_root) {
        bail!(
            "already initialized ({})",
            State::file(&ctx.repo_root).display()
        );
    }

    let trunk = detect_trunk(&ctx.repo_root)?;
    let state = State::new(trunk.clone());
    state.save(&ctx.repo_root)?;

    println!(
        "{} Initialized git-twig (trunk: {})",
        "✓".green().bold(),
        trunk.cyan()
    );
    Ok(())
}

/// Detect the trunk branch by checking what HEAD points to,
/// then falling back to common names.
fn detect_trunk(repo_root: &Path) -> Result<String> {
    // Try: git symbolic-ref refs/remotes/origin/HEAD
    // This gives us what the remote considers the default branch
    if let Ok(output) = Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .current_dir(repo_root)
        .output()
    {
        if output.status.success() {
            let refname = String::from_utf8_lossy(&output.stdout);
            // refs/remotes/origin/main -> main
            if let Some(branch) = refname.trim().strip_prefix("refs/remotes/origin/") {
                return Ok(branch.to_string());
            }
        }
    }

    // Fallback: check if common branch names exist locally
    for candidate in ["main", "master"] {
        let output = Command::new("git")
            .args(["rev-parse", "--verify", candidate])
            .current_dir(repo_root)
            .output()?;
        if output.status.success() {
            return Ok(candidate.to_string());
        }
    }

    // Last resort: whatever branch we're currently on
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_root)
        .output()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    bail!("could not detect trunk branch — is this a git repository with at least one commit?");
}
