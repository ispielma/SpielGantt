use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    agent_scaffold,
    metadata::TaskMetadata,
    project,
    project_payload::{open_project, OpenProjectError, OpenProjectResult},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRefreshResult {
    project: OpenProjectResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgentPrepareResult {
    project: OpenProjectResult,
    outcome: agent_scaffold::PrepareAgentScaffoldOutcome,
    files: Vec<agent_scaffold::PreparedAgentFile>,
}

#[derive(Debug)]
pub enum CreateProjectError {
    CreateProject(project::CreateProjectError),
    PrepareAgents(agent_scaffold::PrepareAgentScaffoldError),
    RefreshProject(OpenProjectError),
}

impl ProjectRefreshResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }
}

impl ProjectAgentPrepareResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }

    pub fn outcome(&self) -> agent_scaffold::PrepareAgentScaffoldOutcome {
        self.outcome
    }

    pub fn files(&self) -> &[agent_scaffold::PreparedAgentFile] {
        &self.files
    }
}

impl std::fmt::Display for CreateProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateProject(source) => write!(formatter, "{source}"),
            Self::PrepareAgents(source) => write!(formatter, "{source}"),
            Self::RefreshProject(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CreateProjectError {}

pub fn refresh_project(path: &Path) -> Result<ProjectRefreshResult, OpenProjectError> {
    Ok(ProjectRefreshResult {
        project: open_project(path)?,
    })
}

pub fn prepare_agent_scaffolding(
    path: &Path,
) -> Result<ProjectAgentPrepareResult, PrepareAgentScaffoldingError> {
    let report = agent_scaffold::prepare_with_current_runtime(path)
        .map_err(PrepareAgentScaffoldingError::PrepareAgents)?;
    let project_root = PathBuf::from(report.project_root());
    let project =
        open_project(&project_root).map_err(PrepareAgentScaffoldingError::RefreshProject)?;
    Ok(ProjectAgentPrepareResult {
        project,
        outcome: report.outcome(),
        files: report.files().to_vec(),
    })
}

pub fn onboard_project(path: &Path) -> Result<OpenProjectResult, OnboardProjectError> {
    project::init(path).map_err(OnboardProjectError::InitProject)?;
    adopt_direct_child_task_buckets(path).map_err(OnboardProjectError::PrepareTaskBuckets)?;
    agent_scaffold::prepare_with_current_runtime(path)
        .map_err(OnboardProjectError::PrepareAgents)?;
    open_project(path).map_err(OnboardProjectError::RefreshProject)
}

pub fn create_project_in_parent(
    project_name: &str,
    parent_destination: &Path,
) -> Result<OpenProjectResult, CreateProjectError> {
    let project_root = project::create_in_parent(project_name, parent_destination)
        .map_err(CreateProjectError::CreateProject)?;
    finish_project_creation(&project_root)
}

fn finish_project_creation(project_root: &Path) -> Result<OpenProjectResult, CreateProjectError> {
    agent_scaffold::prepare_with_current_runtime(project_root)
        .map_err(CreateProjectError::PrepareAgents)?;
    open_project(project_root).map_err(CreateProjectError::RefreshProject)
}

fn adopt_direct_child_task_buckets(project_path: &Path) -> Result<(), PrepareTaskBucketsError> {
    let mut child_entries = std::fs::read_dir(project_path)
        .map_err(|source| PrepareTaskBucketsError::ReadChildren {
            path: project_path.to_path_buf(),
            source,
        })?
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(_) => None,
        })
        .collect::<Vec<_>>();
    child_entries.sort();

    for child_path in child_entries {
        if !child_path.is_dir() {
            continue;
        }

        let Some(folder_name) = child_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if folder_name.starts_with('.') {
            continue;
        }

        align_task_bucket_metadata(&child_path, folder_name)?;
    }

    Ok(())
}

fn align_task_bucket_metadata(
    task_path: &Path,
    task_id: &str,
) -> Result<(), PrepareTaskBucketsError> {
    let task_metadata_path = task_path.join(".spielgantt").join("task.json");
    if task_metadata_path.is_file() {
        let task_metadata_contents =
            std::fs::read_to_string(&task_metadata_path).map_err(|source| {
                PrepareTaskBucketsError::ReadTaskMetadata {
                    path: task_metadata_path.clone(),
                    source,
                }
            })?;
        TaskMetadata::from_json(&task_metadata_contents).map_err(|source| {
            PrepareTaskBucketsError::ParseTaskMetadata {
                path: task_metadata_path.clone(),
                source,
            }
        })?;
        return Ok(());
    }

    std::fs::create_dir_all(
        task_metadata_path
            .parent()
            .expect("task metadata should have a parent"),
    )
    .map_err(|source| PrepareTaskBucketsError::CreateTaskMetadataDir {
        path: task_metadata_path
            .parent()
            .expect("task metadata should have a parent")
            .to_path_buf(),
        source,
    })?;
    let task_metadata =
        TaskMetadata::version_1(task_id).map_err(PrepareTaskBucketsError::InvalidTaskId)?;
    let task_metadata_contents = task_metadata
        .to_json()
        .map_err(PrepareTaskBucketsError::SerializeTaskMetadata)?;
    write_task_metadata_file(&task_metadata_path, &task_metadata_contents)
        .map_err(PrepareTaskBucketsError::WriteTaskMetadata)?;

    Ok(())
}

fn write_task_metadata_file(
    path: &Path,
    contents: &str,
) -> Result<(), PrepareTaskBucketsWriteError> {
    crate::package_mutation::write_json_metadata_file(path, contents)
        .map_err(PrepareTaskBucketsWriteError::from)
}

#[derive(Debug)]
pub enum OnboardProjectError {
    InitProject(project::InitProjectError),
    PrepareTaskBuckets(PrepareTaskBucketsError),
    PrepareAgents(agent_scaffold::PrepareAgentScaffoldError),
    RefreshProject(OpenProjectError),
}

#[derive(Debug)]
pub enum PrepareAgentScaffoldingError {
    PrepareAgents(agent_scaffold::PrepareAgentScaffoldError),
    RefreshProject(OpenProjectError),
}

impl std::fmt::Display for PrepareAgentScaffoldingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrepareAgents(source) => write!(formatter, "{source}"),
            Self::RefreshProject(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for PrepareAgentScaffoldingError {}

impl std::fmt::Display for OnboardProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitProject(source) => write!(formatter, "{source}"),
            Self::PrepareTaskBuckets(source) => write!(formatter, "{source}"),
            Self::PrepareAgents(source) => write!(formatter, "{source}"),
            Self::RefreshProject(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for OnboardProjectError {}

#[derive(Debug)]
pub enum PrepareTaskBucketsError {
    ReadChildren {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateTaskMetadataDir {
        path: PathBuf,
        source: std::io::Error,
    },
    InvalidTaskId(crate::metadata::MetadataError),
    ReadTaskMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseTaskMetadata {
        path: PathBuf,
        source: crate::metadata::MetadataError,
    },
    SerializeTaskMetadata(crate::metadata::MetadataError),
    WriteTaskMetadata(PrepareTaskBucketsWriteError),
}

impl std::fmt::Display for PrepareTaskBucketsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadChildren { path, source } => {
                write!(
                    formatter,
                    "failed to read project children for onboarding in '{}': {source}",
                    path.display()
                )
            }
            Self::CreateTaskMetadataDir { path, source } => {
                write!(
                    formatter,
                    "failed to create task metadata directory '{}': {source}",
                    path.display()
                )
            }
            Self::InvalidTaskId(source) => write!(formatter, "{source}"),
            Self::ReadTaskMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to read task metadata '{}': {source}",
                    path.display()
                )
            }
            Self::ParseTaskMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to parse task metadata '{}': {source}",
                    path.display()
                )
            }
            Self::SerializeTaskMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::WriteTaskMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for PrepareTaskBucketsError {}

#[derive(Debug)]
pub struct PrepareTaskBucketsWriteError(crate::package_mutation::AtomicJsonMetadataWriteError);

impl From<crate::package_mutation::AtomicJsonMetadataWriteError> for PrepareTaskBucketsWriteError {
    fn from(source: crate::package_mutation::AtomicJsonMetadataWriteError) -> Self {
        Self(source)
    }
}

impl std::fmt::Display for PrepareTaskBucketsWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            crate::package_mutation::AtomicJsonMetadataWriteError::Write { path, source } => {
                write!(
                    formatter,
                    "failed to write temporary task metadata '{}': {source}",
                    path.display()
                )
            }
            crate::package_mutation::AtomicJsonMetadataWriteError::Replace { from, to, source } => {
                write!(
                    formatter,
                    "failed to replace task metadata '{}' with '{}': {source}",
                    to.display(),
                    from.display()
                )
            }
        }
    }
}

impl std::error::Error for PrepareTaskBucketsWriteError {}
