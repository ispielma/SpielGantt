use clap::Subcommand;
use serde::Serialize;
use spielgantt_lib::{metadata::TaskStatus, task::TaskUpdate};
use std::{path::PathBuf, process};

use super::{current_dir, print_cli_error, print_json, print_open_command};

#[derive(Debug, Subcommand)]
pub(crate) enum TaskCommand {
    Adopt {
        #[arg(value_name = "TASK_FOLDER")]
        folder: PathBuf,
        #[arg(long, value_name = "TASK_NAME")]
        id: String,
    },
    AdoptableFolders {
        #[arg(long)]
        json: bool,
    },
    Create {
        #[arg(value_name = "TASK_NAME")]
        id: String,
    },
    InsertBefore {
        #[arg(value_name = "SELECTED_TASK_NAME")]
        selected_task_id: String,
        #[arg(value_name = "TASK_NAME")]
        id: String,
        #[arg(long)]
        json: bool,
    },
    InsertAfter {
        #[arg(value_name = "SELECTED_TASK_NAME")]
        selected_task_id: String,
        #[arg(value_name = "TASK_NAME")]
        id: String,
        #[arg(long)]
        json: bool,
    },
    Readme {
        #[arg(value_name = "TASK_NAME")]
        id: String,
        #[arg(long)]
        open: bool,
        #[arg(long, requires = "open")]
        dry_run: bool,
    },
    List {
        #[arg(long)]
        json: bool,
    },
    Rename {
        #[arg(value_name = "TASK_NAME")]
        old_id: String,
        #[arg(value_name = "TASK_NAME")]
        new_id: String,
    },
    Delete {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(long, conflicts_with = "delete_directory")]
        remove_from_chart: bool,
        #[arg(long)]
        delete_directory: bool,
        #[arg(long)]
        json: bool,
    },
    EndsAt {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(
            value_name = "EVENT_NAME",
            required_unless_present = "clear",
            conflicts_with = "clear"
        )]
        event_id: Option<String>,
        #[arg(long)]
        clear: bool,
    },
    Depend {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(value_name = "TASK_NAME")]
        blocker_id: String,
    },
    Dependency {
        #[command(subcommand)]
        command: TaskDependencyCommand,
    },
    Relationships {
        #[arg(long)]
        json: bool,
    },
    Workflow {
        #[arg(long)]
        json: bool,
    },
    Open {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(long)]
        dry_run: bool,
    },
    Update {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(long)]
        status: Option<String>,
    },
    Show {
        #[arg(long)]
        json: bool,
        task_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TaskDependencyCommand {
    Remove {
        #[arg(value_name = "TASK_NAME")]
        task_id: String,
        #[arg(value_name = "BLOCKER_NAME")]
        blocker_id: String,
    },
}

#[derive(Debug, Serialize)]
struct TaskAdoptableFoldersJson {
    schema_version: u32,
    folders: Vec<spielgantt_lib::task::AdoptableTaskFolder>,
}

#[derive(Debug, Serialize)]
struct TaskRelativeInsertJson {
    schema_version: u32,
    mode: spielgantt_lib::task::RelativeInsertMode,
    selected_task_id: String,
    inserted_task_id: String,
    task_path: String,
}

#[derive(Debug, Serialize)]
struct TaskListJson {
    schema_version: u32,
    tasks: Vec<TaskListEntryJson>,
}

#[derive(Debug, Serialize)]
struct TaskListEntryJson {
    id: String,
    status: Option<TaskStatus>,
}

#[derive(Debug, Serialize)]
struct TaskDeleteJson {
    schema_version: u32,
    task_id: String,
    mode: &'static str,
    path: String,
    committed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    cleanup: Option<TaskDeleteCleanupJson>,
}

#[derive(Debug, Serialize)]
struct TaskDeleteCleanupJson {
    status: &'static str,
    path: String,
    error: String,
}

#[derive(Debug, Serialize)]
struct TaskDetailsJson {
    schema_version: u32,
    id: String,
    path: String,
    dependencies: Vec<String>,
    status: Option<TaskStatus>,
}

pub(crate) fn run(command: TaskCommand) {
    match command {
        TaskCommand::Adopt { folder, id } => adopt(folder, &id),
        TaskCommand::AdoptableFolders { json } => adoptable_folders(json),
        TaskCommand::Create { id } => create(&id),
        TaskCommand::InsertBefore {
            selected_task_id,
            id,
            json,
        } => insert_relative(
            spielgantt_lib::task::RelativeInsertMode::Before,
            &selected_task_id,
            &id,
            json,
        ),
        TaskCommand::InsertAfter {
            selected_task_id,
            id,
            json,
        } => insert_relative(
            spielgantt_lib::task::RelativeInsertMode::After,
            &selected_task_id,
            &id,
            json,
        ),
        TaskCommand::Readme { id, open, dry_run } => readme(&id, open, dry_run),
        TaskCommand::List { json } => list(json),
        TaskCommand::Rename { old_id, new_id } => rename(&old_id, &new_id),
        TaskCommand::Delete {
            task_id,
            remove_from_chart: _,
            delete_directory,
            json,
        } => delete(&task_id, delete_directory, json),
        TaskCommand::EndsAt {
            task_id,
            event_id,
            clear,
        } => ends_at(&task_id, event_id.as_deref(), clear),
        TaskCommand::Depend {
            task_id,
            blocker_id,
        } => depend(&task_id, &blocker_id),
        TaskCommand::Dependency { command } => dependency(command),
        TaskCommand::Relationships { json } => relationships(json),
        TaskCommand::Workflow { json } => workflow(json),
        TaskCommand::Open { task_id, dry_run } => open(&task_id, dry_run),
        TaskCommand::Update { task_id, status } => update(&task_id, status),
        TaskCommand::Show { json, task_id } => show(&task_id, json),
    }
}

fn create(id: &str) {
    match spielgantt_lib::task::create(&current_dir(), id) {
        Ok(spielgantt_lib::task::CreateTaskOutcome::Created { task_path }) => {
            println!("Created task '{id}' in {}", task_path.display());
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn adoptable_folders(json: bool) {
    match spielgantt_lib::task::list_adoptable_task_folders(&current_dir()) {
        Ok(folders) => {
            if json {
                print_json(&TaskAdoptableFoldersJson {
                    schema_version: 1,
                    folders,
                });
                return;
            }
            for folder in folders {
                println!("{} -> {}", folder.project_relative_path(), folder.task_id());
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn insert_relative(
    mode: spielgantt_lib::task::RelativeInsertMode,
    selected_task_id: &str,
    inserted_task_id: &str,
    json: bool,
) {
    let result = match mode {
        spielgantt_lib::task::RelativeInsertMode::Before => {
            spielgantt_lib::task::insert_before(&current_dir(), selected_task_id, inserted_task_id)
        }
        spielgantt_lib::task::RelativeInsertMode::After => {
            spielgantt_lib::task::insert_after(&current_dir(), selected_task_id, inserted_task_id)
        }
    };

    match result {
        Ok(spielgantt_lib::task::InsertRelativeTaskOutcome::Inserted {
            mode,
            selected_task_id,
            inserted_task_id,
            task_path,
        }) => {
            if json {
                print_json(&TaskRelativeInsertJson {
                    schema_version: 1,
                    mode,
                    selected_task_id,
                    inserted_task_id,
                    task_path: task_path.display().to_string(),
                });
                return;
            }

            let mode_label = match mode {
                spielgantt_lib::task::RelativeInsertMode::Before => "before",
                spielgantt_lib::task::RelativeInsertMode::After => "after",
            };
            println!(
                "Inserted task '{inserted_task_id}' {mode_label} '{selected_task_id}' in {}",
                task_path.display()
            );
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn readme(task_id: &str, open: bool, dry_run: bool) {
    match spielgantt_lib::task::readme(&current_dir(), task_id) {
        Ok(outcome) => {
            let (path, created) = match outcome {
                spielgantt_lib::task::ReadmeTaskOutcome::Created { path } => (path, true),
                spielgantt_lib::task::ReadmeTaskOutcome::Existing { path } => (path, false),
            };
            if dry_run {
                match spielgantt_lib::platform_open::command_for_current_platform(&path) {
                    Ok(command) => {
                        println!("Would open README for task '{task_id}': {}", path.display());
                        print_open_command(&command);
                    }
                    Err(error) => {
                        print_cli_error(&error.to_string());
                        process::exit(1);
                    }
                }
            } else if open {
                match spielgantt_lib::platform_open::open_path(&path) {
                    Ok(command) => {
                        println!("Opened README for task '{task_id}': {}", path.display());
                        print_open_command(&command);
                    }
                    Err(error) => {
                        print_cli_error(&error.to_string());
                        process::exit(1);
                    }
                }
            } else if created {
                println!("Created README for task '{task_id}': {}", path.display());
            } else {
                println!("README for task '{task_id}': {}", path.display());
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn list(json: bool) {
    match spielgantt_lib::task::list(&current_dir()) {
        Ok(tasks) => {
            if json {
                print_json(&TaskListJson {
                    schema_version: 1,
                    tasks: tasks
                        .iter()
                        .map(|task| TaskListEntryJson {
                            id: task.id().to_string(),
                            status: task.status().cloned(),
                        })
                        .collect(),
                });
                return;
            }

            println!("ID\tStatus");
            for task in tasks {
                println!(
                    "{}\t{}",
                    task.id(),
                    task.status().map(format_status).unwrap_or("-"),
                );
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn rename(old_id: &str, new_id: &str) {
    match spielgantt_lib::task::rename(&current_dir(), old_id, new_id) {
        Ok(spielgantt_lib::task::RenameTaskOutcome::Renamed {
            old_id,
            new_id,
            old_path,
            new_path,
        }) => {
            println!(
                "Renamed task '{old_id}' to '{new_id}': {} -> {}",
                old_path.display(),
                new_path.display()
            );
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn delete(task_id: &str, delete_directory: bool, json: bool) {
    let mode = if delete_directory {
        spielgantt_lib::task::DeleteTaskMode::DeleteDirectory
    } else {
        spielgantt_lib::task::DeleteTaskMode::RemoveFromChart
    };

    match spielgantt_lib::task::delete(&current_dir(), task_id, mode) {
        Ok(outcome) => {
            let (task_id, mode_label, path, cleanup_failure) = match outcome {
                spielgantt_lib::task::DeleteTaskOutcome::RemovedFromChart {
                    task_id,
                    task_path,
                    cleanup_failure,
                } => (task_id, "remove-from-chart", task_path, cleanup_failure),
                spielgantt_lib::task::DeleteTaskOutcome::DeletedDirectory {
                    task_id,
                    task_path,
                    cleanup_failure,
                } => (task_id, "delete-directory", task_path, cleanup_failure),
            };
            let cleanup = cleanup_failure
                .as_ref()
                .map(|failure| TaskDeleteCleanupJson {
                    status: "failed",
                    path: failure.path().display().to_string(),
                    error: failure.error().to_string(),
                });
            if json {
                print_json(&TaskDeleteJson {
                    schema_version: 1,
                    task_id,
                    mode: mode_label,
                    path: path.display().to_string(),
                    committed: true,
                    cleanup,
                });
                if let Some(failure) = cleanup_failure {
                    print_cli_error(&format!(
                        "{}. Package mutation committed; remove '{}' manually after reviewing it.",
                        failure.error(),
                        failure.path().display()
                    ));
                }
                return;
            }
            match mode {
                spielgantt_lib::task::DeleteTaskMode::RemoveFromChart => {
                    println!("Removed task '{task_id}' from chart: {}", path.display());
                }
                spielgantt_lib::task::DeleteTaskMode::DeleteDirectory => {
                    println!("Deleted task directory for '{task_id}': {}", path.display());
                }
            }
            if let Some(failure) = cleanup_failure {
                print_cli_error(&format!(
                    "{}. Package mutation committed; remove '{}' manually after reviewing it.",
                    failure.error(),
                    failure.path().display()
                ));
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn depend(task_id: &str, blocker_id: &str) {
    match spielgantt_lib::task::add_dependency(&current_dir(), task_id, blocker_id) {
        Ok(spielgantt_lib::task::AddDependencyOutcome::Added {
            task_id,
            blocker_id,
        }) => {
            println!("Added blocker '{blocker_id}' to task '{task_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn dependency(command: TaskDependencyCommand) {
    match command {
        TaskDependencyCommand::Remove {
            task_id,
            blocker_id,
        } => dependency_remove(&task_id, &blocker_id),
    }
}

fn dependency_remove(task_id: &str, blocker_id: &str) {
    match spielgantt_lib::task::remove_dependency(&current_dir(), task_id, blocker_id) {
        Ok(spielgantt_lib::task::RemoveDependencyOutcome::Removed {
            task_id,
            blocker_id,
        }) => {
            println!("Removed blocker '{blocker_id}' from task '{task_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn relationships(json: bool) {
    match spielgantt_lib::dependency_relationships::load(&current_dir()) {
        Ok(relationships) => {
            if json {
                print_json(&relationships);
                return;
            }

            println!("Dependency relationships:");
            for task in relationships.tasks() {
                println!("- {}", task.id());
                if task.blockers().is_empty() {
                    println!("  Blockers: none");
                } else {
                    println!(
                        "  Blockers: {}",
                        task.blockers()
                            .iter()
                            .map(|blocker| blocker.id())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if task.blocks().is_empty() {
                    println!("  Blocks: none");
                } else {
                    println!(
                        "  Blocks: {}",
                        task.blocks()
                            .iter()
                            .map(|blocked_task| blocked_task.id())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn workflow(json: bool) {
    match spielgantt_lib::event_axis_workflow::load(&current_dir()) {
        Ok(workflow) => {
            if json {
                print_json(&workflow);
                return;
            }

            println!("Event-axis workflow:");
            println!("Schema version: {}", workflow.schema_version());
            if workflow.events().is_empty() {
                println!("Events: none");
            } else {
                println!("Events: {}", workflow.events().join(", "));
            }
            println!(
                "Validation: {}",
                if workflow.validation().diagnostics().is_empty() {
                    "valid"
                } else {
                    "invalid"
                }
            );
            for task in workflow.tasks() {
                println!("- {}", task.id());
                for diagnostic in task.validation_diagnostics() {
                    println!("  Diagnostic: {diagnostic}");
                }
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn ends_at(task_id: &str, event_id: Option<&str>, clear: bool) {
    match spielgantt_lib::task::set_ends_at(&current_dir(), task_id, event_id, clear) {
        Ok(spielgantt_lib::task::SetEndsAtOutcome::Set { task_id, event_id }) => {
            println!("Set task '{task_id}' to end at event '{event_id}'");
        }
        Ok(spielgantt_lib::task::SetEndsAtOutcome::Cleared { task_id }) => {
            println!("Cleared ends_at for task '{task_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn open(task_id: &str, dry_run: bool) {
    match spielgantt_lib::task::resolve_task_path(&current_dir(), task_id) {
        Ok(path) if dry_run => {
            match spielgantt_lib::platform_open::command_for_current_platform(&path) {
                Ok(command) => {
                    println!("Would open task '{task_id}': {}", path.display());
                    print_open_command(&command);
                }
                Err(error) => {
                    print_cli_error(&error.to_string());
                    process::exit(1);
                }
            }
        }
        Ok(path) => match spielgantt_lib::platform_open::open_path(&path) {
            Ok(command) => {
                println!("Opened task '{task_id}': {}", path.display());
                print_open_command(&command);
            }
            Err(error) => {
                print_cli_error(&error.to_string());
                process::exit(1);
            }
        },
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn update(task_id: &str, status: Option<String>) {
    if status.is_none() {
        print_cli_error(
            "usage: spielgantt task update <task-id> --status <blocked|unblocked|done>",
        );
        process::exit(2);
    }

    let mut update = TaskUpdate::default();
    if let Some(value) = status {
        update.status = Some(parse_status(&value).unwrap_or_else(|error| {
            print_cli_error(&error);
            process::exit(2);
        }));
    }
    match spielgantt_lib::task::update(&current_dir(), task_id, update) {
        Ok(spielgantt_lib::task::UpdateTaskOutcome::Updated { task_id }) => {
            println!("Updated task '{task_id}'");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn show(task_id: &str, json: bool) {
    match spielgantt_lib::task::show(&current_dir(), task_id) {
        Ok(task) => {
            if json {
                print_json(&TaskDetailsJson {
                    schema_version: 1,
                    id: task.id().to_string(),
                    path: task.path().display().to_string(),
                    dependencies: task.dependencies().to_vec(),
                    status: task.status().cloned(),
                });
                return;
            }

            println!("Task: {}", task.id());
            println!("Path: {}", task.path().display());
            if let Some(status) = task.status() {
                println!("Status: {}", format_status(status));
            }
            println!("Blockers:");
            for blocker in task.dependencies() {
                println!("- {blocker}");
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn adopt(target: PathBuf, id: &str) {
    match spielgantt_lib::task::adopt(&target, id) {
        Ok(spielgantt_lib::task::AdoptTaskOutcome::Created) => {
            println!("Adopted task '{id}' in {}", target.display());
        }
        Ok(spielgantt_lib::task::AdoptTaskOutcome::AlreadyAdopted) => {
            println!("Task '{id}' is already adopted in {}", target.display());
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn parse_status(value: &str) -> Result<TaskStatus, String> {
    match value {
        "unblocked" => Ok(TaskStatus::Unblocked),
        "blocked" => Ok(TaskStatus::Blocked),
        "done" => Ok(TaskStatus::Done),
        _ => Err(format!(
            "invalid status '{value}': expected one of blocked, unblocked, done"
        )),
    }
}

fn format_status(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Unblocked => "unblocked",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Done => "done",
    }
}
