use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    metadata::TaskMetadata,
    project, project_graph,
    task::{read_task_metadata, AdoptTaskError, ScanTasksError},
    task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptableTaskFolder {
    folder_path: PathBuf,
    project_relative_path: String,
    task_id: String,
}

impl AdoptableTaskFolder {
    pub fn folder_path(&self) -> &Path {
        &self.folder_path
    }

    pub fn project_relative_path(&self) -> &str {
        &self.project_relative_path
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

pub fn list_adoptable_task_folders(
    start: &Path,
) -> Result<Vec<AdoptableTaskFolder>, ListAdoptableTaskFoldersError> {
    let index = TaskPackageIndex::load(start)
        .map_err(project_graph::map_load_error_with_scan::<ListAdoptableTaskFoldersError>)?;
    let project_root = index.project_root();
    let existing_task_ids = index
        .loaded_tasks()
        .iter()
        .map(|task| task.metadata.id.clone())
        .collect::<HashSet<_>>();
    let mut folders = Vec::new();

    for entry in std::fs::read_dir(project_root).map_err(|source| {
        ListAdoptableTaskFoldersError::ReadProjectDirectory {
            path: project_root.to_path_buf(),
            source,
        }
    })? {
        let entry =
            entry.map_err(
                |source| ListAdoptableTaskFoldersError::ReadProjectDirectory {
                    path: project_root.to_path_buf(),
                    source,
                },
            )?;
        let path = entry.path();
        let file_type =
            entry
                .file_type()
                .map_err(|source| ListAdoptableTaskFoldersError::ReadEntryType {
                    path: path.clone(),
                    source,
                })?;
        if !file_type.is_dir() {
            continue;
        }

        let Some(folder_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if folder_name.starts_with('.') || read_task_metadata(&path)?.is_some() {
            continue;
        }
        if TaskMetadata::version_1(&folder_name).is_err()
            || existing_task_ids.contains(&folder_name)
        {
            continue;
        }

        folders.push(AdoptableTaskFolder {
            folder_path: path,
            project_relative_path: folder_name.clone(),
            task_id: folder_name,
        });
    }

    folders.sort_by(|left, right| {
        left.project_relative_path
            .to_lowercase()
            .cmp(&right.project_relative_path.to_lowercase())
            .then_with(|| left.project_relative_path.cmp(&right.project_relative_path))
    });
    Ok(folders)
}

#[derive(Debug)]
pub enum ListAdoptableTaskFoldersError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ReadProjectDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadEntryType {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadTaskMetadata(AdoptTaskError),
    ScanTasks(ScanTasksError),
}

impl From<AdoptTaskError> for ListAdoptableTaskFoldersError {
    fn from(error: AdoptTaskError) -> Self {
        Self::ReadTaskMetadata(error)
    }
}

impl std::fmt::Display for ListAdoptableTaskFoldersError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => write!(
                formatter,
                "cannot list adoptable task folders from '{}': not inside a SpielGantt project",
                path.display()
            ),
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ReadProjectDirectory { path, source } => write!(
                formatter,
                "failed to read project directory '{}': {source}",
                path.display()
            ),
            Self::ReadEntryType { path, source } => write!(
                formatter,
                "failed to inspect project child '{}': {source}",
                path.display()
            ),
            Self::ReadTaskMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for ListAdoptableTaskFoldersError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for ListAdoptableTaskFoldersError {
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
