use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;
    let current = current_branch(ctx).unwrap_or_default();

    // Build children map: parent -> list of child branch names
    let mut children_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, branch) in &state.branches {
        children_map
            .entry(branch.parent.clone())
            .or_default()
            .push(name.clone());
    }

    let trunk_marker = if current == state.trunk { "●" } else { "○" };
    println!("{} {} (trunk)", trunk_marker, state.trunk);

    if state.branches.is_empty() {
        println!("  No branches. Create one with: gt branch create <name>");
        return Ok(());
    }

    // Print tree starting from trunk
    if let Some(kids) = children_map.get(&state.trunk) {
        print_children(&state, &children_map, kids, &current, "");
    }

    Ok(())
}

fn print_children(
    state: &State,
    children: &BTreeMap<String, Vec<String>>,
    kids: &[String],
    current: &str,
    prefix: &str,
) {
    for (i, name) in kids.iter().enumerate() {
        let is_last = i == kids.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let marker = if name == current { "●" } else { "○" };
        let worktree = &state.branches[name].worktree;

        println!("{}{}{}  {}    {}", prefix, connector, marker, name, worktree);

        if let Some(grandkids) = children.get(name) {
            let new_prefix = format!("{}{}", prefix, child_prefix);
            print_children(state, children, grandkids, current, &new_prefix);
        }
    }
}

pub fn list_branches(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;
    println!("{}", state.trunk);
    for name in state.branches.keys() {
        println!("{}", name);
    }
    Ok(())
}

fn current_branch(ctx: &Ctx) -> Result<String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&ctx.repo_root)
        .output()
        .context("failed to get current branch")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
