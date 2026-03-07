use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::{Branch, State};

pub fn create(ctx: &Ctx, name: &str) -> Result<()> {
    let mut state = State::load(&ctx.git_dir)?;

    if state.branches.contains_key(name) {
        bail!("branch \"{}\" is already tracked by gt", name);
    }

    if name == state.trunk {
        bail!("cannot stack on top of trunk as a new branch");
    }

    let parent = current_branch(&ctx)?;

    // Compute worktree path: always relative to the main repo, not the current worktree
    // git_dir is {main_repo}/.git, so its parent is the main repo root
    let main_root = ctx.git_dir
        .parent()
        .context("git dir has no parent")?;
    let repo_dir = main_root
        .file_name()
        .context("main repo has no directory name")?
        .to_string_lossy();
    let worktree_path = main_root
        .parent()
        .context("main repo has no parent directory")?
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
    state.save(&ctx.git_dir)?;

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

pub fn delete(ctx: &Ctx, name: &str) -> Result<()> {
    let mut state = State::load(&ctx.git_dir)?;

    if name == state.trunk {
        bail!("cannot delete trunk branch \"{}\"", name);
    }

    if !state.branches.contains_key(name) {
        bail!("branch \"{}\" is not tracked by gt", name);
    }

    let parent = state.branches[name].parent.clone();
    let worktree = state.branches[name].worktree.clone();

    // Re-parent children to the deleted branch's parent
    let children: Vec<String> = state
        .branches
        .iter()
        .filter(|(_, b)| b.parent == name)
        .map(|(n, _)| n.clone())
        .collect();

    for child in &children {
        state.branches.get_mut(child).unwrap().parent = parent.clone();
    }

    // Remove the worktree first (so the branch is no longer checked out)
    let output = Command::new("git")
        .args(["worktree", "remove", &worktree, "--force"])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to run git worktree remove")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("warning: git worktree remove: {}", stderr.trim());
    }

    // Delete the git branch
    let output = Command::new("git")
        .args(["branch", "-D", name])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to run git branch -D")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("warning: git branch -D: {}", stderr.trim());
    }

    // Remove from state
    state.branches.remove(name);
    state.save(&ctx.git_dir)?;

    println!("✓ Deleted branch \"{}\"", name);
    if !children.is_empty() {
        println!("  Re-parented {} branch{} to \"{}\"",
            children.len(),
            if children.len() == 1 { "" } else { "es" },
            parent,
        );
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
