use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::ctx::Ctx;
use crate::state::State;

const STACK_MARKER_START: &str = "<!-- gt:stack -->";
const STACK_MARKER_END: &str = "<!-- /gt:stack -->";

pub fn run(ctx: &Ctx, names: &[String]) -> Result<()> {
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

    let all = topo_order(&state);
    let order: Vec<String> = if names.is_empty() {
        all
    } else {
        for n in names {
            if !state.branches.contains_key(n) {
                bail!("branch \"{}\" is not tracked by gt", n);
            }
        }
        all.into_iter().filter(|n| names.contains(n)).collect()
    };

    if order.is_empty() {
        println!("No matching branches to submit.");
        return Ok(());
    }

    // Phase 1: push branches and create/find PRs, collect PR numbers
    let mut pr_numbers: BTreeMap<String, String> = BTreeMap::new();

    for name in &order {
        let branch = &state.branches[name];
        let parent = &branch.parent;

        let output = Command::new("git")
            .args(["push", "-u", "origin", name, "--force-with-lease"])
            .current_dir(main_root)
            .output()
            .with_context(|| format!("failed to push \"{}\"", name))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git push failed for \"{}\": {}", name, stderr.trim());
        }

        let existing = Command::new("gh")
            .args(["pr", "view", name, "--json", "number", "--jq", ".number"])
            .current_dir(main_root)
            .output()
            .context("failed to check existing PR")?;

        if existing.status.success() && !existing.stdout.is_empty() {
            let pr_num = String::from_utf8_lossy(&existing.stdout).trim().to_string();

            Command::new("gh")
                .args(["pr", "edit", &pr_num, "--base", parent])
                .current_dir(main_root)
                .output()
                .context("failed to update PR base")?;

            pr_numbers.insert(name.clone(), pr_num.clone());
            println!("  ✓ {}  pushed (PR #{})", name, pr_num);
        } else {
            let title = name.replace('-', " ");
            let body = format!("Stacked on `{}`.", parent);

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
                // Extract PR number from URL (e.g., .../pull/5)
                let pr_num = url.rsplit('/').next().unwrap_or("?").to_string();
                pr_numbers.insert(name.clone(), pr_num.clone());
                println!("  ✓ {}  PR #{} created", name, pr_num);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("  ✗ {}  push OK, PR failed: {}", name, stderr.trim());
            }
        }
    }

    // Phase 2: update each PR body with the stack tree
    if !pr_numbers.is_empty() {
        println!("Updating stack comments...");
        let repo_url = get_repo_url(main_root);
        let children_map = build_children_map(&state);

        for (name, pr_num) in &pr_numbers {
            let comment_body = render_stack_tree(
                &state, &children_map, &pr_numbers, name, &repo_url,
            );

            // Find existing gt comment to update
            let existing_comment_id = find_stack_comment(main_root, pr_num);

            if let Some(comment_id) = existing_comment_id {
                Command::new("gh")
                    .args(["api", "--method", "PATCH",
                        &format!("repos/{{owner}}/{{repo}}/issues/comments/{}", comment_id),
                        "-f", &format!("body={}", comment_body)])
                    .current_dir(main_root)
                    .output()
                    .ok();
            } else {
                Command::new("gh")
                    .args(["pr", "comment", pr_num, "--body", &comment_body])
                    .current_dir(main_root)
                    .output()
                    .ok();
            }
        }
    }

    println!("Submitted {} branch{}.", pr_numbers.len(), if pr_numbers.len() == 1 { "" } else { "es" });
    Ok(())
}

fn render_stack_tree(
    state: &State,
    _children_map: &BTreeMap<String, Vec<String>>,
    pr_numbers: &BTreeMap<String, String>,
    current: &str,
    repo_url: &str,
) -> String {
    let path = ancestor_path(state, current);

    let mut lines = Vec::new();
    lines.push(STACK_MARKER_START.to_string());
    lines.push("This change is part of the following stack:".to_string());
    lines.push(String::new());

    for (depth, name) in path.iter().enumerate() {
        let is_current = *name == current;
        lines.push(render_line(name, is_current, depth, pr_numbers, repo_url));
    }

    lines.push(String::new());
    lines.push("<sub>Change managed by [**git-twig**](https://github.com/Messier81/git_twig).</sub>".to_string());
    lines.push(STACK_MARKER_END.to_string());
    lines.join("\n")
}

fn render_line(
    name: &str,
    is_current: bool,
    depth: usize,
    pr_numbers: &BTreeMap<String, String>,
    repo_url: &str,
) -> String {
    let indent = "  ".repeat(depth);

    let pr_link = if let Some(pr_num) = pr_numbers.get(name) {
        if repo_url.is_empty() {
            format!("**#{}**", pr_num)
        } else {
            format!("[**#{}**]({}/pull/{})", pr_num, repo_url, pr_num)
        }
    } else {
        String::new()
    };

    if is_current {
        if pr_link.is_empty() {
            format!("{}- **{}** ◀", indent, name)
        } else {
            format!("{}- {} — **{}** ◀", indent, pr_link, name)
        }
    } else if pr_link.is_empty() {
        format!("{}- {}", indent, name)
    } else {
        format!("{}- {} — {}", indent, pr_link, name)
    }
}

/// Walk from a branch up to trunk, return the path top-down
fn ancestor_path(state: &State, name: &str) -> Vec<String> {
    let mut path = vec![name.to_string()];
    let mut current = name.to_string();

    while let Some(branch) = state.branches.get(&current) {
        if branch.parent == state.trunk {
            break;
        }
        path.push(branch.parent.clone());
        current = branch.parent.clone();
    }

    path.reverse();
    path
}

fn find_stack_comment(main_root: &Path, pr_num: &str) -> Option<String> {
    let output = Command::new("gh")
        .args(["api", &format!("repos/{{owner}}/{{repo}}/issues/{}/comments", pr_num),
            "--jq", &format!(".[] | select(.body | contains(\"{}\")) | .id", STACK_MARKER_START)])
        .current_dir(main_root)
        .output()
        .ok()?;

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}

fn build_children_map(state: &State) -> BTreeMap<String, Vec<String>> {
    let mut children_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, branch) in &state.branches {
        children_map
            .entry(branch.parent.clone())
            .or_default()
            .push(name.clone());
    }
    for kids in children_map.values_mut() {
        kids.sort();
    }
    children_map
}

fn get_repo_url(main_root: &Path) -> String {
    Command::new("gh")
        .args(["repo", "view", "--json", "url", "--jq", ".url"])
        .current_dir(main_root)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
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
