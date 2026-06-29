use clap::Subcommand;
use serde::Serialize;
use std::{path::PathBuf, process};

use super::{print_cli_error, print_json, target_or_current};

#[derive(Debug, Subcommand)]
pub(crate) enum CacheCommand {
    Rebuild { path: Option<PathBuf> },
}

#[derive(Debug, Serialize)]
pub(crate) struct ValidationJson {
    schema_version: u32,
    valid: bool,
    project_root: Option<String>,
    issues: Vec<String>,
}

pub(crate) fn validation_json(
    report: &spielgantt_lib::validation::ValidationReport,
) -> ValidationJson {
    ValidationJson {
        schema_version: 1,
        valid: report.is_valid(),
        project_root: report.project_root().map(|path| path.display().to_string()),
        issues: report
            .issues()
            .iter()
            .map(|issue| issue.message().to_string())
            .collect(),
    }
}

pub(crate) fn run_cache(command: CacheCommand) {
    match command {
        CacheCommand::Rebuild { path } => cache_rebuild(target_or_current(path)),
    }
}

pub(crate) fn init(target: PathBuf) {
    match spielgantt_lib::project::init(&target) {
        Ok(spielgantt_lib::project::InitProjectOutcome::Created) => {
            println!("Initialized SpielGantt project in {}", target.display());
        }
        Ok(spielgantt_lib::project::InitProjectOutcome::AlreadyExists) => {
            println!("SpielGantt project already exists in {}", target.display());
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }

    if let Err(error) = spielgantt_lib::agent_scaffold::prepare_with_current_runtime(&target) {
        print_cli_error(&error.to_string());
        process::exit(1);
    }
}

pub(crate) fn validate(target: PathBuf, json: bool) {
    match spielgantt_lib::validation::validate(&target) {
        Ok(report) if report.is_valid() => {
            if json {
                print_json(&validation_json(&report));
            } else {
                let project_root = report
                    .project_root()
                    .expect("valid reports should include a project root");
                println!("SpielGantt project is valid: {}", project_root.display());
            }
        }
        Ok(report) => {
            if json {
                print_json(&validation_json(&report));
            } else {
                eprintln!("SpielGantt project is invalid:");
                for issue in report.issues() {
                    eprintln!("- {}", issue.message());
                }
            }
            process::exit(1);
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

pub(crate) fn export(target: PathBuf) {
    match spielgantt_lib::export::project_markdown(&target) {
        Ok(markdown) => {
            print!("{markdown}");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

pub(crate) fn repair(target: PathBuf) {
    match spielgantt_lib::repair::report(&target) {
        Ok(report) if report.is_clean() => {
            println!(
                "No repair issues found: {}",
                report.project_root().display()
            );
        }
        Ok(report) => {
            eprintln!("Repair found {} issue(s):", report.issues().len());
            for issue in report.issues() {
                eprintln!("- {}", issue.message());
            }
            process::exit(1);
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

pub(crate) fn normalize(target: PathBuf, apply: bool) {
    if apply {
        normalize_apply(target);
    } else {
        normalize_dry_run(target);
    }
}

fn cache_rebuild(target: PathBuf) {
    match spielgantt_lib::cache::rebuild(&target) {
        Ok(outcome) => {
            let suffix = if outcome.task_count() == 1 { "" } else { "s" };
            println!(
                "Rebuilt cache with {} task{}: {}",
                outcome.task_count(),
                suffix,
                outcome.path().display()
            );
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn normalize_apply(target: PathBuf) {
    match spielgantt_lib::task::apply_normalization(&target) {
        Ok(renames) => {
            if renames.is_empty() {
                println!("Renamed 0 task folders.");
            } else {
                println!("Renamed task folders:");
                for rename in renames {
                    println!(
                        "- {}: {} -> {}",
                        rename.id(),
                        rename.from().display(),
                        rename.to().display()
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

fn normalize_dry_run(target: PathBuf) {
    match spielgantt_lib::mutation_plan::plan_task_folder_normalization(&target) {
        Ok(plan) => {
            if plan.operations().is_empty() {
                println!("Dry run: all task folders already match task IDs.");
            } else {
                println!("Dry run: planned task folder renames:");
                for operation in plan.operations() {
                    match operation {
                        spielgantt_lib::mutation_plan::MutationPlanOperation::RenameTaskFolder {
                            task_id,
                            from,
                            to,
                        } => {
                            println!("- {task_id}: {} -> {}", from.display(), to.display());
                        }
                    }
                }
            }

            if !plan.is_safe() {
                for issue in plan.preflight_issues() {
                    print_cli_error(&format!("Dry run preflight failed: {}", issue.message()));
                }
                process::exit(1);
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}
