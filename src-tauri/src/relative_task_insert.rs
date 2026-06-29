use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    metadata::{MetadataError, TaskMetadata},
    package_mutation::PackageMutation,
    project, project_graph,
    project_namespace::{ProjectNamespace, ProjectNamespaceError},
    task::{
        create_prepared_task_bucket, normalized_task_folder_name, CreateTaskBucketError,
        ScanTasksError, WriteTaskMetadataFileError,
    },
    task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelativeInsertMode {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertRelativeTaskOutcome {
    Inserted {
        mode: RelativeInsertMode,
        selected_task_id: String,
        inserted_task_id: String,
        task_path: PathBuf,
    },
}

pub fn insert_before(
    start: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<InsertRelativeTaskOutcome, InsertRelativeTaskError> {
    insert_relative(
        start,
        selected_task_id,
        inserted_task_id,
        RelativeInsertMode::Before,
    )
}

pub fn insert_after(
    start: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<InsertRelativeTaskOutcome, InsertRelativeTaskError> {
    insert_relative(
        start,
        selected_task_id,
        inserted_task_id,
        RelativeInsertMode::After,
    )
}

fn insert_relative(
    start: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
    mode: RelativeInsertMode,
) -> Result<InsertRelativeTaskOutcome, InsertRelativeTaskError> {
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let project_root = package_index.project_root().to_path_buf();
    let project_metadata = package_index.project_metadata();
    let mut inserted_task_metadata = TaskMetadata::version_1(inserted_task_id)
        .map_err(InsertRelativeTaskError::InvalidTaskId)?;
    let deferred_duplicate_task_error = match ProjectNamespace::from_package_index(&package_index)
        .validate_new_task_id(&inserted_task_metadata.id)
    {
        Ok(()) => None,
        Err(error @ ProjectNamespaceError::TaskIdCollidesWithProjectTask { .. }) => Some(error),
        Err(error) => return Err(map_namespace_error_to_insert_relative_task(error)),
    };

    let selected_task = package_index.task(selected_task_id).ok_or_else(|| {
        InsertRelativeTaskError::SelectedTaskNotFound(selected_task_id.to_string())
    })?;

    if let Some(error) = deferred_duplicate_task_error {
        return Err(map_namespace_error_to_insert_relative_task(error));
    }

    let task_path = project_root.join(normalized_task_folder_name(
        &project_metadata.folder_naming,
        &inserted_task_metadata.id,
    ));
    if task_path.exists() {
        return Err(InsertRelativeTaskError::FolderAlreadyExists(task_path));
    }

    let selected_starting_dependencies = selected_task.metadata.dependencies.clone();
    let selected_ends_at = selected_task.metadata.ends_at.clone();
    match mode {
        RelativeInsertMode::Before => {
            inserted_task_metadata.dependencies = selected_starting_dependencies.clone();
        }
        RelativeInsertMode::After => {
            inserted_task_metadata
                .dependencies
                .push(selected_task_id.to_string());
            inserted_task_metadata.ends_at = selected_ends_at.clone();
        }
    }

    let metadata_updates = package_index.plan_task_metadata_rewrites(
        |_, task_metadata| {
            match mode {
                RelativeInsertMode::Before => {
                    if task_metadata.id == selected_task_id {
                        task_metadata.dependencies = vec![inserted_task_metadata.id.clone()];
                    }
                }
                RelativeInsertMode::After => {
                    if task_metadata.id == selected_task_id {
                        task_metadata.ends_at = None;
                    } else {
                        let replacement = selected_ends_at
                            .as_ref()
                            .unwrap_or(&inserted_task_metadata.id);
                        replace_dependency_once(
                            &mut task_metadata.dependencies,
                            selected_task_id,
                            replacement,
                        );
                    }
                }
            }
            Ok(None)
        },
        InsertRelativeTaskError::SerializeMetadata,
    )?;

    create_prepared_task_bucket(&task_path, &inserted_task_metadata)
        .map_err(create_inserted_task_error_from_bucket)
        .map_err(InsertRelativeTaskError::CreateInsertedTask)?;

    if let Err(error) = PackageMutation::metadata_rewrites(metadata_updates)
        .commit()
        .map_err(crate::package_mutation::MetadataCommitError::into_source)
        .map_err(WriteTaskMetadataFileError::from)
    {
        let _ = std::fs::remove_dir_all(&task_path);
        return Err(InsertRelativeTaskError::WriteMetadata(error));
    }

    Ok(InsertRelativeTaskOutcome::Inserted {
        mode,
        selected_task_id: selected_task_id.to_string(),
        inserted_task_id: inserted_task_metadata.id,
        task_path,
    })
}

fn map_namespace_error_to_insert_relative_task(
    error: ProjectNamespaceError,
) -> InsertRelativeTaskError {
    match error {
        ProjectNamespaceError::InvalidTaskId(source) => {
            InsertRelativeTaskError::InvalidTaskId(source)
        }
        ProjectNamespaceError::TaskIdCollidesWithProjectEvent { id } => {
            InsertRelativeTaskError::TaskIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::TaskIdCollidesWithProjectTask { id, existing_path } => {
            InsertRelativeTaskError::DuplicateTaskId { id, existing_path }
        }
        ProjectNamespaceError::InvalidEventId(_)
        | ProjectNamespaceError::EventIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::EventIdCollidesWithProjectEvent { .. } => {
            unreachable!("task namespace validation should only return task namespace errors")
        }
    }
}

fn replace_dependency_once(dependencies: &mut Vec<String>, old_id: &str, new_id: &str) {
    let mut rewired_dependencies = Vec::new();
    let mut seen_dependencies = HashSet::new();

    for dependency in std::mem::take(dependencies) {
        let dependency = if dependency == old_id {
            new_id.to_string()
        } else {
            dependency
        };
        if seen_dependencies.insert(dependency.clone()) {
            rewired_dependencies.push(dependency);
        }
    }

    *dependencies = rewired_dependencies;
}

fn create_inserted_task_error_from_bucket(
    error: CreateTaskBucketError,
) -> CreateInsertedTaskFilesError {
    match error {
        CreateTaskBucketError::CreateTaskDir { path, source } => {
            CreateInsertedTaskFilesError::CreateTaskDir { path, source }
        }
        CreateTaskBucketError::CreateDir { path, source } => {
            CreateInsertedTaskFilesError::CreateDir { path, source }
        }
        CreateTaskBucketError::WriteReadme { path, source } => {
            CreateInsertedTaskFilesError::WriteReadme { path, source }
        }
        CreateTaskBucketError::SerializeMetadata(source) => {
            CreateInsertedTaskFilesError::SerializeMetadata(source)
        }
        CreateTaskBucketError::WriteMetadata { source, .. } => {
            CreateInsertedTaskFilesError::WriteMetadata(source)
        }
    }
}

#[derive(Debug)]
pub enum InsertRelativeTaskError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    InvalidTaskId(MetadataError),
    TaskIdCollidesWithProjectEvent { id: String },
    ScanTasks(ScanTasksError),
    SelectedTaskNotFound(String),
    DuplicateTaskId { id: String, existing_path: PathBuf },
    FolderAlreadyExists(PathBuf),
    SerializeMetadata(MetadataError),
    CreateInsertedTask(CreateInsertedTaskFilesError),
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for InsertRelativeTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot insert task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::InvalidTaskId(source) => write!(formatter, "{source}"),
            Self::TaskIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "task id '{id}' collides with project event id '{id}'"
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::SelectedTaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::DuplicateTaskId { id, existing_path } => {
                write!(
                    formatter,
                    "task id '{id}' already exists in '{}'",
                    existing_path.display()
                )
            }
            Self::FolderAlreadyExists(path) => {
                write!(formatter, "task folder '{}' already exists", path.display())
            }
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::CreateInsertedTask(source) => write!(formatter, "{source}"),
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for InsertRelativeTaskError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for InsertRelativeTaskError {
    type ScanError = ScanTasksError;

    fn project_not_found(path: PathBuf) -> Self {
        Self::ProjectNotFound(path)
    }

    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self {
        Self::ReadProjectMetadata(source)
    }

    fn scan_error(source: Self::ScanError) -> Self {
        Self::ScanTasks(source)
    }
}

#[derive(Debug)]
pub enum CreateInsertedTaskFilesError {
    CreateTaskDir {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteReadme {
        path: PathBuf,
        source: std::io::Error,
    },
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for CreateInsertedTaskFilesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateTaskDir { path, source } => {
                write!(
                    formatter,
                    "failed to create task folder '{}': {source}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create metadata directory '{}': {source}",
                    path.display()
                )
            }
            Self::WriteReadme { path, source } => {
                write!(
                    formatter,
                    "failed to write task README '{}': {source}",
                    path.display()
                )
            }
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CreateInsertedTaskFilesError {}
