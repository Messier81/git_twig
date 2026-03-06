use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::{Branch, State};

pub fn create(ctx: &Ctx, name: &str) -> Result<()> {
    let mut state = State::load(&ctx.repo_root)?;

    if state.branches.contains_key(name) {
        bail!("branch \"{}\" is already tracked by gt", name);
    }

    if name == state.trunk {
        bail!("cannot stack on top of trunk as a new branch");
    }

    let parent = current_branch(&ctx)?;

    // Compute worktree path: sibling directory named {repo}.{branch}
    let repo_dir = ctx.repo_root
        .file_name()
        .context("repo root has no directory name")?
        .to_string_lossy();
    let worktree_path = ctx.repo_root
        .parent()
        .context("repo root has no parent directory")?
        .join(format!("{}.{}", repo_dir, name));

    let worktree_str = worktree_path.to_string_lossy().to_string();

    // Create the git branch
    let output = Command::new("git")
        .args(["branch", name])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to run git branch")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git branch failed: {}", stderr.trim());
    }

    // Create the worktree
    let output = Command::new("git")
        .args(["worktree", "add", &worktree_str, name])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to run git worktree add")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree add failed: {}", stderr.trim());
    }

    // Track in state
    state.branches.insert(
        name.to_string(),
        Branch {
            parent: parent.clone(),
            worktree: worktree_str.clone(),
        },
    );
    state.save(&ctx.repo_root)?;

    println!(
        "{} Created branch \"{}\" (parent: {})",
        "✓".green().bold(),
        name.cyan(),
        parent.cyan()
    );
    println!(
        "  Worktree: {}",
        worktree_str.dimmed()
    );

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
