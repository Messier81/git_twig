use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Runtime context shared by all commands.
/// Works from any worktree — always finds the shared .git/ directory.
pub struct Ctx {
    pub repo_root: PathBuf,
    pub git_dir: PathBuf,
}

impl Ctx {
    pub fn discover() -> Result<Self> {
        let repo_root = git_toplevel()?;
        let git_dir = git_common_dir(&repo_root)?;
        Ok(Self { repo_root, git_dir })
    }
}

/// Get the worktree root (or main repo root) via git
fn git_toplevel() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git")?;

    if !output.status.success() {
        bail!("not a git repository");
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

/// Get the shared .git/ directory (same across all worktrees)
fn git_common_dir(repo_root: &PathBuf) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(repo_root)
        .output()
        .context("failed to find git common dir")?;

    if !output.status.success() {
        bail!("could not determine git directory");
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(&path);

    // git may return a relative path, make it absolute
    if path.is_relative() {
        Ok(repo_root.join(path))
    } else {
        Ok(path)
    }
}
