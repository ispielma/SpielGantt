use clap::Subcommand;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process,
};

use super::{
    package::{validation_json, ValidationJson},
    print_cli_error, print_json, target_or_current,
};

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Prepare {
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Runtime {
        #[arg(long)]
        json: bool,
    },
    Status {
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Snapshot {
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct AgentSnapshotJson {
    schema_version: u32,
    project_root: String,
    agent: AgentStatusJson,
    validation: ValidationJson,
    paths: AgentPathsJson,
    tasks: Vec<AgentSnapshotTaskJson>,
    events: Vec<String>,
    dependencies: Vec<AgentSnapshotDependencyJson>,
}

#[derive(Debug, Serialize)]
struct AgentStatusReportJson {
    schema_version: u32,
    project_root: Option<String>,
    agent: AgentStatusJson,
    validation: ValidationJson,
}

#[derive(Debug, Serialize)]
struct AgentPrepareReportJson {
    schema_version: u32,
    project_root: String,
    outcome: spielgantt_lib::agent_scaffold::PrepareAgentScaffoldOutcome,
    files: Vec<spielgantt_lib::agent_scaffold::PreparedAgentFile>,
    agent: AgentStatusJson,
}

#[derive(Debug, Serialize)]
struct AgentStatusJson {
    ready: bool,
    agents_md_present: bool,
    skills_dir_present: bool,
    metadata_present: bool,
    recorded_cli_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentPathsJson {
    project_metadata: &'static str,
    agents_md: &'static str,
    skills_dir: &'static str,
    agent_metadata: &'static str,
}

#[derive(Debug, Serialize)]
struct AgentSnapshotTaskJson {
    id: String,
    path: String,
    project_relative_path: String,
    dependencies: Vec<String>,
    dependency_references:
        Vec<spielgantt_lib::project_snapshot::ProjectSnapshotDependencyReference>,
    ends_at: Option<String>,
    status: Option<spielgantt_lib::metadata::TaskStatus>,
}

#[derive(Debug, Serialize)]
struct AgentSnapshotDependencyJson {
    task_id: String,
    id: String,
    kind: spielgantt_lib::project_snapshot::ProjectSnapshotDependencyKind,
}

pub(crate) fn run(command: AgentCommand) {
    match command {
        AgentCommand::Prepare { path, json } => prepare(target_or_current(path), json),
        AgentCommand::Runtime { json } => runtime(json),
        AgentCommand::Status { path, json } => status(target_or_current(path), json),
        AgentCommand::Snapshot { path, json } => snapshot(target_or_current(path), json),
    }
}

fn prepare(target: PathBuf, json: bool) {
    let runtime = match spielgantt_lib::runtime::current_runtime_info(env!("CARGO_PKG_VERSION")) {
        Ok(runtime) => runtime,
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    };
    let report = match spielgantt_lib::agent_scaffold::prepare(&target, &runtime) {
        Ok(report) => report,
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    };

    if json {
        let project_root = PathBuf::from(report.project_root());
        print_json(&AgentPrepareReportJson {
            schema_version: 1,
            project_root: report.project_root().to_string(),
            outcome: report.outcome(),
            files: report.files().to_vec(),
            agent: agent_status_json(&project_root),
        });
        return;
    }

    println!(
        "Prepared agent scaffolding for {}: {:?}",
        report.project_root(),
        report.outcome()
    );
    for file in report.files() {
        println!("- {}: {:?}", file.path(), file.status());
    }
}

fn runtime(json: bool) {
    let runtime = match spielgantt_lib::runtime::current_runtime_info(env!("CARGO_PKG_VERSION")) {
        Ok(runtime) => runtime,
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    };

    if json {
        print_json(&runtime);
        return;
    }

    println!("Version: {}", runtime.version());
    println!("Executable path: {}", runtime.executable_path());
    println!("Package context: {:?}", runtime.package_context().kind());
    if let Some(package_path) = runtime.package_context().package_path() {
        println!("Package path: {package_path}");
    }
}

fn status(target: PathBuf, json: bool) {
    match spielgantt_lib::validation::validate(&target) {
        Ok(report) => {
            let project_root = report.project_root().map(Path::to_path_buf);
            let agent = project_root
                .as_deref()
                .map(agent_status_json)
                .unwrap_or(AgentStatusJson {
                    ready: false,
                    agents_md_present: false,
                    skills_dir_present: false,
                    metadata_present: false,
                    recorded_cli_path: None,
                });
            if json {
                print_json(&AgentStatusReportJson {
                    schema_version: 1,
                    project_root: project_root.as_ref().map(|path| path.display().to_string()),
                    agent,
                    validation: validation_json(&report),
                });
                return;
            }

            println!("Agent ready: {}", if agent.ready { "yes" } else { "no" });
            println!(
                "Validation: {}",
                if report.is_valid() {
                    "valid"
                } else {
                    "invalid"
                }
            );
            if let Some(project_root) = project_root {
                println!("Project root: {}", project_root.display());
            }
            if let Some(cli_path) = agent.recorded_cli_path {
                println!("Recorded CLI path: {cli_path}");
            }

            if !report.is_valid() {
                process::exit(1);
            }
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}

fn snapshot(target: PathBuf, _json: bool) {
    let validation_report = match spielgantt_lib::validation::validate(&target) {
        Ok(report) => report,
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    };
    if !validation_report.is_valid() {
        print_cli_error("agent snapshot requires a valid SpielGantt project");
        process::exit(1);
    }

    let projections = match spielgantt_lib::semantic_projection::load(&target) {
        Ok(projections) => projections,
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    };
    let snapshot = projections.snapshot();
    let project_root = snapshot.project_root();
    let mut dependencies = Vec::new();
    let tasks = snapshot
        .tasks()
        .iter()
        .map(|task| {
            dependencies.extend(task.dependency_references().iter().map(|dependency| {
                AgentSnapshotDependencyJson {
                    task_id: task.id().to_string(),
                    id: dependency.id().to_string(),
                    kind: dependency.kind(),
                }
            }));

            AgentSnapshotTaskJson {
                id: task.id().to_string(),
                path: task.path().display().to_string(),
                project_relative_path: task.project_relative_path().display().to_string(),
                dependencies: task.dependencies().to_vec(),
                dependency_references: task.dependency_references().to_vec(),
                ends_at: task.ends_at().map(str::to_string),
                status: task.status().cloned(),
            }
        })
        .collect::<Vec<_>>();

    let snapshot_json = AgentSnapshotJson {
        schema_version: 1,
        project_root: project_root.display().to_string(),
        agent: agent_status_json(project_root),
        validation: validation_json(&validation_report),
        paths: agent_paths_json(),
        tasks,
        events: snapshot.events().to_vec(),
        dependencies,
    };

    print_json(&snapshot_json);
}

fn agent_paths_json() -> AgentPathsJson {
    AgentPathsJson {
        project_metadata: ".spielgantt/project.json",
        agents_md: "AGENTS.md",
        skills_dir: ".agents/skills",
        agent_metadata: ".spielgantt/agent.json",
    }
}

fn agent_status_json(project_root: &Path) -> AgentStatusJson {
    let readiness = spielgantt_lib::agent_scaffold::readiness_status(project_root);
    AgentStatusJson {
        ready: readiness.ready(),
        agents_md_present: readiness.agents_md_present(),
        skills_dir_present: readiness.skills_dir_present(),
        metadata_present: readiness.metadata_present(),
        recorded_cli_path: readiness.recorded_cli_path().map(str::to_string),
    }
}
