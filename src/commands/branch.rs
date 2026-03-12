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

pub fn delete(ctx: &Ctx, name: &str, force: bool) -> Result<()> {
    let mut state = State::load(&ctx.git_dir)?;

    if name == state.trunk {
        bail!("cannot delete trunk branch \"{}\"", name);
    }

    if !state.branches.contains_key(name) {
        bail!("branch \"{}\" is not tracked by gt", name);
    }

    let parent = state.branches[name].parent.clone();
    let worktree = state.branches[name].worktree.clone();

    let children: Vec<String> = state
        .branches
        .iter()
        .filter(|(_, b)| b.parent == name)
        .map(|(n, _)| n.clone())
        .collect();

    if !force {
    eprint!("Delete \"{}\" and remove {}?", name, worktree);
    if !children.is_empty() {
        eprint!(" ({} child branch{} will be re-parented to \"{}\")",
            children.len(),
            if children.len() == 1 { "" } else { "es" },
            parent,
        );
    }
    eprint!(" [y/N] ");
    use std::io::{self, Read as _};
    let mut buf = [0u8; 1];
    io::stdin().read_exact(&mut buf).unwrap_or_default();
    if buf[0] != b'y' && buf[0] != b'Y' {
        println!("Cancelled.");
        return Ok(());
    }
    }

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

pub fn move_branch(ctx: &Ctx, name: &str, new_parent: &str) -> Result<()> {
    let mut state = State::load(&ctx.git_dir)?;

    if name == state.trunk {
        bail!("cannot move trunk branch");
    }

    if !state.branches.contains_key(name) {
        bail!("branch \"{}\" is not tracked by gt", name);
    }

    if new_parent != state.trunk && !state.branches.contains_key(new_parent) {
        bail!("parent \"{}\" is not tracked by gt", new_parent);
    }

    // Prevent cycles — new_parent can't be a descendant of name
    let mut check = new_parent.to_string();
    while let Some(b) = state.branches.get(&check) {
        if b.parent == name {
            bail!("\"{}\" is a descendant of \"{}\" — would create a cycle", new_parent, name);
        }
        if b.parent == state.trunk {
            break;
        }
        check = b.parent.clone();
    }

    let old_parent = state.branches[name].parent.clone();
    let worktree = state.branches[name].worktree.clone();

    if old_parent == new_parent {
        bail!("\"{}\" is already under \"{}\"", name, new_parent);
    }

    // Rebase --onto to carry only this branch's unique commits
    let output = Command::new("git")
        .args(["rebase", "--onto", new_parent, &old_parent, name])
        .current_dir(&worktree)
        .output()
        .context("failed to rebase")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Abort the failed rebase so the branch isn't left in a broken state
        let _ = Command::new("git")
            .args(["rebase", "--abort"])
            .current_dir(&worktree)
            .output();
        bail!("rebase failed — branch not moved:\n{}", stderr.trim());
    }

    state.branches.get_mut(name).unwrap().parent = new_parent.to_string();
    state.save(&ctx.git_dir)?;

    println!("✓ Moved \"{}\" from \"{}\" to \"{}\"", name, old_parent, new_parent);

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
