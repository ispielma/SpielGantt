use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    app_mutation_refresh, event,
    mutation_plan::{self, MutationPlanOperation},
    project,
    project_payload::{open_project, OpenProjectError, OpenProjectResult},
    task,
    task_edit_action::{apply_task_edit, TaskEditError},
};

pub use crate::task_edit_action::TaskEdit;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskActionResult {
    project: OpenProjectResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNormalizationResult {
    project: OpenProjectResult,
    renames: Vec<TaskRenamePayload>,
    issues: Vec<String>,
    applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFolderAlignmentResult {
    project: OpenProjectResult,
    renames: Vec<TaskRenamePayload>,
    issues: Vec<String>,
    applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRenamePayload {
    id: String,
    from: PathBuf,
    to: PathBuf,
}

impl TaskActionResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }
}

impl TaskNormalizationResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }

    pub fn renames(&self) -> &[TaskRenamePayload] {
        &self.renames
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }

    pub fn applied(&self) -> bool {
        self.applied
    }
}

impl TaskFolderAlignmentResult {
    pub fn project(&self) -> &OpenProjectResult {
        &self.project
    }

    pub fn renames(&self) -> &[TaskRenamePayload] {
        &self.renames
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }

    pub fn applied(&self) -> bool {
        self.applied
    }
}

impl TaskRenamePayload {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn from(&self) -> &Path {
        &self.from
    }

    pub fn to(&self) -> &Path {
        &self.to
    }
}

pub fn create_task(project_path: &Path, id: &str) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::create(project_path, id),
        TaskActionError::CreateTask,
        task_action_result,
    )
}

pub fn create_event(project_path: &Path, id: &str) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || event::create(project_path, id),
        TaskActionError::CreateEvent,
        task_action_result,
    )
}

pub fn delete_event(
    project_path: &Path,
    event_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || event::delete(project_path, event_id),
        TaskActionError::DeleteEvent,
        task_action_result,
    )
}

pub fn delete_task(
    project_path: &Path,
    task_id: &str,
    mode: task::DeleteTaskMode,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::delete(project_path, task_id, mode),
        TaskActionError::DeleteTask,
        task_action_result,
    )
}

pub fn adopt_task(
    project_path: &Path,
    folder_path: &Path,
    id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    ensure_folder_belongs_to_open_project(project_path, folder_path)?;
    mutate_then_refresh(
        project_path,
        || task::adopt(folder_path, id),
        TaskActionError::AdoptTask,
        task_action_result,
    )
}

pub fn list_adoptable_task_folders(
    project_path: &Path,
) -> Result<Vec<task::AdoptableTaskFolder>, task::ListAdoptableTaskFoldersError> {
    task::list_adoptable_task_folders(project_path)
}

pub fn preview_task_normalization(
    project_path: &Path,
) -> Result<TaskNormalizationResult, TaskActionError> {
    let plan = mutation_plan::plan_task_folder_normalization(project_path)
        .map_err(TaskActionError::PlanNormalization)?;
    let project = open_project(project_path).map_err(TaskActionError::RefreshProject)?;
    Ok(TaskNormalizationResult {
        project,
        renames: task_rename_payloads_from_operations(plan.operations()),
        issues: plan
            .preflight_issues()
            .iter()
            .map(|issue| issue.message())
            .collect(),
        applied: false,
    })
}

pub fn preview_task_folder_alignment(
    project_path: &Path,
) -> Result<mutation_plan::TaskFolderAlignmentPlan, mutation_plan::PlanTaskFolderAlignmentError> {
    mutation_plan::plan_task_folder_alignment(project_path)
}

pub fn apply_task_folder_alignment(
    project_path: &Path,
    plan: &mutation_plan::TaskFolderAlignmentPlan,
) -> Result<TaskFolderAlignmentResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || mutation_plan::apply_task_folder_alignment(project_path, plan),
        TaskActionError::ApplyTaskFolderAlignment,
        |project, renames| TaskFolderAlignmentResult {
            project,
            renames: task_rename_payloads_from_alignment_operations(&renames),
            issues: Vec::new(),
            applied: true,
        },
    )
}

pub fn apply_task_normalization(
    project_path: &Path,
) -> Result<TaskNormalizationResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::apply_normalization(project_path),
        TaskActionError::ApplyNormalization,
        |project, renames| TaskNormalizationResult {
            project,
            renames: renames
                .into_iter()
                .map(|rename| TaskRenamePayload {
                    id: rename.id().to_string(),
                    from: rename.from().to_path_buf(),
                    to: rename.to().to_path_buf(),
                })
                .collect(),
            issues: Vec::new(),
            applied: true,
        },
    )
}

pub fn add_task_dependency(
    project_path: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::add_dependency(project_path, task_id, blocker_id),
        TaskActionError::AddDependency,
        task_action_result,
    )
}

pub fn insert_task_before(
    project_path: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::insert_before(project_path, selected_task_id, inserted_task_id),
        TaskActionError::InsertRelativeTask,
        task_action_result,
    )
}

pub fn insert_task_after(
    project_path: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::insert_after(project_path, selected_task_id, inserted_task_id),
        TaskActionError::InsertRelativeTask,
        task_action_result,
    )
}

pub fn rename_task(
    project_path: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::rename(project_path, old_id, new_id),
        TaskActionError::RenameTask,
        task_action_result,
    )
}

pub fn rename_event(
    project_path: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || event::rename(project_path, old_id, new_id),
        TaskActionError::RenameEvent,
        task_action_result,
    )
}

pub fn remove_task_dependency(
    project_path: &Path,
    task_id: &str,
    blocker_id: &str,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::remove_dependency(project_path, task_id, blocker_id),
        TaskActionError::RemoveDependency,
        task_action_result,
    )
}

pub fn set_task_ends_at(
    project_path: &Path,
    task_id: &str,
    event_id: Option<&str>,
    clear: bool,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || task::set_ends_at(project_path, task_id, event_id, clear),
        TaskActionError::SetEndsAt,
        task_action_result,
    )
}

pub fn edit_task(
    project_path: &Path,
    task_id: &str,
    edit: TaskEdit,
) -> Result<TaskActionResult, TaskActionError> {
    mutate_then_refresh(
        project_path,
        || apply_task_edit(project_path, task_id, edit),
        TaskActionError::EditTask,
        task_action_result,
    )
}

fn mutate_then_refresh<Mutation, MutationOutput, MutationError, ActionResult>(
    project_path: &Path,
    mutate: Mutation,
    map_mutation_error: impl FnOnce(MutationError) -> TaskActionError,
    build_result: impl FnOnce(OpenProjectResult, MutationOutput) -> ActionResult,
) -> Result<ActionResult, TaskActionError>
where
    Mutation: FnOnce() -> Result<MutationOutput, MutationError>,
{
    app_mutation_refresh::mutate_then_refresh(
        project_path,
        mutate,
        map_mutation_error,
        TaskActionError::RefreshProject,
        build_result,
    )
}

fn task_action_result<T>(project: OpenProjectResult, _mutation_output: T) -> TaskActionResult {
    TaskActionResult { project }
}

fn task_rename_payloads_from_operations(
    operations: &[MutationPlanOperation],
) -> Vec<TaskRenamePayload> {
    operations
        .iter()
        .map(|operation| match operation {
            MutationPlanOperation::RenameTaskFolder { task_id, from, to } => TaskRenamePayload {
                id: task_id.clone(),
                from: from.clone(),
                to: to.clone(),
            },
        })
        .collect()
}

fn task_rename_payloads_from_alignment_operations(
    operations: &[mutation_plan::TaskFolderAlignmentOperation],
) -> Vec<TaskRenamePayload> {
    operations
        .iter()
        .map(|operation| match operation {
            mutation_plan::TaskFolderAlignmentOperation::RenameTaskFolder { task_id, from, to } => {
                TaskRenamePayload {
                    id: task_id.clone(),
                    from: from.clone(),
                    to: to.clone(),
                }
            }
        })
        .collect()
}

fn ensure_folder_belongs_to_open_project(
    project_path: &Path,
    folder_path: &Path,
) -> Result<(), TaskActionError> {
    let open_project_root = project::find_root(project_path).ok_or_else(|| {
        TaskActionError::InvalidInput(format!(
            "cannot adopt folder '{}': open project '{}' is not a SpielGantt project",
            folder_path.display(),
            project_path.display()
        ))
    })?;
    let folder_project_root = project::find_root(folder_path).ok_or_else(|| {
        TaskActionError::InvalidInput(format!(
            "cannot adopt folder '{}': folder is not inside the open project '{}'",
            folder_path.display(),
            open_project_root.display()
        ))
    })?;
    let open_project_root = std::fs::canonicalize(&open_project_root).map_err(|source| {
        TaskActionError::InvalidInput(format!(
            "cannot resolve open project '{}': {source}",
            open_project_root.display()
        ))
    })?;
    let folder_project_root = std::fs::canonicalize(&folder_project_root).map_err(|source| {
        TaskActionError::InvalidInput(format!(
            "cannot resolve folder project '{}': {source}",
            folder_project_root.display()
        ))
    })?;

    if folder_project_root != open_project_root {
        return Err(TaskActionError::InvalidInput(format!(
            "cannot adopt folder '{}': folder is not inside the open project '{}'",
            folder_path.display(),
            open_project_root.display()
        )));
    }

    Ok(())
}

#[derive(Debug)]
pub enum TaskActionError {
    AdoptTask(task::AdoptTaskError),
    AddDependency(task::AddDependencyError),
    CreateEvent(event::CreateEventError),
    DeleteEvent(event::DeleteEventError),
    DeleteTask(task::DeleteTaskError),
    EditTask(TaskEditError),
    RenameTask(task::RenameTaskError),
    RenameEvent(event::RenameEventError),
    ApplyNormalization(task::NormalizeTasksError),
    ApplyTaskFolderAlignment(mutation_plan::ApplyTaskFolderAlignmentError),
    CreateTask(task::CreateTaskError),
    InsertRelativeTask(task::InsertRelativeTaskError),
    InvalidInput(String),
    PlanNormalization(mutation_plan::PlanMutationError),
    RefreshProject(OpenProjectError),
    RemoveDependency(task::RemoveDependencyError),
    SetEndsAt(task::SetEndsAtError),
}

impl std::fmt::Display for TaskActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdoptTask(source) => write!(formatter, "{source}"),
            Self::AddDependency(source) => write!(formatter, "{source}"),
            Self::CreateEvent(source) => write!(formatter, "{source}"),
            Self::DeleteEvent(source) => write!(formatter, "{source}"),
            Self::DeleteTask(source) => write!(formatter, "{source}"),
            Self::EditTask(source) => write!(formatter, "{source}"),
            Self::RenameTask(source) => write!(formatter, "{source}"),
            Self::RenameEvent(source) => write!(formatter, "{source}"),
            Self::ApplyNormalization(source) => write!(formatter, "{source}"),
            Self::ApplyTaskFolderAlignment(source) => write!(formatter, "{source}"),
            Self::CreateTask(source) => write!(formatter, "{source}"),
            Self::InsertRelativeTask(source) => write!(formatter, "{source}"),
            Self::InvalidInput(message) => write!(formatter, "{message}"),
            Self::PlanNormalization(source) => write!(formatter, "{source}"),
            Self::RefreshProject(source) => write!(formatter, "{source}"),
            Self::RemoveDependency(source) => write!(formatter, "{source}"),
            Self::SetEndsAt(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for TaskActionError {}
