use anyhow::{bail, Result};
use std::env;
use std::path::PathBuf;

/// Runtime context shared by all commands.
/// Discovers the git repo root and provides it to commands.
pub struct Ctx {
    pub repo_root: PathBuf,
}

impl Ctx {
    pub fn discover() -> Result<Self> {
        let repo_root = find_repo_root()?;
        Ok(Self { repo_root })
    }
}

/// Walk up from cwd until we find a directory containing .git
fn find_repo_root() -> Result<PathBuf> {
    let mut dir = env::current_dir()?;
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("not a git repository (no .git found in any parent directory)");
        }
    }
}
