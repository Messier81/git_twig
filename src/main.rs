mod cli;
mod commands;
mod ctx;
mod state;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, BranchAction};
use ctx::Ctx;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Ctx::discover()?;

    match cli.command {
        Commands::Init => commands::init::run(&ctx),
        Commands::Branch { action } => match action {
            BranchAction::Create { name } => commands::branch::create(&ctx, &name),
        },
    }
}
