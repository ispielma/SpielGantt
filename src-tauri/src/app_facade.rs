use std::path::Path;

use serde::Serialize;

use crate::{dependency_relationships, mutation_plan, project_actions, project_lifecycle, task};

pub use crate::project_actions::{ProjectActionResult, ProjectReadmeEdit, ProjectReadmeEditError};
pub use crate::project_lifecycle::{
    OnboardProjectError, PrepareAgentScaffoldingError, PrepareTaskBucketsError,
    PrepareTaskBucketsWriteError, ProjectAgentPrepareResult, ProjectRefreshResult,
};
pub use crate::project_payload::{
    OpenProjectError, OpenProjectEventReferences, OpenProjectResult, OpenProjectTask,
    ReadProjectReadmeError, ReadTaskReadmeError,
};
pub use crate::task_actions::{
    TaskActionError, TaskActionResult, TaskEdit, TaskFolderAlignmentResult,
    TaskNormalizationResult, TaskRenamePayload,
};

pub const APP_NAME: &str = "spielgantt";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendHealth {
    app_name: &'static str,
    version: &'static str,
    core: &'static str,
}

#[derive(Debug)]
pub enum CreateProjectError {
    CreateProject(project_lifecycle::CreateProjectError),
}

impl std::fmt::Display for CreateProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateProject(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for CreateProjectError {}

pub fn app_name() -> &'static str {
    APP_NAME
}

pub fn backend_health() -> BackendHealth {
    BackendHealth {
        app_name: app_name(),
        version: env!("CARGO_PKG_VERSION"),
        core: "ready",
    }
}

pub fn open_project(path: &Path) -> Result<OpenProjectResult, OpenProjectError> {
    crate::project_payload::open_project(path)
}

pub fn refresh_project(path: &Path) -> Result<ProjectRefreshResult, OpenProjectError> {
    project_lifecycle::refresh_project(path)
}

pub fn edit_project_readme(
    project_path: &Path,
    edit: ProjectReadmeEdit,
) -> Result<ProjectActionResult, ProjectReadmeEditError> {
    project_actions::edit_project_readme(project_path, edit)
}

pub fn dependency_relationships(
    path: &Path,
) -> Result<
    dependency_relationships::DependencyRelationships,
    dependency_relationships::LoadDependencyRelationshipsError,
> {
    dependency_relationships::load(path)
}

pub fn onboard_project(path: &Path) -> Result<OpenProjectResult, OnboardProjectError> {
    project_lifecycle::onboard_project(path)
}

pub fn create_project_in_parent(
    project_name: &str,
    parent_destination: &Path,
) -> Result<OpenProjectResult, CreateProjectError> {
    project_lifecycle::create_project_in_parent(project_name, parent_destination)
        .map_err(CreateProjectError::CreateProject)
}

pub fn prepare_agent_scaffolding(
    path: &Path,
) -> Result<ProjectAgentPrepareResult, PrepareAgentScaffoldingError> {
    project_lifecycle::prepare_agent_scaffolding(path)
}

pub fn create_task(project_path: &Path, id: &str) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::create_task(project_path, id)
}

pub fn insert_task_before(
    project_path: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::insert_task_before(project_path, selected_task_id, inserted_task_id)
}

pub fn insert_task_after(
    project_path: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::insert_task_after(project_path, selected_task_id, inserted_task_id)
}

pub fn create_event(project_path: &Path, id: &str) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::create_event(project_path, id)
}

pub fn delete_event(
    project_path: &Path,
    event_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::delete_event(project_path, event_id)
}

pub fn delete_task(
    project_path: &Path,
    task_id: &str,
    mode: task::DeleteTaskMode,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::delete_task(project_path, task_id, mode)
}

pub fn adopt_task(
    project_path: &Path,
    folder_path: &Path,
    id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::adopt_task(project_path, folder_path, id)
}

pub fn list_adoptable_task_folders(
    project_path: &Path,
) -> Result<Vec<task::AdoptableTaskFolder>, task::ListAdoptableTaskFoldersError> {
    crate::task_actions::list_adoptable_task_folders(project_path)
}

pub fn preview_task_normalization(
    project_path: &Path,
) -> Result<TaskNormalizationResult, TaskActionError> {
    crate::task_actions::preview_task_normalization(project_path)
}

pub fn preview_task_folder_alignment(
    project_path: &Path,
) -> Result<mutation_plan::TaskFolderAlignmentPlan, mutation_plan::PlanTaskFolderAlignmentError> {
    crate::task_actions::preview_task_folder_alignment(project_path)
}

pub fn apply_task_folder_alignment(
    project_path: &Path,
    plan: &mutation_plan::TaskFolderAlignmentPlan,
) -> Result<TaskFolderAlignmentResult, TaskActionError> {
    crate::task_actions::apply_task_folder_alignment(project_path, plan)
}

pub fn apply_task_normalization(
    project_path: &Path,
) -> Result<TaskNormalizationResult, TaskActionError> {
    crate::task_actions::apply_task_normalization(project_path)
}

pub fn edit_task(
    project_path: &Path,
    task_id: &str,
    edit: TaskEdit,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::edit_task(project_path, task_id, edit)
}

pub fn add_task_dependency(
    project_path: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::add_task_dependency(project_path, task_id, blocker_id)
}

pub fn rename_task(
    project_path: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::rename_task(project_path, old_id, new_id)
}

pub fn rename_event(
    project_path: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::rename_event(project_path, old_id, new_id)
}

pub fn remove_task_dependency(
    project_path: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::remove_task_dependency(project_path, task_id, blocker_id)
}

pub fn set_task_ends_at(
    project_path: &Path,
    task_id: &str,
    event_id: Option<&str>,
    clear: bool,
) -> Result<TaskActionResult, TaskActionError> {
    crate::task_actions::set_task_ends_at(project_path, task_id, event_id, clear)
}
