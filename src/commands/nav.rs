use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

/// Move to the next worktree in depth-first tree order
pub fn down(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;
    let current = current_branch(ctx)?;
    let order = tree_order(&state);

    let pos = order.iter().position(|n| *n == current)
        .context("current branch not found in tree")?;

    if pos + 1 < order.len() {
        print_worktree_path(ctx, &state, &order[pos + 1])
    } else {
        bail!("already at the last branch")
    }
}

/// Move to the previous worktree in depth-first tree order
pub fn up(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;
    let current = current_branch(ctx)?;
    let order = tree_order(&state);

    let pos = order.iter().position(|n| *n == current)
        .context("current branch not found in tree")?;

    if pos > 0 {
        print_worktree_path(ctx, &state, &order[pos - 1])
    } else {
        bail!("already at the first branch")
    }
}

/// Jump to a specific branch's worktree
pub fn switch(ctx: &Ctx, name: &str) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;

    if name == state.trunk {
        print!("{}", ctx.git_dir.parent().unwrap().display());
        return Ok(());
    }

    match state.branches.get(name) {
        Some(branch) => {
            print!("{}", branch.worktree);
            Ok(())
        }
        None => bail!("branch \"{}\" is not tracked by gt", name),
    }
}

/// Depth-first walk of the tree in the same order as `gt status` displays it
fn tree_order(state: &State) -> Vec<String> {
    let mut children_map: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, branch) in &state.branches {
        children_map
            .entry(branch.parent.as_str())
            .or_default()
            .push(name.as_str());
    }
    for kids in children_map.values_mut() {
        kids.sort();
    }

    let mut order = vec![state.trunk.clone()];
    collect_children(&children_map, &state.trunk, &mut order);
    order
}

fn collect_children(
    children_map: &BTreeMap<&str, Vec<&str>>,
    parent: &str,
    order: &mut Vec<String>,
) {
    if let Some(kids) = children_map.get(parent) {
        for kid in kids {
            order.push(kid.to_string());
            collect_children(children_map, kid, order);
        }
    }
}

fn print_worktree_path(ctx: &Ctx, state: &State, name: &str) -> Result<()> {
    if name == state.trunk {
        print!("{}", ctx.git_dir.parent().unwrap().display());
    } else if let Some(branch) = state.branches.get(name) {
        print!("{}", branch.worktree);
    } else {
        bail!("branch \"{}\" has no worktree", name);
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
