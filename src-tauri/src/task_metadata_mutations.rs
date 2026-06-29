use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    metadata::{validate_single_path_component, MetadataError, TaskStatus},
    project, project_graph,
    task::{ScanTasksError, WriteTaskMetadataFileError},
    task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddDependencyOutcome {
    Added { task_id: String, blocker_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveDependencyOutcome {
    Removed { task_id: String, blocker_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateTaskOutcome {
    Updated { task_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetEndsAtOutcome {
    Set { task_id: String, event_id: String },
    Cleared { task_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskUpdate {
    pub status: Option<TaskStatus>,
}

pub(crate) struct PlannedTaskUpdate {
    task_id: String,
    rewrite: crate::MetadataRewrite,
}

impl PlannedTaskUpdate {
    pub(crate) fn task_id(&self) -> &str {
        &self.task_id
    }

    pub(crate) fn into_rewrite(self) -> crate::MetadataRewrite {
        self.rewrite
    }
}

pub fn add_dependency(
    start: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<AddDependencyOutcome, AddDependencyError> {
    let index = TaskPackageIndex::load(start).map_err(project_graph::map_load_error_with_scan)?;
    let graph = index.graph();

    let task = index
        .task(task_id)
        .ok_or_else(|| AddDependencyError::TaskNotFound(task_id.to_string()))?;
    let blocker_task = graph.task(blocker_id);
    if blocker_task.is_none() && !graph.has_event(blocker_id) {
        return Err(AddDependencyError::DependencyTargetNotFound(
            blocker_id.to_string(),
        ));
    }

    if let Some(blocker_task) = blocker_task {
        if let Some(event_id) = blocker_task.ends_at() {
            return Err(AddDependencyError::BlockerTaskEndsAtEvent {
                task_id: task.metadata.id.clone(),
                blocker_id: blocker_task.id().to_string(),
                event_id: event_id.to_string(),
            });
        }
    }
    if graph.has_event(blocker_id) {
        if let Some(ends_at) = task.metadata.ends_at.as_deref() {
            if event_after_event(graph, blocker_id, ends_at) {
                return Err(AddDependencyError::EventBlockerAfterEndsAt {
                    task_id: task.metadata.id.clone(),
                    blocker_id: blocker_id.to_string(),
                    event_id: ends_at.to_string(),
                });
            }
        }
    }

    if graph.dependency_would_create_cycle(&task.metadata.id, blocker_id) {
        return Err(AddDependencyError::DependencyCycle {
            task_id: task.metadata.id.clone(),
            blocker_id: blocker_id.to_string(),
        });
    }
    let task_id = task.metadata.id.clone();

    let metadata_updates = index.plan_task_metadata_rewrites(
        |_, task_metadata| {
            if task_metadata.id == task_id
                && !task_metadata.dependencies.iter().any(|id| id == blocker_id)
            {
                task_metadata.dependencies.push(blocker_id.to_string());
            }
            Ok(None)
        },
        AddDependencyError::SerializeMetadata,
    )?;
    commit_task_metadata_rewrites(metadata_updates).map_err(AddDependencyError::WriteMetadata)?;

    Ok(AddDependencyOutcome::Added {
        task_id,
        blocker_id: blocker_id.to_string(),
    })
}

pub fn remove_dependency(
    start: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<RemoveDependencyOutcome, RemoveDependencyError> {
    let index = TaskPackageIndex::load(start).map_err(project_graph::map_load_error_with_scan)?;
    let task = index
        .task(task_id)
        .ok_or_else(|| RemoveDependencyError::TaskNotFound(task_id.to_string()))?;
    let task_id = task.metadata.id.clone();

    let metadata_updates = index.plan_task_metadata_rewrites(
        |_, task_metadata| {
            if task_metadata.id == task_id {
                task_metadata
                    .dependencies
                    .retain(|dependency| dependency != blocker_id);
            }
            Ok(None)
        },
        RemoveDependencyError::SerializeMetadata,
    )?;
    commit_task_metadata_rewrites(metadata_updates)
        .map_err(RemoveDependencyError::WriteMetadata)?;

    Ok(RemoveDependencyOutcome::Removed {
        task_id,
        blocker_id: blocker_id.to_string(),
    })
}

pub fn update(
    start: &Path,
    task_id: &str,
    update: TaskUpdate,
) -> Result<UpdateTaskOutcome, UpdateTaskError> {
    let planned_update = plan_update(start, task_id, update)?;
    let task_id = planned_update.task_id().to_string();
    commit_task_metadata_rewrites([planned_update.into_rewrite()])
        .map_err(UpdateTaskError::WriteMetadata)?;

    Ok(UpdateTaskOutcome::Updated { task_id })
}

pub(crate) fn plan_update(
    start: &Path,
    task_id: &str,
    update: TaskUpdate,
) -> Result<PlannedTaskUpdate, UpdateTaskError> {
    let index = TaskPackageIndex::load(start).map_err(project_graph::map_load_error_with_scan)?;
    let task = index
        .task(task_id)
        .ok_or_else(|| UpdateTaskError::TaskNotFound(task_id.to_string()))?;

    let mut task_metadata = task.metadata.clone();
    if let Some(status) = update.status {
        task_metadata.status = Some(status);
    }
    let updated_task_metadata_contents = task_metadata
        .to_json()
        .map_err(UpdateTaskError::SerializeMetadata)?;

    Ok(PlannedTaskUpdate {
        task_id: task.metadata.id.clone(),
        rewrite: crate::MetadataRewrite {
            path: task.path.join(".spielgantt").join("task.json"),
            original_contents: task.original_metadata_contents.clone(),
            updated_contents: updated_task_metadata_contents,
        },
    })
}

pub fn set_ends_at(
    start: &Path,
    task_id: &str,
    event_id: Option<&str>,
    clear: bool,
) -> Result<SetEndsAtOutcome, SetEndsAtError> {
    let index = TaskPackageIndex::load(start).map_err(project_graph::map_load_error_with_scan)?;
    let task = index
        .task(task_id)
        .ok_or_else(|| SetEndsAtError::TaskNotFound(task_id.to_string()))?;
    let task_id = task.metadata.id.clone();

    if clear {
        let metadata_updates = index.plan_task_metadata_rewrites(
            |_, task_metadata| {
                if task_metadata.id == task_id {
                    task_metadata.ends_at = None;
                }
                Ok(None)
            },
            SetEndsAtError::SerializeMetadata,
        )?;
        commit_task_metadata_rewrites(metadata_updates).map_err(SetEndsAtError::WriteMetadata)?;

        return Ok(SetEndsAtOutcome::Cleared { task_id });
    }

    let event_id = event_id.ok_or_else(|| SetEndsAtError::MissingEventIdForSet {
        task_id: task_id.clone(),
    })?;
    validate_single_path_component(event_id, "event id")
        .map_err(MetadataError::InvalidProjectEventId)
        .map_err(SetEndsAtError::InvalidEventId)?;

    if !index.graph().has_event(event_id) {
        return Err(SetEndsAtError::EventNotFound(event_id.to_string()));
    }
    if let Some(blocker_id) = event_blocker_after_requested_end(index.graph(), &task_id, event_id) {
        return Err(SetEndsAtError::EndsAtBeforeEventBlocker {
            task_id: task_id.clone(),
            event_id: event_id.to_string(),
            blocker_id,
        });
    }

    let metadata_updates = index.plan_task_metadata_rewrites(
        |_, task_metadata| {
            let mut healed_dependencies = Vec::new();
            let mut seen_dependencies = HashSet::new();
            for dependency in std::mem::take(&mut task_metadata.dependencies) {
                let healed_dependency = if dependency == task_id {
                    event_id.to_string()
                } else {
                    dependency
                };
                if seen_dependencies.insert(healed_dependency.clone()) {
                    healed_dependencies.push(healed_dependency);
                }
            }
            task_metadata.dependencies = healed_dependencies;

            if task_metadata.id == task_id {
                task_metadata.ends_at = Some(event_id.to_string());
            }
            Ok(None)
        },
        SetEndsAtError::SerializeMetadata,
    )?;

    commit_task_metadata_rewrites(metadata_updates).map_err(SetEndsAtError::WriteMetadata)?;

    Ok(SetEndsAtOutcome::Set {
        task_id,
        event_id: event_id.to_string(),
    })
}

fn commit_task_metadata_rewrites(
    rewrites: impl IntoIterator<Item = crate::MetadataRewrite>,
) -> Result<(), WriteTaskMetadataFileError> {
    crate::package_mutation::PackageMutation::metadata_rewrites(rewrites)
        .commit()
        .map(|_| ())
        .map_err(crate::package_mutation::MetadataCommitError::into_source)
        .map_err(WriteTaskMetadataFileError::from)
}

fn event_blocker_after_requested_end(
    graph: &project_graph::ProjectGraph,
    task_id: &str,
    event_id: &str,
) -> Option<String> {
    let requested_event_index = event_index(graph, event_id)?;
    graph
        .task(task_id)?
        .dependency_references()
        .iter()
        .filter(|dependency| dependency.kind() == project_graph::ProjectGraphDependencyKind::Event)
        .filter_map(|dependency| {
            event_index(graph, dependency.id()).map(|index| (index, dependency.id().to_string()))
        })
        .filter(|(index, _)| *index > requested_event_index)
        .max_by_key(|(index, _)| *index)
        .map(|(_, event_id)| event_id)
}

fn event_index(graph: &project_graph::ProjectGraph, event_id: &str) -> Option<usize> {
    graph
        .events()
        .iter()
        .position(|candidate| candidate == event_id)
}

fn event_after_event(
    graph: &project_graph::ProjectGraph,
    candidate_event_id: &str,
    reference_event_id: &str,
) -> bool {
    match (
        event_index(graph, candidate_event_id),
        event_index(graph, reference_event_id),
    ) {
        (Some(candidate_index), Some(reference_index)) => candidate_index > reference_index,
        _ => false,
    }
}

macro_rules! impl_task_mutation_load_error {
    ($error:ty) => {
        impl project_graph::FromLoadProjectGraphErrorWithScan for $error {
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
    };
}

impl_task_mutation_load_error!(AddDependencyError);
impl_task_mutation_load_error!(RemoveDependencyError);
impl_task_mutation_load_error!(UpdateTaskError);
impl_task_mutation_load_error!(SetEndsAtError);

#[derive(Debug)]
pub enum AddDependencyError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TaskNotFound(String),
    DependencyTargetNotFound(String),
    SerializeMetadata(MetadataError),
    BlockerTaskEndsAtEvent {
        task_id: String,
        blocker_id: String,
        event_id: String,
    },
    EventBlockerAfterEndsAt {
        task_id: String,
        blocker_id: String,
        event_id: String,
    },
    DependencyCycle {
        task_id: String,
        blocker_id: String,
    },
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for AddDependencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot add dependency from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::DependencyTargetNotFound(id) => {
                write!(formatter, "task or event id '{id}' was not found")
            }
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::BlockerTaskEndsAtEvent {
                task_id,
                blocker_id,
                event_id,
            } => {
                write!(
                    formatter,
                    "cannot add blocker '{blocker_id}' to task '{task_id}': task '{blocker_id}' ends at event '{event_id}'; depend on the event instead"
                )
            }
            Self::EventBlockerAfterEndsAt {
                task_id,
                blocker_id,
                event_id,
            } => {
                write!(
                    formatter,
                    "cannot add event blocker '{blocker_id}' to task '{task_id}' after ends_at event '{event_id}'"
                )
            }
            Self::DependencyCycle {
                task_id,
                blocker_id,
            } => {
                write!(
                    formatter,
                    "cannot add blocker '{blocker_id}' to task '{task_id}': dependency cycle would be created"
                )
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for AddDependencyError {}

#[derive(Debug)]
pub enum RemoveDependencyError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TaskNotFound(String),
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for RemoveDependencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot remove dependency from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for RemoveDependencyError {}

#[derive(Debug)]
pub enum UpdateTaskError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TaskNotFound(String),
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for UpdateTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot update task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for UpdateTaskError {}

#[derive(Debug)]
pub enum SetEndsAtError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TaskNotFound(String),
    MissingEventIdForSet {
        task_id: String,
    },
    InvalidEventId(MetadataError),
    EventNotFound(String),
    EndsAtBeforeEventBlocker {
        task_id: String,
        event_id: String,
        blocker_id: String,
    },
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteTaskMetadataFileError),
}

impl std::fmt::Display for SetEndsAtError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot set task ends_at from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::MissingEventIdForSet { task_id } => {
                write!(
                    formatter,
                    "cannot set ends_at for task '{task_id}' without an event id"
                )
            }
            Self::InvalidEventId(source) => write!(formatter, "{source}"),
            Self::EventNotFound(id) => write!(formatter, "event id '{id}' was not found"),
            Self::EndsAtBeforeEventBlocker {
                task_id,
                event_id,
                blocker_id,
            } => {
                write!(
                    formatter,
                    "cannot set task '{task_id}' to end at event '{event_id}' before existing event blocker '{blocker_id}'"
                )
            }
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for SetEndsAtError {}
