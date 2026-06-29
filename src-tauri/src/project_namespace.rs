use std::path::PathBuf;

use crate::{
    metadata::{
        validate_project_namespace_entry, MetadataError, ProjectMetadata, ProjectNamespaceKind,
    },
    project_graph::LoadedProjectGraphTask,
    task_package_index::TaskPackageIndex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistingTaskPolicy<'a> {
    RejectAll,
    AllowId(&'a str),
}

#[derive(Debug)]
pub(crate) enum ProjectNamespaceError {
    InvalidTaskId(MetadataError),
    InvalidEventId(MetadataError),
    TaskIdCollidesWithProjectTask { id: String, existing_path: PathBuf },
    TaskIdCollidesWithProjectEvent { id: String },
    EventIdCollidesWithProjectTask { id: String },
    EventIdCollidesWithProjectEvent { id: String },
}

pub(crate) struct ProjectNamespace<'a> {
    project_metadata: &'a ProjectMetadata,
    loaded_tasks: &'a [LoadedProjectGraphTask],
}

impl<'a> ProjectNamespace<'a> {
    pub(crate) fn from_package_index(package_index: &'a TaskPackageIndex) -> Self {
        Self::new(
            package_index.project_metadata(),
            package_index.loaded_tasks(),
        )
    }

    pub(crate) fn new(
        project_metadata: &'a ProjectMetadata,
        loaded_tasks: &'a [LoadedProjectGraphTask],
    ) -> Self {
        Self {
            project_metadata,
            loaded_tasks,
        }
    }

    pub(crate) fn validate_new_task_id(&self, id: &str) -> Result<(), ProjectNamespaceError> {
        self.validate_task_id(id, ExistingTaskPolicy::RejectAll)
    }

    pub(crate) fn validate_task_id(
        &self,
        id: &str,
        existing_task_policy: ExistingTaskPolicy<'_>,
    ) -> Result<(), ProjectNamespaceError> {
        validate_project_namespace_entry(
            id,
            ProjectNamespaceKind::Task,
            ProjectNamespaceKind::Event,
            self.project_metadata.events.as_deref().unwrap_or(&[]),
        )
        .map_err(map_task_namespace_error)?;

        if let Some(existing_task) = self.colliding_task(id, existing_task_policy) {
            return Err(ProjectNamespaceError::TaskIdCollidesWithProjectTask {
                id: id.to_string(),
                existing_path: existing_task.path.clone(),
            });
        }

        Ok(())
    }

    pub(crate) fn validate_new_event_id(&self, id: &str) -> Result<(), ProjectNamespaceError> {
        self.validate_event_id(id, None)
    }

    pub(crate) fn validate_renamed_event_id(
        &self,
        id: &str,
        current_event_id: &str,
    ) -> Result<(), ProjectNamespaceError> {
        self.validate_event_id(id, Some(current_event_id))
    }

    pub(crate) fn validate_event_id_format(id: &str) -> Result<(), ProjectNamespaceError> {
        validate_project_namespace_entry(
            id,
            ProjectNamespaceKind::Event,
            ProjectNamespaceKind::Task,
            &[],
        )
        .map_err(map_event_namespace_error)
    }

    fn validate_event_id(
        &self,
        id: &str,
        ignored_event_id: Option<&str>,
    ) -> Result<(), ProjectNamespaceError> {
        Self::validate_event_id_format(id)?;

        let task_ids = self
            .loaded_tasks
            .iter()
            .map(|task| task.metadata.id.clone())
            .collect::<Vec<_>>();
        validate_project_namespace_entry(
            id,
            ProjectNamespaceKind::Event,
            ProjectNamespaceKind::Task,
            &task_ids,
        )
        .map_err(map_event_namespace_error)?;

        let event_ids = self
            .project_metadata
            .events
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .filter(|event_id| Some(event_id.as_str()) != ignored_event_id)
            .cloned()
            .collect::<Vec<_>>();
        validate_project_namespace_entry(
            id,
            ProjectNamespaceKind::Event,
            ProjectNamespaceKind::Event,
            &event_ids,
        )
        .map_err(map_event_namespace_error)?;

        Ok(())
    }

    fn colliding_task(
        &self,
        id: &str,
        existing_task_policy: ExistingTaskPolicy<'_>,
    ) -> Option<&'a LoadedProjectGraphTask> {
        self.loaded_tasks.iter().find(|task| {
            task.metadata.id == id
                && !matches!(
                    existing_task_policy,
                    ExistingTaskPolicy::AllowId(allowed_id) if allowed_id == id
                )
        })
    }
}

fn map_task_namespace_error(error: MetadataError) -> ProjectNamespaceError {
    match error {
        MetadataError::InvalidTaskId(source) => {
            ProjectNamespaceError::InvalidTaskId(MetadataError::InvalidTaskId(source))
        }
        MetadataError::ProjectNamespaceCollision {
            id,
            existing_kind: ProjectNamespaceKind::Event,
            ..
        } => ProjectNamespaceError::TaskIdCollidesWithProjectEvent { id },
        other => ProjectNamespaceError::InvalidTaskId(other),
    }
}

fn map_event_namespace_error(error: MetadataError) -> ProjectNamespaceError {
    match error {
        MetadataError::InvalidProjectEventId(source) => {
            ProjectNamespaceError::InvalidEventId(MetadataError::InvalidProjectEventId(source))
        }
        MetadataError::ProjectNamespaceCollision {
            id,
            existing_kind: ProjectNamespaceKind::Task,
            ..
        } => ProjectNamespaceError::EventIdCollidesWithProjectTask { id },
        MetadataError::ProjectNamespaceCollision {
            id,
            existing_kind: ProjectNamespaceKind::Event,
            ..
        } => ProjectNamespaceError::EventIdCollidesWithProjectEvent { id },
        other => ProjectNamespaceError::InvalidEventId(other),
    }
}
