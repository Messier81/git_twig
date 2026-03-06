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

    /// Returns the path to .git/gt/ for a given repo root
    pub fn dir(repo_root: &Path) -> PathBuf {
        repo_root.join(".git").join("gt")
    }

    /// Returns the path to .git/gt/state.json
    pub fn file(repo_root: &Path) -> PathBuf {
        Self::dir(repo_root).join("state.json")
    }

    pub fn exists(repo_root: &Path) -> bool {
        Self::file(repo_root).exists()
    }

    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = Self::file(repo_root);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state: Self = serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(state)
    }

    pub fn save(&self, repo_root: &Path) -> Result<()> {
        let dir = Self::dir(repo_root);
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;

        let path = Self::file(repo_root);
        let json = serde_json::to_string_pretty(self)
            .context("failed to serialize state")?;
        fs::write(&path, json)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }
}
