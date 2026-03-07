use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

pub fn run(ctx: &Ctx) -> Result<()> {
    if !has_gh() {
        bail!("GitHub CLI (gh) is not installed. Install it: https://cli.github.com");
    }

    let state = State::load(&ctx.git_dir)?;

    if state.branches.is_empty() {
        println!("No branches to submit.");
        return Ok(());
    }

    let main_root = ctx.git_dir
        .parent()
        .context("git dir has no parent")?;

    let order = topo_order(&state);

    let mut submitted = 0;
    for name in &order {
        let branch = &state.branches[name];
        let parent = &branch.parent;

        // Push the branch
        let output = Command::new("git")
            .args(["push", "-u", "origin", name, "--force-with-lease"])
            .current_dir(main_root)
            .output()
            .with_context(|| format!("failed to push \"{}\"", name))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git push failed for \"{}\": {}", name, stderr.trim());
        }

        // Check if a PR already exists
        let existing = Command::new("gh")
            .args(["pr", "view", name, "--json", "number", "--jq", ".number"])
            .current_dir(main_root)
            .output()
            .context("failed to check existing PR")?;

        if existing.status.success() && !existing.stdout.is_empty() {
            let pr_num = String::from_utf8_lossy(&existing.stdout).trim().to_string();

            // Update the PR base branch
            Command::new("gh")
                .args(["pr", "edit", &pr_num, "--base", parent])
                .current_dir(main_root)
                .output()
                .context("failed to update PR base")?;

            println!("  ✓ {}  pushed (PR #{} already exists)", name, pr_num);
        } else {
            // Create a new PR
            let title = name.replace('-', " ");
            let body = format!("Stacked on `{}`.\n\nCreated by `gt submit`.", parent);

            let output = Command::new("gh")
                .args([
                    "pr", "create",
                    "--head", name,
                    "--base", parent,
                    "--title", &title,
                    "--body", &body,
                ])
                .current_dir(main_root)
                .output()
                .context("failed to create PR")?;

            if output.status.success() {
                let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("  ✓ {}  pushed + PR created: {}", name, url);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("  ✗ {}  push OK but PR creation failed: {}", name, stderr.trim());
            }
        }

        submitted += 1;
    }

    println!("Submitted {} branch{}.", submitted, if submitted == 1 { "" } else { "es" });
    Ok(())
}

fn has_gh() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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
