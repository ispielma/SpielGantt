use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{
    metadata::TaskStatus, project_payload::open_project, project_payload::OpenProjectError, task,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEdit {
    pub status: Option<String>,
    pub readme_content: String,
    pub expected_readme_version: String,
}

pub fn apply_task_edit(
    project_path: &Path,
    task_id: &str,
    edit: TaskEdit,
) -> Result<(), TaskEditError> {
    let project = open_project(project_path).map_err(TaskEditError::RefreshProject)?;
    let task = project
        .tasks()
        .iter()
        .find(|task| task.id() == task_id)
        .ok_or_else(|| TaskEditError::TaskNotFound(task_id.to_string()))?;
    let current_readme_version = task.readme_version().to_string();
    if current_readme_version != edit.expected_readme_version {
        return Err(TaskEditError::StaleReadme {
            task_id: task_id.to_string(),
        });
    }

    let update = task::TaskUpdate {
        status: parse_optional_status(edit.status.as_deref())?,
    };
    let planned_update =
        task::plan_update(project_path, task_id, update).map_err(TaskEditError::UpdateTask)?;
    let readme_path = task.path().join("README.md");
    let original_readme_content = if readme_path.is_file() {
        std::fs::read_to_string(&readme_path).map_err(|source| TaskEditError::ReadReadme {
            path: readme_path.clone(),
            source,
        })?
    } else {
        String::new()
    };
    let rewrites = [
        TaskEditRewrite::TaskMetadata(planned_update.into_rewrite()),
        TaskEditRewrite::Readme {
            path: readme_path,
            original_contents: original_readme_content,
            updated_contents: edit.readme_content,
        },
    ];
    apply_task_edit_rewrites_with_rollback(&rewrites)
}

enum TaskEditRewrite {
    TaskMetadata(crate::MetadataRewrite),
    Readme {
        path: PathBuf,
        original_contents: String,
        updated_contents: String,
    },
}

fn apply_task_edit_rewrites_with_rollback(
    rewrites: &[TaskEditRewrite],
) -> Result<(), TaskEditError> {
    let mut applied_indices = Vec::new();

    for (index, rewrite) in rewrites.iter().enumerate() {
        if rewrite.original_contents() == rewrite.updated_contents() {
            continue;
        }

        if let Err(error) = write_task_edit_rewrite(rewrite, RewriteSide::Updated) {
            for rollback_index in applied_indices.iter().rev().copied() {
                let _ = write_task_edit_rewrite(&rewrites[rollback_index], RewriteSide::Original);
            }
            return Err(error);
        }

        applied_indices.push(index);
    }

    Ok(())
}

fn write_task_edit_rewrite(
    rewrite: &TaskEditRewrite,
    side: RewriteSide,
) -> Result<(), TaskEditError> {
    match rewrite {
        TaskEditRewrite::TaskMetadata(rewrite) => {
            task::write_task_metadata_file(rewrite.path.as_path(), side.contents(rewrite)).map_err(
                |source| TaskEditError::UpdateTask(task::UpdateTaskError::WriteMetadata(source)),
            )
        }
        TaskEditRewrite::Readme {
            path,
            original_contents,
            updated_contents,
        } => {
            let contents = match side {
                RewriteSide::Original => original_contents,
                RewriteSide::Updated => updated_contents,
            };
            std::fs::write(path, contents).map_err(|source| TaskEditError::WriteReadme {
                path: path.clone(),
                source,
            })
        }
    }
}

#[derive(Clone, Copy)]
enum RewriteSide {
    Original,
    Updated,
}

impl RewriteSide {
    fn contents<'a>(&self, rewrite: &'a crate::MetadataRewrite) -> &'a str {
        match self {
            Self::Original => &rewrite.original_contents,
            Self::Updated => &rewrite.updated_contents,
        }
    }
}

impl TaskEditRewrite {
    fn original_contents(&self) -> &str {
        match self {
            Self::TaskMetadata(rewrite) => &rewrite.original_contents,
            Self::Readme {
                original_contents, ..
            } => original_contents,
        }
    }

    fn updated_contents(&self) -> &str {
        match self {
            Self::TaskMetadata(rewrite) => &rewrite.updated_contents,
            Self::Readme {
                updated_contents, ..
            } => updated_contents,
        }
    }
}

fn parse_optional_status(value: Option<&str>) -> Result<Option<TaskStatus>, TaskEditError> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| match value.trim() {
            "unblocked" => Ok(TaskStatus::Unblocked),
            "blocked" => Ok(TaskStatus::Blocked),
            "done" => Ok(TaskStatus::Done),
            value => Err(TaskEditError::InvalidInput(format!(
                "invalid status '{value}': expected one of blocked, unblocked, done"
            ))),
        })
        .transpose()
}

#[derive(Debug)]
pub enum TaskEditError {
    InvalidInput(String),
    ReadReadme {
        path: PathBuf,
        source: std::io::Error,
    },
    RefreshProject(OpenProjectError),
    StaleReadme {
        task_id: String,
    },
    TaskNotFound(String),
    UpdateTask(task::UpdateTaskError),
    WriteReadme {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for TaskEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "{message}"),
            Self::ReadReadme { path, source } => {
                write!(
                    formatter,
                    "failed to read task README '{}': {source}",
                    path.display()
                )
            }
            Self::RefreshProject(source) => write!(formatter, "{source}"),
            Self::StaleReadme { task_id } => write!(
                formatter,
                "README for task '{task_id}' changed on disk; reload the project before saving"
            ),
            Self::TaskNotFound(task_id) => write!(formatter, "task '{task_id}' not found"),
            Self::UpdateTask(source) => write!(formatter, "{source}"),
            Self::WriteReadme { path, source } => {
                write!(
                    formatter,
                    "failed to write task README '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for TaskEditError {}
