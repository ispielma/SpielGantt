use std::path::{Path, PathBuf};

use crate::{
    dependency_relationships,
    metadata::{MetadataError, ProjectMetadata},
    package_mutation::AtomicJsonMetadataWriteError,
    project, project_graph,
    project_namespace::{ProjectNamespace, ProjectNamespaceError},
    task,
    task_package_index::TaskPackageIndex,
    MetadataRewrite,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateEventOutcome {
    Created { event_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameEventOutcome {
    Renamed { old_id: String, new_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteEventOutcome {
    Deleted { event_id: String },
}

impl CreateEventOutcome {
    pub fn event_id(&self) -> &str {
        match self {
            Self::Created { event_id } => event_id,
        }
    }
}

impl RenameEventOutcome {
    pub fn old_id(&self) -> &str {
        match self {
            Self::Renamed { old_id, .. } => old_id,
        }
    }

    pub fn new_id(&self) -> &str {
        match self {
            Self::Renamed { new_id, .. } => new_id,
        }
    }
}

impl DeleteEventOutcome {
    pub fn event_id(&self) -> &str {
        match self {
            Self::Deleted { event_id } => event_id,
        }
    }
}

pub fn delete(start: &Path, id: &str) -> Result<DeleteEventOutcome, DeleteEventError> {
    let project_root = project::find_root(start)
        .ok_or_else(|| DeleteEventError::ProjectNotFound(start.to_path_buf()))?;
    let project_metadata =
        project::read_metadata(&project_root).map_err(DeleteEventError::ReadProjectMetadata)?;

    let Some(existing_event) = project_metadata
        .events
        .as_ref()
        .and_then(|events| events.iter().find(|event| event.as_str() == id))
    else {
        return Err(DeleteEventError::EventNotFound(id.to_string()));
    };

    let package_index = TaskPackageIndex::load(&project_root)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let referencing_task_ids = event_deletion_blocking_task_ids(&package_index, id);

    if !referencing_task_ids.is_empty() {
        return Err(DeleteEventError::EventReferenced {
            id: id.to_string(),
            task_ids: referencing_task_ids,
        });
    }

    if boundary_events_reference(&project_metadata, id) {
        return Err(DeleteEventError::BoundaryEventReferenced(id.to_string()));
    }

    let removed_event_id = existing_event.clone();
    let rewrite_plan = crate::mutation_plan::plan_project_metadata_rewrites(
        &project_root,
        |project_metadata| {
            if let Some(events) = project_metadata.events.as_mut() {
                events.retain(|event| event != id);
                if events.is_empty() {
                    project_metadata.events = None;
                }
            }
        },
        |_, _| {},
    )
    .map_err(map_project_metadata_rewrite_plan_delete_error)?;
    commit_event_metadata_rewrites(rewrite_plan_rewrites(&rewrite_plan))
        .map_err(DeleteEventError::WriteMetadata)?;

    Ok(DeleteEventOutcome::Deleted {
        event_id: removed_event_id,
    })
}

pub fn create(start: &Path, id: &str) -> Result<CreateEventOutcome, CreateEventError> {
    let project_root = project::find_root(start)
        .ok_or_else(|| CreateEventError::ProjectNotFound(start.to_path_buf()))?;
    let project_metadata =
        project::read_metadata(&project_root).map_err(CreateEventError::ReadProjectMetadata)?;

    ProjectNamespace::validate_event_id_format(id).map_err(map_namespace_error_to_create_event)?;

    let package_index = TaskPackageIndex::load(&project_root)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    ProjectNamespace::new(&project_metadata, package_index.loaded_tasks())
        .validate_new_event_id(id)
        .map_err(map_namespace_error_to_create_event)?;

    let rewrite_plan = crate::mutation_plan::plan_project_metadata_rewrites(
        &project_root,
        |project_metadata| {
            if project_metadata.boundary_events.is_none() {
                project_metadata.boundary_events = project_metadata.resolved_boundary_events();
            }
            match &mut project_metadata.events {
                Some(events) => {
                    if let Some(finish_index) =
                        project_metadata
                            .boundary_events
                            .as_ref()
                            .and_then(|boundary_events| {
                                events
                                    .iter()
                                    .position(|event| event == &boundary_events.finish)
                            })
                    {
                        events.insert(finish_index, id.to_string());
                    } else {
                        events.push(id.to_string());
                    }
                }
                None => project_metadata.events = Some(vec![id.to_string()]),
            }
        },
        |_, _| {},
    )
    .map_err(map_project_metadata_rewrite_plan_create_error)?;
    commit_event_metadata_rewrites(rewrite_plan_rewrites(&rewrite_plan))
        .map_err(CreateEventError::WriteMetadata)?;

    Ok(CreateEventOutcome::Created {
        event_id: id.to_string(),
    })
}

pub fn rename(
    start: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<RenameEventOutcome, RenameEventError> {
    let project_root = project::find_root(start)
        .ok_or_else(|| RenameEventError::ProjectNotFound(start.to_path_buf()))?;
    let project_metadata_path = project_root.join(".spielgantt").join("project.json");
    let project_metadata_contents =
        std::fs::read_to_string(&project_metadata_path).map_err(|source| {
            RenameEventError::ReadProjectMetadata(project::ReadProjectMetadataError::ReadMetadata {
                path: project_metadata_path.clone(),
                source,
            })
        })?;
    let project_metadata = crate::metadata::ProjectMetadata::from_json(&project_metadata_contents)
        .map_err(|source| {
            RenameEventError::ReadProjectMetadata(
                project::ReadProjectMetadataError::ParseMetadata {
                    path: project_metadata_path.clone(),
                    source,
                },
            )
        })?;

    ProjectNamespace::validate_event_id_format(new_id)
        .map_err(map_namespace_error_to_rename_event)?;

    let event_exists = project_metadata
        .events
        .as_ref()
        .is_some_and(|events| events.iter().any(|event| event == old_id));
    if !event_exists {
        return Err(RenameEventError::EventNotFound(old_id.to_string()));
    }

    let package_index = TaskPackageIndex::load(&project_root)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    ProjectNamespace::new(&project_metadata, package_index.loaded_tasks())
        .validate_renamed_event_id(new_id, old_id)
        .map_err(map_namespace_error_to_rename_event)?;

    let rewrite_plan = crate::mutation_plan::plan_project_metadata_rewrites(
        &project_root,
        |project_metadata| {
            if project_metadata.boundary_events.is_none() {
                project_metadata.boundary_events = project_metadata.resolved_boundary_events();
            }
            let existing_event = project_metadata
                .events
                .as_mut()
                .and_then(|events| events.iter_mut().find(|event| event.as_str() == old_id))
                .expect("existing event should still be present after validation");
            *existing_event = new_id.to_string();
            if let Some(boundary_events) = project_metadata.boundary_events.as_mut() {
                if boundary_events.start == old_id {
                    boundary_events.start = new_id.to_string();
                }
                if boundary_events.finish == old_id {
                    boundary_events.finish = new_id.to_string();
                }
            }
        },
        |_, task_metadata| {
            for dependency in &mut task_metadata.dependencies {
                if dependency == old_id {
                    *dependency = new_id.to_string();
                }
            }
            if task_metadata.ends_at.as_deref() == Some(old_id) {
                task_metadata.ends_at = Some(new_id.to_string());
            }
        },
    )
    .map_err(map_project_metadata_rewrite_plan_error)?;
    commit_event_metadata_rewrites(rewrite_plan_rewrites(&rewrite_plan))
        .map_err(RenameEventError::WriteMetadata)?;

    Ok(RenameEventOutcome::Renamed {
        old_id: old_id.to_string(),
        new_id: new_id.to_string(),
    })
}

#[derive(Debug)]
pub enum CreateEventError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    InvalidEventId(MetadataError),
    EventIdCollidesWithProjectTask { id: String },
    EventIdCollidesWithProjectEvent { id: String },
    ScanTasks(task::ScanTasksError),
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteMetadataFileError),
}

impl std::fmt::Display for CreateEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot create event from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::InvalidEventId(source) => write!(formatter, "{source}"),
            Self::EventIdCollidesWithProjectTask { id } => {
                write!(
                    formatter,
                    "event id '{id}' collides with project task id '{id}'"
                )
            }
            Self::EventIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "event id '{id}' collides with project event id '{id}'"
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::SerializeMetadata(source) => write!(formatter, "{source}"),
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CreateEventError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for CreateEventError {
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
}

#[derive(Debug)]
pub enum DeleteEventError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    EventNotFound(String),
    BoundaryEventReferenced(String),
    EventReferenced {
        id: String,
        task_ids: Vec<String>,
    },
    ScanTasks(task::ScanTasksError),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteMetadataFileError),
}

impl std::fmt::Display for DeleteEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot delete event from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::EventNotFound(id) => write!(formatter, "event id '{id}' was not found"),
            Self::BoundaryEventReferenced(id) => write!(
                formatter,
                "cannot delete event '{id}': referenced by project boundary events"
            ),
            Self::EventReferenced { id, task_ids } => {
                let task_list = task_ids
                    .iter()
                    .map(|task_id| format!("'{task_id}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                if task_ids.len() == 1 {
                    write!(
                        formatter,
                        "cannot delete event '{id}': referenced by task {task_list}"
                    )
                } else {
                    write!(
                        formatter,
                        "cannot delete event '{id}': referenced by tasks {task_list}"
                    )
                }
            }
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
            Self::SerializeMetadata(source) => write!(formatter, "{source}"),
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for DeleteEventError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for DeleteEventError {
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
}

#[derive(Debug)]
pub enum RenameEventError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    InvalidEventId(MetadataError),
    EventIdCollidesWithProjectTask {
        id: String,
    },
    EventIdCollidesWithProjectEvent {
        id: String,
    },
    ScanTasks(task::ScanTasksError),
    EventNotFound(String),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
    SerializeMetadata(MetadataError),
    WriteMetadata(WriteMetadataFileError),
}

impl std::fmt::Display for RenameEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot rename event from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::InvalidEventId(source) => write!(formatter, "{source}"),
            Self::EventIdCollidesWithProjectTask { id } => {
                write!(
                    formatter,
                    "event id '{id}' collides with project task id '{id}'"
                )
            }
            Self::EventIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "event id '{id}' collides with project event id '{id}'"
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::EventNotFound(id) => write!(formatter, "event id '{id}' was not found"),
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
            Self::SerializeMetadata(source) => write!(formatter, "{source}"),
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for RenameEventError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for RenameEventError {
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
}

type WriteMetadataFileError = AtomicJsonMetadataWriteError;

fn map_namespace_error_to_create_event(error: ProjectNamespaceError) -> CreateEventError {
    match error {
        ProjectNamespaceError::InvalidEventId(source) => CreateEventError::InvalidEventId(source),
        ProjectNamespaceError::EventIdCollidesWithProjectTask { id } => {
            CreateEventError::EventIdCollidesWithProjectTask { id }
        }
        ProjectNamespaceError::EventIdCollidesWithProjectEvent { id } => {
            CreateEventError::EventIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::InvalidTaskId(_)
        | ProjectNamespaceError::TaskIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::TaskIdCollidesWithProjectEvent { .. } => {
            unreachable!("event namespace validation should only return event namespace errors")
        }
    }
}

fn map_namespace_error_to_rename_event(error: ProjectNamespaceError) -> RenameEventError {
    match error {
        ProjectNamespaceError::InvalidEventId(source) => RenameEventError::InvalidEventId(source),
        ProjectNamespaceError::EventIdCollidesWithProjectTask { id } => {
            RenameEventError::EventIdCollidesWithProjectTask { id }
        }
        ProjectNamespaceError::EventIdCollidesWithProjectEvent { id } => {
            RenameEventError::EventIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::InvalidTaskId(_)
        | ProjectNamespaceError::TaskIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::TaskIdCollidesWithProjectEvent { .. } => {
            unreachable!("event namespace validation should only return event namespace errors")
        }
    }
}

fn event_deletion_blocking_task_ids(package_index: &TaskPackageIndex, id: &str) -> Vec<String> {
    let relationships = dependency_relationships::from_graph(package_index.graph());
    let Some(event_relationships) = relationships.events().iter().find(|event| event.id() == id)
    else {
        return Vec::new();
    };

    let mut task_ids = Vec::new();
    for reference in event_relationships.deletion_blockers() {
        if !task_ids
            .iter()
            .any(|task_id| task_id == reference.task_id())
        {
            task_ids.push(reference.task_id().to_string());
        }
    }
    task_ids
}

fn boundary_events_reference(project_metadata: &ProjectMetadata, id: &str) -> bool {
    project_metadata
        .resolved_boundary_events()
        .is_some_and(|boundary_events| boundary_events.start == id || boundary_events.finish == id)
}

fn rewrite_plan_rewrites(
    rewrite_plan: &crate::mutation_plan::ProjectMetadataRewritePlan,
) -> Vec<MetadataRewrite> {
    let mut rewrites = Vec::with_capacity(1 + rewrite_plan.task_rewrites().len());
    rewrites.push(rewrite_plan.project_rewrite().clone());
    rewrites.extend(rewrite_plan.task_rewrites().iter().cloned());
    rewrites
}

fn commit_event_metadata_rewrites(
    rewrites: impl IntoIterator<Item = MetadataRewrite>,
) -> Result<(), WriteMetadataFileError> {
    crate::package_mutation::PackageMutation::metadata_rewrites(rewrites)
        .commit()
        .map(|_| ())
        .map_err(crate::package_mutation::MetadataCommitError::into_source)
}

fn map_project_metadata_rewrite_plan_create_error(
    error: crate::mutation_plan::PlanProjectMetadataRewritesError,
) -> CreateEventError {
    match error {
        crate::mutation_plan::PlanProjectMetadataRewritesError::ProjectNotFound(path) => {
            CreateEventError::ProjectNotFound(path)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::ReadProjectMetadata(source) => {
            CreateEventError::ReadProjectMetadata(source)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::LoadProjectGraph(source) => {
            CreateEventError::ScanTasks(project_graph::map_load_error(source))
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::SerializeMetadata(source) => {
            CreateEventError::SerializeMetadata(source)
        }
    }
}

fn map_project_metadata_rewrite_plan_delete_error(
    error: crate::mutation_plan::PlanProjectMetadataRewritesError,
) -> DeleteEventError {
    match error {
        crate::mutation_plan::PlanProjectMetadataRewritesError::ProjectNotFound(path) => {
            DeleteEventError::ProjectNotFound(path)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::ReadProjectMetadata(source) => {
            DeleteEventError::ReadProjectMetadata(source)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::LoadProjectGraph(source) => {
            DeleteEventError::ScanTasks(project_graph::map_load_error(source))
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::SerializeMetadata(source) => {
            DeleteEventError::SerializeMetadata(source)
        }
    }
}

fn map_project_metadata_rewrite_plan_error(
    error: crate::mutation_plan::PlanProjectMetadataRewritesError,
) -> RenameEventError {
    match error {
        crate::mutation_plan::PlanProjectMetadataRewritesError::ProjectNotFound(path) => {
            RenameEventError::ProjectNotFound(path)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::ReadProjectMetadata(source) => {
            RenameEventError::ReadProjectMetadata(source)
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::LoadProjectGraph(source) => {
            RenameEventError::ScanTasks(project_graph::map_load_error(source))
        }
        crate::mutation_plan::PlanProjectMetadataRewritesError::SerializeMetadata(source) => {
            RenameEventError::SerializeMetadata(source)
        }
    }
}
