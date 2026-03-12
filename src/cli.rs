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

    /// Show branches, their stack relationships, and worktree status
    #[command(alias = "s")]
    Status,

    /// Show commits unique to the current branch (since parent)
    #[command(alias = "lg")]
    Log,

    /// Move to the previous worktree in the tree
    #[command(alias = "u")]
    Up,

    /// Move to the next worktree in the tree
    #[command(alias = "d")]
    Down,

    /// Rebase all branches onto their parents
    #[command(alias = "rs")]
    Restack,

    /// Pull latest trunk from remote and restack all branches
    #[command(alias = "sy")]
    Sync,

    /// Push branches and create/update stacked PRs
    #[command(alias = "su")]
    Submit {
        /// Specific branches to submit (default: all)
        names: Vec<String>,
    },

    /// Jump to a specific branch's worktree
    #[command(alias = "sw")]
    Switch {
        /// Branch name to switch to
        name: String,
    },

    /// Manage shell integration
    Shell {
        #[command(subcommand)]
        action: ShellAction,
    },

    /// List branch names (used by shell completions)
    #[command(hide = true)]
    #[command(name = "_branches")]
    Branches,
}

#[derive(Subcommand)]
pub enum BranchAction {
    /// Create a new branch stacked on the current branch, with its own worktree
    #[command(alias = "c")]
    Create {
        /// Name for the new branch
        name: String,
    },

    /// Delete a branch, its worktree, and re-parent any children
    #[command(alias = "d")]
    Delete {
        /// Branch name to delete
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Move a branch to a new parent
    #[command(alias = "m")]
    Move {
        /// Branch to move
        name: String,

        /// New parent branch
        new_parent: String,
    },
}

#[derive(Subcommand)]
pub enum ShellAction {
    /// Install shell integration (adds gt() wrapper to your shell rc file)
    Install,
    /// Remove shell integration
    Uninstall,
}
