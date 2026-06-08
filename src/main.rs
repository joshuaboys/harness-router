//! harness-router (`hr`): add multiple OAuth/API accounts for AI coding CLIs and switch between
//! them by launching the tool with the right credentials. No proxy, no daemon — just isolate the
//! tool's config and `exec` it.

mod adapter;
mod cli;
mod commands;
mod config;
mod invoke;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Add(args) => commands::add(args),
        Command::Remove(args) => commands::remove(args),
        Command::List(args) => commands::list(args),
        Command::Login(args) => commands::login(args),
        Command::Tools => commands::tools(),
        Command::Run(tokens) => commands::run(tokens),
    };
    if let Err(err) = result {
        eprintln!("hr: {err:#}");
        std::process::exit(1);
    }
}
