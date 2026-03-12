mod cli;
mod commands;
mod ctx;
mod state;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, BranchAction, ShellAction};
use ctx::Ctx;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Shell commands don't need a git repo
    if let Commands::Shell { action } = cli.command {
        return match action {
            ShellAction::Install => commands::shell::install(),
            ShellAction::Uninstall => commands::shell::uninstall(),
        };
    }

    let ctx = Ctx::discover()?;

    match cli.command {
        Commands::Init => commands::init::run(&ctx),
        Commands::Branch { action } => match action {
            BranchAction::Create { name } => commands::branch::create(&ctx, &name),
            BranchAction::Delete { name, force } => commands::branch::delete(&ctx, &name, force),
        },
        Commands::Status => commands::status::run(&ctx),
        Commands::Log => commands::log::run(&ctx),
        Commands::Restack => commands::restack::run(&ctx),
        Commands::Sync => commands::sync::run(&ctx),
        Commands::Submit { names } => commands::submit::run(&ctx, &names),
        Commands::Up => commands::nav::up(&ctx),
        Commands::Down => commands::nav::down(&ctx),
        Commands::Switch { name } => commands::nav::switch(&ctx, &name),
        Commands::Branches => commands::status::list_branches(&ctx),
        Commands::Shell { .. } => unreachable!(),
    }
}
