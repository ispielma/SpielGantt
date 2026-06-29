use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

mod agent;
mod event;
mod package;
mod project;
mod task;

#[derive(Debug, Parser)]
#[command(
    name = "spielgantt",
    version,
    about = "local-first scientific workflow planner"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        path: Option<PathBuf>,
    },
    Validate {
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Export {
        path: Option<PathBuf>,
    },
    Cache {
        #[command(subcommand)]
        command: package::CacheCommand,
    },
    Repair {
        path: Option<PathBuf>,
    },
    Normalize {
        path: Option<PathBuf>,
        #[arg(long)]
        apply: bool,
    },
    Agent {
        #[command(subcommand)]
        command: agent::AgentCommand,
    },
    Event {
        #[command(subcommand)]
        command: event::EventCommand,
    },
    Project {
        #[command(subcommand)]
        command: project::ProjectCommand,
    },
    Task {
        #[command(subcommand)]
        command: task::TaskCommand,
    },
}

pub fn run() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => package::init(target_or_current(path)),
        Commands::Validate { path, json } => package::validate(target_or_current(path), json),
        Commands::Export { path } => package::export(target_or_current(path)),
        Commands::Cache { command } => package::run_cache(command),
        Commands::Repair { path } => package::repair(target_or_current(path)),
        Commands::Normalize { path, apply } => package::normalize(target_or_current(path), apply),
        Commands::Agent { command } => agent::run(command),
        Commands::Event { command } => event::run(command),
        Commands::Project { command } => project::run(command),
        Commands::Task { command } => task::run(command),
    }
}

pub(crate) fn print_cli_error(message: &str) {
    eprintln!("{message}");
}

pub(crate) fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            print_cli_error(&format!("failed to serialize JSON output: {error}"));
            std::process::exit(1);
        }
    }
}

pub(crate) fn current_dir() -> PathBuf {
    std::env::current_dir().expect("current directory should exist")
}

pub(crate) fn target_or_current(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(current_dir)
}

pub(crate) fn print_open_command(command: &spielgantt_lib::platform_open::OpenCommand) {
    println!(
        "Command: {} {}",
        command.program(),
        command.args().join(" ")
    );
}
