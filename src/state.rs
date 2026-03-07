use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub trunk: String,
    pub branches: BTreeMap<String, Branch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub parent: String,
    pub worktree: String,
}

impl State {
    pub fn new(trunk: String) -> Self {
        Self {
            trunk,
            branches: BTreeMap::new(),
        }
    }

    /// Returns the path to {git_dir}/gt/
    pub fn dir(git_dir: &Path) -> PathBuf {
        git_dir.join("gt")
    }

    /// Returns the path to {git_dir}/gt/state.json
    pub fn file(git_dir: &Path) -> PathBuf {
        Self::dir(git_dir).join("state.json")
    }

    pub fn exists(git_dir: &Path) -> bool {
        Self::file(git_dir).exists()
    }

    pub fn load(git_dir: &Path) -> Result<Self> {
        let path = Self::file(git_dir);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(state)
    }

    pub fn save(&self, git_dir: &Path) -> Result<()> {
        let dir = Self::dir(git_dir);
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let path = Self::file(git_dir);
        let json = serde_json::to_string_pretty(self)
            .context("failed to serialize state")?;
        fs::write(&path, json)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
