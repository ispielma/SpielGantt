use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    metadata::{MetadataError, TaskStatus},
    project, project_graph, task,
    task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    project_root: PathBuf,
    events: Vec<String>,
    tasks: Vec<ProjectSnapshotTask>,
    task_indexes_by_id: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshotTask {
    id: String,
    path: PathBuf,
    project_relative_path: PathBuf,
    dependencies: Vec<String>,
    blocks: Vec<ProjectSnapshotDependencyTarget>,
    dependency_references: Vec<ProjectSnapshotDependencyReference>,
    dependency_targets: Vec<ProjectSnapshotDependencyTarget>,
    ends_at: Option<String>,
    status: Option<TaskStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshotDependencyReference {
    id: String,
    kind: ProjectSnapshotDependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshotDependencyTarget {
    id: String,
    kind: ProjectSnapshotDependencyKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSnapshotDependencyKind {
    Task,
    Event,
}

impl ProjectSnapshot {
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn events(&self) -> &[String] {
        &self.events
    }

    pub fn tasks(&self) -> &[ProjectSnapshotTask] {
        &self.tasks
    }

    pub fn task(&self, id: &str) -> Option<&ProjectSnapshotTask> {
        self.task_indexes_by_id
            .get(id)
            .map(|index| &self.tasks[*index])
    }

    pub fn tasks_ending_at(&self, event_id: &str) -> Vec<&ProjectSnapshotTask> {
        self.tasks
            .iter()
            .filter(|task| task.ends_at() == Some(event_id))
            .collect()
    }
}

impl ProjectSnapshotTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project_relative_path(&self) -> &Path {
        &self.project_relative_path
    }

    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub fn blocks(&self) -> &[ProjectSnapshotDependencyTarget] {
        &self.blocks
    }

    pub fn dependency_references(&self) -> &[ProjectSnapshotDependencyReference] {
        &self.dependency_references
    }

    pub fn dependency_targets(&self) -> &[ProjectSnapshotDependencyTarget] {
        &self.dependency_targets
    }

    pub fn ends_at(&self) -> Option<&str> {
        self.ends_at.as_deref()
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }
}

impl ProjectSnapshotDependencyReference {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ProjectSnapshotDependencyKind {
        self.kind
    }
}

impl ProjectSnapshotDependencyTarget {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ProjectSnapshotDependencyKind {
        self.kind
    }
}

pub fn load(start: &Path) -> Result<ProjectSnapshot, LoadProjectSnapshotError> {
    let index = TaskPackageIndex::load(start).map_err(LoadProjectSnapshotError::from)?;
    Ok(from_graph(index.graph()))
}

pub fn from_graph(graph: &project_graph::ProjectGraph) -> ProjectSnapshot {
    let project_root = graph.project_root().to_path_buf();
    let mut tasks = Vec::new();

    for graph_task in graph.tasks() {
        let dependency_references = graph_task
            .dependency_references()
            .iter()
            .map(|dependency| ProjectSnapshotDependencyReference {
                id: dependency.id().to_string(),
                kind: match dependency.kind() {
                    project_graph::ProjectGraphDependencyKind::Task => {
                        ProjectSnapshotDependencyKind::Task
                    }
                    project_graph::ProjectGraphDependencyKind::Event => {
                        ProjectSnapshotDependencyKind::Event
                    }
                },
            })
            .collect::<Vec<_>>();
        let blocks = graph
            .tasks()
            .iter()
            .filter(|candidate| {
                candidate.dependency_references().iter().any(|dependency| {
                    dependency.kind() == project_graph::ProjectGraphDependencyKind::Task
                        && dependency.id() == graph_task.id()
                })
            })
            .map(|candidate| ProjectSnapshotDependencyTarget {
                id: candidate.id().to_string(),
                kind: ProjectSnapshotDependencyKind::Task,
            })
            .collect::<Vec<_>>();
        let dependency_targets = graph
            .valid_dependency_targets(graph_task.id())
            .unwrap_or_default()
            .iter()
            .map(|target| ProjectSnapshotDependencyTarget {
                id: target.id().to_string(),
                kind: match target.kind() {
                    project_graph::ProjectGraphDependencyKind::Task => {
                        ProjectSnapshotDependencyKind::Task
                    }
                    project_graph::ProjectGraphDependencyKind::Event => {
                        ProjectSnapshotDependencyKind::Event
                    }
                },
            })
            .collect::<Vec<_>>();

        tasks.push(ProjectSnapshotTask {
            id: graph_task.id().to_string(),
            path: graph_task.path().to_path_buf(),
            project_relative_path: graph_task.project_relative_path().to_path_buf(),
            dependencies: graph_task.dependencies().to_vec(),
            blocks,
            dependency_references,
            dependency_targets,
            ends_at: graph_task.ends_at().map(str::to_string),
            status: graph_task.status().cloned(),
        });
    }

    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    let task_indexes_by_id = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.clone(), index))
        .collect();

    ProjectSnapshot {
        project_root,
        events: graph.events().to_vec(),
        tasks,
        task_indexes_by_id,
    }
}

impl From<project_graph::LoadProjectGraphError> for LoadProjectSnapshotError {
    fn from(error: project_graph::LoadProjectGraphError) -> Self {
        project_graph::map_load_error_with_scan_and_metadata(error)
    }
}

#[derive(Debug)]
pub enum LoadProjectSnapshotError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(task::ScanTasksError),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl std::fmt::Display for LoadProjectSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "SpielGantt project not found from '{}'",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::ReadMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to read task metadata '{}': {source}",
                    path.display()
                )
            }
            Self::ParseMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to parse task metadata '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for LoadProjectSnapshotError {}

impl project_graph::FromLoadProjectGraphErrorWithScanAndMetadata for LoadProjectSnapshotError {
    type ScanError = task::ScanTasksError;

    fn project_not_found(path: PathBuf) -> Self {
        Self::ProjectNotFound(path)
    }

    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self {
        Self::ReadProjectMetadata(source)
    }

    fn scan_error(source: Self::ScanError) -> Self {
        Self::ScanTasks(source)
    }

    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadMetadata { path, source }
    }

    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self {
        Self::ParseMetadata { path, source }
    }
}
