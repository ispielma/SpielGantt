use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    metadata::MetadataError, project, project_graph, task, task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyRelationships {
    schema_version: u32,
    tasks: Vec<DependencyRelationshipsTask>,
    events: Vec<DependencyRelationshipsEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyRelationshipsTask {
    id: String,
    blockers: Vec<DependencyNode>,
    blocks: Vec<DependencyNode>,
    valid_dependency_targets: Vec<DependencyNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyRelationshipsEvent {
    id: String,
    references: Vec<EventReference>,
    deletion_blockers: Vec<EventReference>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DependencyNode {
    id: String,
    kind: DependencyNodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyNodeKind {
    Task,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventReference {
    task_id: String,
    kind: EventReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventReferenceKind {
    Dependency,
    EndsAt,
}

impl DependencyRelationships {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn tasks(&self) -> &[DependencyRelationshipsTask] {
        &self.tasks
    }

    pub fn events(&self) -> &[DependencyRelationshipsEvent] {
        &self.events
    }
}

impl DependencyRelationshipsTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn blockers(&self) -> &[DependencyNode] {
        &self.blockers
    }

    pub fn blocks(&self) -> &[DependencyNode] {
        &self.blocks
    }

    pub fn valid_dependency_targets(&self) -> &[DependencyNode] {
        &self.valid_dependency_targets
    }
}

impl DependencyRelationshipsEvent {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn references(&self) -> &[EventReference] {
        &self.references
    }

    pub fn deletion_blockers(&self) -> &[EventReference] {
        &self.deletion_blockers
    }
}

impl DependencyNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> DependencyNodeKind {
        self.kind
    }
}

impl EventReference {
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn kind(&self) -> EventReferenceKind {
        self.kind
    }
}

pub fn load(start: &Path) -> Result<DependencyRelationships, LoadDependencyRelationshipsError> {
    let index = TaskPackageIndex::load(start)
        .map_err(project_graph::map_load_error_with_scan_and_metadata)?;
    Ok(from_graph(index.graph()))
}

pub fn from_graph(graph: &project_graph::ProjectGraph) -> DependencyRelationships {
    let mut tasks = Vec::new();

    for graph_task in graph.tasks() {
        let blockers = graph_task
            .dependency_references()
            .iter()
            .map(|dependency| DependencyNode {
                id: dependency.id().to_string(),
                kind: dependency_node_kind(dependency.kind()),
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
            .map(|candidate| DependencyNode {
                id: candidate.id().to_string(),
                kind: DependencyNodeKind::Task,
            })
            .collect::<Vec<_>>();
        let valid_dependency_targets = graph
            .valid_dependency_targets(graph_task.id())
            .unwrap_or_default()
            .into_iter()
            .map(|target| DependencyNode {
                id: target.id().to_string(),
                kind: dependency_node_kind(target.kind()),
            })
            .collect::<Vec<_>>();

        tasks.push(DependencyRelationshipsTask {
            id: graph_task.id().to_string(),
            blockers,
            blocks,
            valid_dependency_targets,
        });
    }

    let events = graph
        .events()
        .iter()
        .map(|event_id| {
            let mut references = Vec::new();
            for graph_task in graph.tasks() {
                references.extend(
                    graph_task
                        .dependency_references()
                        .iter()
                        .filter(|dependency| {
                            dependency.kind() == project_graph::ProjectGraphDependencyKind::Event
                                && dependency.id() == event_id
                        })
                        .map(|_| EventReference {
                            task_id: graph_task.id().to_string(),
                            kind: EventReferenceKind::Dependency,
                        }),
                );
                if graph_task.ends_at() == Some(event_id.as_str()) {
                    references.push(EventReference {
                        task_id: graph_task.id().to_string(),
                        kind: EventReferenceKind::EndsAt,
                    });
                }
            }

            DependencyRelationshipsEvent {
                id: event_id.clone(),
                deletion_blockers: references.clone(),
                references,
            }
        })
        .collect::<Vec<_>>();

    DependencyRelationships {
        schema_version: 1,
        tasks,
        events,
    }
}

fn dependency_node_kind(kind: project_graph::ProjectGraphDependencyKind) -> DependencyNodeKind {
    match kind {
        project_graph::ProjectGraphDependencyKind::Task => DependencyNodeKind::Task,
        project_graph::ProjectGraphDependencyKind::Event => DependencyNodeKind::Event,
    }
}

#[derive(Debug)]
pub enum LoadDependencyRelationshipsError {
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

impl std::fmt::Display for LoadDependencyRelationshipsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "no SpielGantt project found at or above '{}'",
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

impl std::error::Error for LoadDependencyRelationshipsError {}

impl project_graph::FromLoadProjectGraphErrorWithScanAndMetadata
    for LoadDependencyRelationshipsError
{
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
