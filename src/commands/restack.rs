use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    let state = State::load(&ctx.git_dir)?;

    if state.branches.is_empty() {
        println!("No branches to restack.");
        return Ok(());
    }

    let order = topo_order(&state);

    let mut restacked = 0;
    for name in &order {
        let branch = &state.branches[name];
        let worktree = &branch.worktree;
        let parent = &branch.parent;

        let status = Command::new("git")
            .args(["rebase", parent])
            .current_dir(worktree)
            .status()
            .with_context(|| format!("failed to run git rebase for \"{}\"", name))?;

        if !status.success() {
            eprintln!();
            eprintln!("Rebase conflict in \"{}\"", name);
            eprintln!("  Resolve conflicts in: {}", worktree);
            eprintln!("  Then run:  cd {} && git rebase --continue", worktree);
            eprintln!("  Or abort:  cd {} && git rebase --abort", worktree);
            bail!("restack stopped at \"{}\" due to conflicts", name);
        }

        restacked += 1;
        println!("  ✓ {}  rebased onto {}", name, parent);
    }

    println!("Restacked {} branch{}.", restacked, if restacked == 1 { "" } else { "es" });
    Ok(())
}

/// Topological order: parents before children (depth-first from trunk)
fn topo_order(state: &State) -> Vec<String> {
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

    let mut order = Vec::new();
    collect(&children_map, &state.trunk, &mut order);
    order
}

fn collect(
    children_map: &BTreeMap<&str, Vec<&str>>,
    parent: &str,
    order: &mut Vec<String>,
) {
    if let Some(kids) = children_map.get(parent) {
        for kid in kids {
            order.push(kid.to_string());
            collect(children_map, kid, order);
        }
    }
}
