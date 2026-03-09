use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;
    let current = current_branch(ctx)?;

    let parent = if current == state.trunk {
        bail!("on trunk — no parent to diff against");
    } else if let Some(branch) = state.branches.get(&current) {
        &branch.parent
    } else {
        bail!("branch \"{}\" is not tracked by gt", current);
    };

    println!("Commits on \"{}\" (since \"{}\"):\n", current, parent);

    let range = format!("{}..{}", parent, current);
    let status = Command::new("git")
        .args(["log", "--oneline", "--no-decorate", &range])
        .current_dir(&ctx.repo_root)
        .status()
        .context("failed to run git log")?;

    if !status.success() {
        bail!("git log failed");
    }

    Ok(())
}

fn current_branch(ctx: &Ctx) -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to get current branch")?;

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        bail!("HEAD is detached — checkout a branch first");
    }
    Ok(branch)
}
