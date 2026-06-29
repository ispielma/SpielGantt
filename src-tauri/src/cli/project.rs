use clap::Subcommand;
use serde::Serialize;
use std::process;

use super::{current_dir, print_cli_error, print_json};

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    UpdateReadme {
        #[arg(long)]
        content: String,
        #[arg(long, value_name = "README_VERSION")]
        expected_version: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct ProjectReadmeEditJson<'a> {
    schema_version: u32,
    project: &'a spielgantt_lib::app_facade::OpenProjectResult,
}

pub(crate) fn run(command: ProjectCommand) {
    match command {
        ProjectCommand::UpdateReadme {
            content,
            expected_version,
            json,
        } => update_readme(content, expected_version, json),
    }
}

fn update_readme(content: String, expected_version: String, json: bool) {
    match spielgantt_lib::app_facade::edit_project_readme(
        &current_dir(),
        spielgantt_lib::app_facade::ProjectReadmeEdit {
            readme_content: content,
            expected_readme_version: expected_version,
        },
    ) {
        Ok(result) => {
            if json {
                print_json(&ProjectReadmeEditJson {
                    schema_version: 1,
                    project: result.project(),
                });
                return;
            }
            let project_root = result
                .project()
                .project_root()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| current_dir().display().to_string());
            println!("Updated project README in {project_root}");
        }
        Err(error) => {
            print_cli_error(&error.to_string());
            process::exit(1);
        }
    }
}
