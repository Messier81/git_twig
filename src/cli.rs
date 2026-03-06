use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gt",
    about = "git-twig: stacked branches + parallel worktrees for the AI agent age",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize git-twig in the current repository
    #[command(alias = "i")]
    Init,

    /// Manage branches in a stack
    #[command(alias = "b")]
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },
}

#[derive(Subcommand)]
pub enum BranchAction {
    /// Create a new branch stacked on the current branch, with its own worktree
    #[command(alias = "c")]
    Create {
        /// Name for the new branch
        name: String,
    },
}
