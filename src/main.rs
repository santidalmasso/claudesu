mod account_state;
mod cli;
mod commands;
mod config;
mod errors;
mod fs_util;
mod live;
mod lock;
mod models;
mod paths;
mod printer;
mod sequence;
mod storage;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let output = dispatch(cli.command)?;
    println!("{output}");
    Ok(())
}

fn dispatch(cmd: Commands) -> Result<String> {
    let out = match cmd {
        Commands::Add { slot } => commands::add::run(slot)?,
        Commands::Remove { who, yes } => commands::remove::run(&who, yes)?,
        Commands::List => commands::list::run()?,
        Commands::Switch => commands::switch::run_next()?,
        Commands::SwitchTo { who } => commands::switch::run_to(&who)?,
        Commands::Status => commands::status::run()?,
        Commands::Purge { yes } => commands::purge::run(yes)?,
    };
    Ok(out)
}
