use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::{
    metadata::{MetadataError, ProjectMetadata, TaskMetadata},
    project, project_graph, task,
    task_package_index::TaskPackageIndex,
    MetadataRewrite,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationPlan {
    operations: Vec<MutationPlanOperation>,
    preflight_issues: Vec<MutationPreflightIssue>,
    normalization_renames: Vec<task::PlannedTaskRename>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MutationPlanOperation {
    RenameTaskFolder {
        task_id: String,
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MutationPreflightIssue {
    TargetAlreadyExists(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFolderAlignmentPlan {
    operations: Vec<TaskFolderAlignmentOperation>,
    preflight_issues: Vec<TaskFolderAlignmentPreflightIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskFolderAlignmentOperation {
    RenameTaskFolder {
        task_id: String,
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskFolderAlignmentPreflightIssue {
    TargetAlreadyExists(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMetadataRewritePlan {
    project_rewrite: MetadataRewrite,
    task_rewrites: Vec<MetadataRewrite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedTaskMetadata {
    id: String,
    path: PathBuf,
    metadata: TaskMetadata,
}

impl MutationPlan {
    pub fn operations(&self) -> &[MutationPlanOperation] {
        &self.operations
    }

    pub fn preflight_issues(&self) -> &[MutationPreflightIssue] {
        &self.preflight_issues
    }

    pub fn is_safe(&self) -> bool {
        self.preflight_issues.is_empty()
    }

    pub(crate) fn normalization_renames(&self) -> &[task::PlannedTaskRename] {
        &self.normalization_renames
    }
}

impl MutationPreflightIssue {
    pub fn message(&self) -> String {
        match self {
            Self::TargetAlreadyExists(path) => {
                format!("target folder '{}' already exists", path.display())
            }
        }
    }
}

impl TaskFolderAlignmentPlan {
    pub fn operations(&self) -> &[TaskFolderAlignmentOperation] {
        &self.operations
    }

    pub fn preflight_issues(&self) -> &[TaskFolderAlignmentPreflightIssue] {
        &self.preflight_issues
    }

    pub fn is_safe(&self) -> bool {
        self.preflight_issues.is_empty()
    }
}

impl TaskFolderAlignmentPreflightIssue {
    pub fn message(&self) -> String {
        match self {
            Self::TargetAlreadyExists(path) => {
                format!("target folder '{}' already exists", path.display())
            }
        }
    }
}

impl ProjectMetadataRewritePlan {
    pub fn project_rewrite(&self) -> &MetadataRewrite {
        &self.project_rewrite
    }

    pub fn task_rewrites(&self) -> &[MetadataRewrite] {
        &self.task_rewrites
    }

    pub fn all_rewrites(&self) -> Vec<&MetadataRewrite> {
        std::iter::once(&self.project_rewrite)
            .chain(self.task_rewrites.iter())
            .collect()
    }
}

impl LoadedTaskMetadata {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn metadata(&self) -> &TaskMetadata {
        &self.metadata
    }
}

pub fn plan_task_folder_normalization(start: &Path) -> Result<MutationPlan, PlanMutationError> {
    let renames =
        task::plan_normalization(start).map_err(PlanMutationError::NormalizeTaskFolders)?;
    let operations = renames
        .iter()
        .map(|rename| MutationPlanOperation::RenameTaskFolder {
            task_id: rename.id().to_string(),
            from: rename.from().to_path_buf(),
            to: rename.to().to_path_buf(),
        })
        .collect();
    let preflight_issues = renames
        .iter()
        .filter(|rename| rename.operation_to().exists())
        .map(|rename| {
            MutationPreflightIssue::TargetAlreadyExists(rename.operation_to().to_path_buf())
        })
        .collect();

    Ok(MutationPlan {
        operations,
        preflight_issues,
        normalization_renames: renames,
    })
}

pub fn plan_task_folder_alignment(
    start: &Path,
) -> Result<TaskFolderAlignmentPlan, PlanTaskFolderAlignmentError> {
    let project_root = crate::project::find_root(start)
        .ok_or_else(|| PlanTaskFolderAlignmentError::ProjectNotFound(start.to_path_buf()))?;
    let scanned_tasks =
        task::scan(&project_root).map_err(PlanTaskFolderAlignmentError::ScanTasks)?;
    let task_paths = scanned_tasks
        .iter()
        .map(|task| task.path().to_path_buf())
        .collect::<std::collections::HashSet<_>>();

    for task in &scanned_tasks {
        if let Some((nested_task_path, ancestor_task_path)) =
            nested_task_bucket(task.path(), &project_root, &task_paths)
        {
            return Err(PlanTaskFolderAlignmentError::NestedTaskBucket {
                task_path: nested_task_path,
                ancestor_path: ancestor_task_path,
            });
        }
        if task.path().parent() != Some(project_root.as_path()) {
            return Err(
                PlanTaskFolderAlignmentError::TaskBucketOutsideDirectProjectChildren {
                    task_path: task.path().to_path_buf(),
                    project_root: project_root.clone(),
                },
            );
        }
    }

    let mut operations = Vec::new();
    let mut preflight_issues = Vec::new();

    for task in &scanned_tasks {
        let Some(task_folder_name) = task.path().file_name() else {
            continue;
        };
        if task_folder_name == OsStr::new(task.id()) {
            continue;
        }

        let from = task.path().to_path_buf();
        let to = task
            .path()
            .parent()
            .unwrap_or(&project_root)
            .join(task.id());
        operations.push(TaskFolderAlignmentOperation::RenameTaskFolder {
            task_id: task.id().to_string(),
            from: from.clone(),
            to: to.clone(),
        });
        if crate::path_points_to_distinct_existing_entry(&to, &from) {
            preflight_issues.push(TaskFolderAlignmentPreflightIssue::TargetAlreadyExists(to));
        }
    }

    operations.sort_by(|left, right| match (left, right) {
        (
            TaskFolderAlignmentOperation::RenameTaskFolder { from: left, .. },
            TaskFolderAlignmentOperation::RenameTaskFolder { from: right, .. },
        ) => left.cmp(right),
    });

    Ok(TaskFolderAlignmentPlan {
        operations,
        preflight_issues,
    })
}

pub fn plan_project_metadata_rewrites<F, G>(
    start: &Path,
    edit_project_metadata: F,
    mut edit_task_metadata: G,
) -> Result<ProjectMetadataRewritePlan, PlanProjectMetadataRewritesError>
where
    F: FnOnce(&mut ProjectMetadata),
    G: FnMut(&LoadedTaskMetadata, &mut TaskMetadata),
{
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let project_root = package_index.project_root();
    let project_metadata_path = project_root.join(".spielgantt").join("project.json");
    let original_project_metadata_contents = std::fs::read_to_string(&project_metadata_path)
        .map_err(|source| {
            PlanProjectMetadataRewritesError::ReadProjectMetadata(
                project::ReadProjectMetadataError::ReadMetadata {
                    path: project_metadata_path.clone(),
                    source,
                },
            )
        })?;
    let mut project_metadata = package_index.project_metadata().clone();
    edit_project_metadata(&mut project_metadata);
    let updated_project_metadata_contents = project_metadata
        .to_json()
        .map_err(PlanProjectMetadataRewritesError::SerializeMetadata)?;

    let task_rewrites = package_index.plan_task_metadata_rewrites(
        |loaded_task, task_metadata| {
            let loaded_task_metadata = LoadedTaskMetadata {
                id: loaded_task.metadata.id.clone(),
                path: loaded_task.path.clone(),
                metadata: loaded_task.metadata.clone(),
            };
            edit_task_metadata(&loaded_task_metadata, task_metadata);
            Ok(None)
        },
        PlanProjectMetadataRewritesError::SerializeMetadata,
    )?;

    Ok(ProjectMetadataRewritePlan {
        project_rewrite: MetadataRewrite {
            path: project_metadata_path,
            original_contents: original_project_metadata_contents,
            updated_contents: updated_project_metadata_contents,
        },
        task_rewrites,
    })
}

pub fn apply_task_folder_alignment(
    start: &Path,
    plan: &TaskFolderAlignmentPlan,
) -> Result<Vec<TaskFolderAlignmentOperation>, ApplyTaskFolderAlignmentError> {
    let project_root = crate::project::find_root(start)
        .ok_or_else(|| ApplyTaskFolderAlignmentError::ProjectNotFound(start.to_path_buf()))?;
    let canonical_plan = plan_task_folder_alignment(&project_root)
        .map_err(ApplyTaskFolderAlignmentError::PlanTaskFolderAlignment)?;

    for issue in canonical_plan.preflight_issues() {
        match issue {
            TaskFolderAlignmentPreflightIssue::TargetAlreadyExists(path) => {
                return Err(ApplyTaskFolderAlignmentError::TargetAlreadyExists(
                    path.clone(),
                ));
            }
        }
    }

    if plan != &canonical_plan {
        return Err(ApplyTaskFolderAlignmentError::SubmittedPlanIsNotCanonical);
    }

    let mut operations_to_apply = canonical_plan.operations().to_vec();
    operations_to_apply.sort_by(|left, right| match (left, right) {
        (
            TaskFolderAlignmentOperation::RenameTaskFolder { from: left, .. },
            TaskFolderAlignmentOperation::RenameTaskFolder { from: right, .. },
        ) => {
            let left_depth = left.components().count();
            let right_depth = right.components().count();
            right_depth.cmp(&left_depth).then_with(|| left.cmp(right))
        }
    });

    for operation in &operations_to_apply {
        match operation {
            TaskFolderAlignmentOperation::RenameTaskFolder { from, to, .. }
                if crate::path_points_to_distinct_existing_entry(to, from) =>
            {
                return Err(ApplyTaskFolderAlignmentError::TargetAlreadyExists(
                    to.clone(),
                ));
            }
            TaskFolderAlignmentOperation::RenameTaskFolder { .. } => {}
        }
    }

    for operation in &operations_to_apply {
        match operation {
            TaskFolderAlignmentOperation::RenameTaskFolder { from, to, .. } => {
                std::fs::rename(from, to).map_err(|source| {
                    ApplyTaskFolderAlignmentError::RenameTaskFolder {
                        from: from.clone(),
                        to: to.clone(),
                        source,
                    }
                })?;
            }
        }
    }

    Ok(canonical_plan.operations().to_vec())
}

fn nested_task_bucket(
    task_path: &Path,
    project_root: &Path,
    task_paths: &std::collections::HashSet<PathBuf>,
) -> Option<(PathBuf, PathBuf)> {
    let mut ancestor = task_path.parent();

    while let Some(candidate) = ancestor {
        if candidate == project_root {
            break;
        }
        if task_paths.contains(candidate) {
            return Some((task_path.to_path_buf(), candidate.to_path_buf()));
        }
        ancestor = candidate.parent();
    }

    None
}

#[derive(Debug)]
pub enum PlanMutationError {
    NormalizeTaskFolders(task::NormalizeTasksError),
}

#[derive(Debug)]
pub enum PlanTaskFolderAlignmentError {
    ProjectNotFound(PathBuf),
    ScanTasks(task::ScanTasksError),
    NestedTaskBucket {
        task_path: PathBuf,
        ancestor_path: PathBuf,
    },
    TaskBucketOutsideDirectProjectChildren {
        task_path: PathBuf,
        project_root: PathBuf,
    },
}

#[derive(Debug)]
pub enum ApplyTaskFolderAlignmentError {
    ProjectNotFound(PathBuf),
    PlanTaskFolderAlignment(PlanTaskFolderAlignmentError),
    SubmittedPlanIsNotCanonical,
    TargetAlreadyExists(PathBuf),
    RenameTaskFolder {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum PlanProjectMetadataRewritesError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    LoadProjectGraph(project_graph::LoadProjectGraphError),
    SerializeMetadata(MetadataError),
}

impl std::fmt::Display for PlanMutationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NormalizeTaskFolders(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for PlanMutationError {}

impl std::fmt::Display for PlanTaskFolderAlignmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "SpielGantt project not found from '{}'",
                    path.display()
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::NestedTaskBucket {
                task_path,
                ancestor_path,
            } => write!(
                formatter,
                "nested task bucket '{}' is unsupported by the strict project model because '{}' is already a task bucket",
                task_path.display(),
                ancestor_path.display()
            ),
            Self::TaskBucketOutsideDirectProjectChildren {
                task_path,
                project_root,
            } => write!(
                formatter,
                "task bucket '{}' is unsupported by the strict project model because task buckets must be direct children of project '{}'",
                task_path.display(),
                project_root.display()
            ),
        }
    }
}

impl std::error::Error for PlanTaskFolderAlignmentError {}

impl std::fmt::Display for ApplyTaskFolderAlignmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot apply folder alignment: '{}' is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::PlanTaskFolderAlignment(source) => write!(formatter, "{source}"),
            Self::SubmittedPlanIsNotCanonical => write!(
                formatter,
                "submitted folder alignment plan does not match the canonical folder alignment plan; refresh preview and try again"
            ),
            Self::TargetAlreadyExists(path) => {
                write!(
                    formatter,
                    "target folder '{}' already exists",
                    path.display()
                )
            }
            Self::RenameTaskFolder { from, to, source } => {
                write!(
                    formatter,
                    "failed to rename task folder '{}' to '{}': {source}",
                    from.display(),
                    to.display()
                )
            }
        }
    }
}

impl std::error::Error for ApplyTaskFolderAlignmentError {}

impl std::fmt::Display for PlanProjectMetadataRewritesError {
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
            Self::LoadProjectGraph(source) => write!(formatter, "{source}"),
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize metadata: {source}")
            }
        }
    }
}

impl std::error::Error for PlanProjectMetadataRewritesError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for PlanProjectMetadataRewritesError {
    type ScanError = project_graph::LoadProjectGraphError;

    fn project_not_found(path: PathBuf) -> Self {
        Self::ProjectNotFound(path)
    }

    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self {
        Self::ReadProjectMetadata(source)
    }

    fn scan_error(source: Self::ScanError) -> Self {
        Self::LoadProjectGraph(source)
    }
}
