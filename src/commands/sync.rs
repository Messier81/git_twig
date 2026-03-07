use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;

    let main_root = ctx.git_dir
        .parent()
        .context("git dir has no parent")?;

    // Check if a remote exists
    let has_remote = Command::new("git")
        .args(["remote"])
        .current_dir(main_root)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    if has_remote {
        println!("Fetching from remote...");
        let output = Command::new("git")
            .args(["fetch", "--prune"])
            .current_dir(main_root)
            .output()
            .context("failed to run git fetch")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git fetch failed: {}", stderr.trim());
        }

        println!("Pulling {}...", state.trunk);
        let output = Command::new("git")
            .args(["pull", "--ff-only"])
            .current_dir(main_root)
            .output()
            .context("failed to run git pull")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git pull on \"{}\" failed: {}", state.trunk, stderr.trim());
        }

        let pull_msg = String::from_utf8_lossy(&output.stdout);
        if pull_msg.contains("Already up to date") {
            println!("  {} is already up to date.", state.trunk);
        } else {
            println!("  ✓ {} updated.", state.trunk);
        }
    } else {
        println!("No remote configured — skipping pull.");
    }

    // Restack all branches on top
    if !state.branches.is_empty() {
        println!("Restacking...");
        super::restack::run(ctx)?;
    }

    Ok(())
}
