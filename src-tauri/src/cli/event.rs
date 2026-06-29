use clap::Subcommand;
use serde::Serialize;
use std::process;

use super::{current_dir, print_cli_error, print_json};

#[derive(Debug, Subcommand)]
pub(crate) enum EventCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Create {
        #[arg(value_name = "EVENT_NAME")]
        id: String,
    },
    Rename {
        #[arg(value_name = "EVENT_NAME")]
        old_id: String,
        #[arg(value_name = "EVENT_NAME")]
        new_id: String,
    },
    Delete {
        #[arg(value_name = "EVENT_NAME")]
        id: String,
    },
}

#[derive(Debug, Serialize)]
struct EventListJson {
    schema_version: u32,
    events: Vec<String>,
}

pub(crate) fn run(command: EventCommand) {
    match command {
        EventCommand::List { json } => list(json),
        EventCommand::Create { id } => create(&id),
        EventCommand::Rename { old_id, new_id } => rename(&old_id, &new_id),
        EventCommand::Delete { id } => delete(&id),
    }
}

fn list(json: bool) {
    match spielgantt_lib::project_snapshot::load(&current_dir()) {
        Ok(snapshot) => {
            let events = snapshot.events();
            if json {
                print_json(&EventListJson {
                    schema_version: 1,
                    events: events.to_vec(),
                });
                return;
            }

            if events.is_empty() {
                println!("No project events defined.");
                return;
            }

            println!("Events:");
            for (index, event) in events.iter().enumerate() {
                println!("{}. {}", index + 1, event);
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn create(id: &str) {
    match spielgantt_lib::event::create(&current_dir(), id) {
        Ok(spielgantt_lib::event::CreateEventOutcome::Created { event_id }) => {
            println!("Created event '{event_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn rename(old_id: &str, new_id: &str) {
    match spielgantt_lib::event::rename(&current_dir(), old_id, new_id) {
        Ok(spielgantt_lib::event::RenameEventOutcome::Renamed { old_id, new_id }) => {
            println!("Renamed event '{old_id}' to '{new_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn delete(id: &str) {
    match spielgantt_lib::event::delete(&current_dir(), id) {
        Ok(spielgantt_lib::event::DeleteEventOutcome::Deleted { event_id }) => {
            println!("Deleted event '{event_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}
