use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{
    metadata::{FolderNamingPolicy, MetadataError, TaskMetadata, TaskStatus},
    project, project_graph,
    project_namespace::{ExistingTaskPolicy, ProjectNamespace, ProjectNamespaceError},
    project_snapshot,
    task_package_index::TaskPackageIndex,
};

pub use crate::relative_task_insert::{
    CreateInsertedTaskFilesError, InsertRelativeTaskError, InsertRelativeTaskOutcome,
    RelativeInsertMode,
};
pub use crate::task_adoptable_folders::{
    list_adoptable_task_folders, AdoptableTaskFolder, ListAdoptableTaskFoldersError,
};
pub(crate) use crate::task_metadata_mutations::plan_update;
pub use crate::task_metadata_mutations::{
    add_dependency, remove_dependency, set_ends_at, update, AddDependencyError,
    AddDependencyOutcome, RemoveDependencyError, RemoveDependencyOutcome, SetEndsAtError,
    SetEndsAtOutcome, TaskUpdate, UpdateTaskError, UpdateTaskOutcome,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptTaskOutcome {
    Created,
    AlreadyAdopted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateTaskOutcome {
    Created { task_path: std::path::PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadmeTaskOutcome {
    Created { path: PathBuf },
    Existing { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameTaskOutcome {
    Renamed {
        old_id: String,
        new_id: String,
        old_path: PathBuf,
        new_path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeleteTaskMode {
    RemoveFromChart,
    DeleteDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteTaskOutcome {
    RemovedFromChart {
        task_id: String,
        task_path: PathBuf,
        cleanup_failure: Option<StagedCleanupFailure>,
    },
    DeletedDirectory {
        task_id: String,
        task_path: PathBuf,
        cleanup_failure: Option<StagedCleanupFailure>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedCleanupFailure {
    path: PathBuf,
    error: String,
}

impl StagedCleanupFailure {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListEntry {
    id: String,
    status: Option<TaskStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedTask {
    id: String,
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PlannedTaskRename {
    id: String,
    from: PathBuf,
    to: PathBuf,
    operation_to: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDetails {
    id: String,
    path: PathBuf,
    dependencies: Vec<String>,
    status: Option<TaskStatus>,
}

impl ScannedTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TaskListEntry {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }
}

impl PlannedTaskRename {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn from(&self) -> &Path {
        &self.from
    }

    pub fn to(&self) -> &Path {
        &self.to
    }

    pub(crate) fn operation_to(&self) -> &Path {
        &self.operation_to
    }
}

impl TaskDetails {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }
}

pub fn adopt(target: &Path, id: &str) -> Result<AdoptTaskOutcome, AdoptTaskError> {
    let target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(AdoptTaskError::CurrentDirectory)?
            .join(target)
    };
    let target =
        std::fs::canonicalize(&target).map_err(|source| AdoptTaskError::InvalidTargetPath {
            path: target.clone(),
            source,
        })?;
    let target_metadata =
        std::fs::metadata(&target).map_err(|source| AdoptTaskError::InvalidTargetPath {
            path: target.clone(),
            source,
        })?;
    if !target_metadata.is_dir() {
        return Err(AdoptTaskError::TargetIsNotDirectory(target));
    }

    let project_root = project::find_root(&target)
        .ok_or_else(|| AdoptTaskError::ProjectNotFound(target.clone()))?;
    let task_metadata = TaskMetadata::version_1(id).map_err(AdoptTaskError::InvalidTaskMetadata)?;
    if let Some(existing_task_metadata) = read_task_metadata(&target)? {
        if existing_task_metadata.id == task_metadata.id {
            return Ok(AdoptTaskOutcome::AlreadyAdopted);
        }

        return Err(AdoptTaskError::AlreadyAdoptedWithDifferentId {
            path: target,
            existing_id: existing_task_metadata.id,
            requested_id: task_metadata.id,
        });
    }

    let package_index =
        TaskPackageIndex::load(&project_root).map_err(project_graph::map_load_error)?;
    ProjectNamespace::from_package_index(&package_index)
        .validate_new_task_id(&task_metadata.id)
        .map_err(map_namespace_error_to_adopt_task)?;

    let spielgantt_dir = target.join(".spielgantt");
    std::fs::create_dir_all(&spielgantt_dir).map_err(|source| AdoptTaskError::CreateDir {
        path: spielgantt_dir.clone(),
        source,
    })?;

    let task_metadata_path = spielgantt_dir.join("task.json");
    commit_new_task_metadata(
        &task_metadata_path,
        task_metadata
            .to_json()
            .map_err(AdoptTaskError::InvalidTaskMetadata)?,
    )
    .map_err(|source| AdoptTaskError::WriteMetadata {
        path: task_metadata_path,
        source,
    })?;

    Ok(AdoptTaskOutcome::Created)
}

pub fn create(start: &Path, id: &str) -> Result<CreateTaskOutcome, CreateTaskError> {
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let project_root = package_index.project_root().to_path_buf();
    let project_metadata = package_index.project_metadata();
    let task_metadata = TaskMetadata::version_1(id).map_err(CreateTaskError::InvalidTaskId)?;

    ProjectNamespace::from_package_index(&package_index)
        .validate_new_task_id(&task_metadata.id)
        .map_err(map_namespace_error_to_create_task)?;

    let folder_name = match project_metadata.folder_naming {
        FolderNamingPolicy::TaskId => task_metadata.id.clone(),
    };
    let task_path = project_root.join(folder_name);

    if task_path.exists() {
        return Err(CreateTaskError::FolderAlreadyExists(task_path));
    }

    create_prepared_task_bucket(&task_path, &task_metadata)
        .map_err(create_task_error_from_bucket)?;

    Ok(CreateTaskOutcome::Created { task_path })
}

pub fn insert_before(
    start: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<InsertRelativeTaskOutcome, InsertRelativeTaskError> {
    crate::relative_task_insert::insert_before(start, selected_task_id, inserted_task_id)
}

pub fn insert_after(
    start: &Path,
    selected_task_id: &str,
    inserted_task_id: &str,
) -> Result<InsertRelativeTaskOutcome, InsertRelativeTaskError> {
    crate::relative_task_insert::insert_after(start, selected_task_id, inserted_task_id)
}

pub fn readme(start: &Path, task_id: &str) -> Result<ReadmeTaskOutcome, ReadmeTaskError> {
    let task_path = resolve_task_path(start, task_id).map_err(ReadmeTaskError::ResolveTask)?;
    let readme_path = task_path.join("README.md");

    if readme_path.is_file() {
        return Ok(ReadmeTaskOutcome::Existing { path: readme_path });
    }

    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&readme_path)
        .map_err(|source| ReadmeTaskError::CreateReadme {
            path: readme_path.clone(),
            source,
        })?;

    Ok(ReadmeTaskOutcome::Created { path: readme_path })
}

pub fn scan(project_root: &Path) -> Result<Vec<ScannedTask>, ScanTasksError> {
    let loaded_tasks = read_loaded_task_metadata(project_root)?;
    Ok(scanned_tasks_from_loaded(&loaded_tasks))
}

pub fn plan_normalization(start: &Path) -> Result<Vec<PlannedTaskRename>, NormalizeTasksError> {
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let project_root = package_index.project_root().to_path_buf();
    let project_metadata = package_index.project_metadata();
    let scanned_tasks = scanned_tasks_from_loaded(package_index.loaded_tasks());
    let task_ids_by_path = scanned_tasks
        .iter()
        .map(|task| (task.path.clone(), task.id.clone()))
        .collect::<HashMap<_, _>>();
    let mut final_paths_by_path = HashMap::<PathBuf, PathBuf>::new();

    let mut renames = scanned_tasks
        .iter()
        .filter_map(|task| {
            let normalized_folder_name =
                normalized_task_folder_name(&project_metadata.folder_naming, &task.id);
            let current_folder_name = task.path.file_name()?;
            (current_folder_name != OsStr::new(&normalized_folder_name)).then(|| {
                let final_to = final_normalized_task_path(
                    &task.path,
                    &task.id,
                    &project_root,
                    &project_metadata.folder_naming,
                    &task_ids_by_path,
                    &mut final_paths_by_path,
                );
                PlannedTaskRename {
                    id: task.id.clone(),
                    from: task.path.clone(),
                    to: final_to,
                    operation_to: task
                        .path
                        .parent()
                        .unwrap_or(&project_root)
                        .join(normalized_folder_name),
                }
            })
        })
        .collect::<Vec<_>>();

    renames.sort_by(|left, right| left.from.cmp(&right.from));
    Ok(renames)
}

pub fn apply_normalization(start: &Path) -> Result<Vec<PlannedTaskRename>, NormalizeTasksError> {
    let plan =
        crate::mutation_plan::plan_task_folder_normalization(start).map_err(
            |error| match error {
                crate::mutation_plan::PlanMutationError::NormalizeTaskFolders(source) => source,
            },
        )?;
    for issue in plan.preflight_issues() {
        match issue {
            crate::mutation_plan::MutationPreflightIssue::TargetAlreadyExists(path) => {
                return Err(NormalizeTasksError::TargetAlreadyExists(path.clone()));
            }
        }
    }

    let renames = plan.normalization_renames().to_vec();
    let mut renames_to_apply = renames.clone();
    renames_to_apply.sort_by(|left, right| {
        let left_depth = left.from.components().count();
        let right_depth = right.from.components().count();
        right_depth
            .cmp(&left_depth)
            .then_with(|| left.from.cmp(&right.from))
    });

    for rename in &renames_to_apply {
        std::fs::rename(&rename.from, &rename.operation_to).map_err(|source| {
            NormalizeTasksError::RenameTaskFolder {
                from: rename.from.clone(),
                to: rename.operation_to.clone(),
                source,
            }
        })?;
    }

    Ok(renames)
}

pub fn rename(
    start: &Path,
    old_id: &str,
    new_id: &str,
) -> Result<RenameTaskOutcome, RenameTaskError> {
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let project_root = package_index.project_root().to_path_buf();
    let project_metadata = package_index.project_metadata();
    let new_task_metadata =
        TaskMetadata::version_1(new_id).map_err(RenameTaskError::InvalidTaskId)?;
    let loaded_tasks = package_index.loaded_tasks();
    let scanned_tasks = scanned_tasks_from_loaded(loaded_tasks);

    let task_to_rename = scanned_tasks
        .iter()
        .find(|task| task.id == old_id)
        .ok_or_else(|| RenameTaskError::TaskNotFound(old_id.to_string()))?;

    ProjectNamespace::from_package_index(&package_index)
        .validate_task_id(&new_task_metadata.id, ExistingTaskPolicy::AllowId(old_id))
        .map_err(map_namespace_error_to_rename_task)?;

    let normalized_folder_name =
        normalized_task_folder_name(&project_metadata.folder_naming, &new_task_metadata.id);
    let new_task_path = task_to_rename
        .path
        .parent()
        .unwrap_or(&project_root)
        .join(normalized_folder_name);
    let should_rename_folder = new_task_path != task_to_rename.path;
    if should_rename_folder
        && crate::path_points_to_distinct_existing_entry(&new_task_path, &task_to_rename.path)
    {
        return Err(RenameTaskError::TargetAlreadyExists(new_task_path));
    }

    let metadata_updates = crate::plan_loaded_task_metadata_rewrites(
        loaded_tasks,
        |loaded_task, task_metadata| {
            if task_metadata.id == old_id {
                task_metadata.id = new_task_metadata.id.clone();
            }
            for dependency in &mut task_metadata.dependencies {
                if dependency == old_id {
                    *dependency = new_task_metadata.id.clone();
                }
            }

            let task_metadata_path = loaded_task.path.join(".spielgantt").join("task.json");
            let update_path = if should_rename_folder
                && task_metadata_path.starts_with(&task_to_rename.path)
            {
                new_task_path.join(
                    task_metadata_path
                        .strip_prefix(&task_to_rename.path)
                        .expect("task metadata path should remain inside the renamed task folder"),
                )
            } else {
                task_metadata_path
            };
            Ok(Some(update_path))
        },
        RenameTaskError::SerializeMetadata,
    )?;

    if should_rename_folder {
        std::fs::rename(&task_to_rename.path, &new_task_path).map_err(|source| {
            RenameTaskError::RenameTaskFolder {
                from: task_to_rename.path.clone(),
                to: new_task_path.clone(),
                source,
            }
        })?;
    }

    if let Err(error) = crate::apply_metadata_rewrites_with_rollback(&metadata_updates)
        .map_err(WriteTaskMetadataFileError::from)
    {
        if should_rename_folder {
            let _ = std::fs::rename(&new_task_path, &task_to_rename.path);
        }
        return Err(RenameTaskError::WriteMetadata(error));
    }

    Ok(RenameTaskOutcome::Renamed {
        old_id: old_id.to_string(),
        new_id: new_task_metadata.id,
        old_path: task_to_rename.path.clone(),
        new_path: new_task_path,
    })
}

pub fn delete(
    start: &Path,
    task_id: &str,
    mode: DeleteTaskMode,
) -> Result<DeleteTaskOutcome, DeleteTaskError> {
    let package_index = TaskPackageIndex::load(start)
        .map_err(|error| project_graph::map_load_error_with_scan_from_start(error, start))?;
    let task_to_delete = package_index
        .task(task_id)
        .ok_or_else(|| DeleteTaskError::TaskNotFound(task_id.to_string()))?;
    let deleted_task_dependencies = task_to_delete.metadata.dependencies.clone();

    let deleted_metadata_path = task_to_delete.path.join(".spielgantt").join("task.json");
    let metadata_updates = package_index
        .plan_task_metadata_rewrites(
            |_, task_metadata| {
                if task_metadata.id != task_id {
                    let mut rewired_dependencies = Vec::new();
                    let mut seen_dependencies = HashSet::new();

                    for dependency in std::mem::take(&mut task_metadata.dependencies) {
                        if dependency == task_id {
                            for inherited_dependency in &deleted_task_dependencies {
                                if inherited_dependency != task_id
                                    && inherited_dependency != &task_metadata.id
                                    && seen_dependencies.insert(inherited_dependency.clone())
                                {
                                    rewired_dependencies.push(inherited_dependency.clone());
                                }
                            }
                        } else if seen_dependencies.insert(dependency.clone()) {
                            rewired_dependencies.push(dependency);
                        }
                    }

                    task_metadata.dependencies = rewired_dependencies;
                }
                Ok(None)
            },
            DeleteTaskError::SerializeMetadata,
        )?
        .into_iter()
        .filter(|rewrite| rewrite.path() != deleted_metadata_path)
        .collect::<Vec<_>>();

    match mode {
        DeleteTaskMode::RemoveFromChart => {
            let metadata_dir = task_to_delete.path.join(".spielgantt");
            let staged_metadata_dir = stage_task_metadata_for_chart_removal(&metadata_dir)?;

            let commit_report = match crate::package_mutation::PackageMutation::metadata_rewrites(
                metadata_updates.clone(),
            )
            .remove_dir_after_commit(staged_metadata_dir.clone())
            .commit()
            {
                Ok(report) => report,
                Err(error) => {
                    let write_source = WriteTaskMetadataFileError::from(error.into_source());
                    if let Err(restore_source) =
                        std::fs::rename(&staged_metadata_dir, &metadata_dir)
                    {
                        return Err(DeleteTaskError::RestoreStagedMetadataAfterWriteFailure {
                            staged_path: staged_metadata_dir,
                            metadata_path: metadata_dir,
                            write_source,
                            restore_source,
                        });
                    }
                    return Err(DeleteTaskError::WriteMetadata(write_source));
                }
            };

            let cleanup_failure = cleanup_failure_from_report(commit_report).map(|failure| {
                let error = format!(
                    "failed to remove task metadata '{}': {}",
                    failure.path().display(),
                    failure.error()
                );
                StagedCleanupFailure {
                    path: failure.path().to_path_buf(),
                    error,
                }
            });

            Ok(DeleteTaskOutcome::RemovedFromChart {
                task_id: task_id.to_string(),
                task_path: task_to_delete.path.clone(),
                cleanup_failure,
            })
        }
        DeleteTaskMode::DeleteDirectory => {
            let staged_task_dir = stage_task_directory_for_deletion(&task_to_delete.path)?;

            let commit_report = match crate::package_mutation::PackageMutation::metadata_rewrites(
                metadata_updates.clone(),
            )
            .remove_dir_after_commit(staged_task_dir.clone())
            .commit()
            {
                Ok(report) => report,
                Err(error) => {
                    let write_source = WriteTaskMetadataFileError::from(error.into_source());
                    if let Err(restore_source) =
                        std::fs::rename(&staged_task_dir, &task_to_delete.path)
                    {
                        return Err(
                            DeleteTaskError::RestoreStagedTaskDirectoryAfterWriteFailure {
                                staged_path: staged_task_dir,
                                task_path: task_to_delete.path.clone(),
                                write_source,
                                restore_source,
                            },
                        );
                    }
                    return Err(DeleteTaskError::WriteMetadata(write_source));
                }
            };

            let cleanup_failure = cleanup_failure_from_report(commit_report).map(|failure| {
                let error = format!(
                    "failed to delete task directory '{}': {}",
                    failure.path().display(),
                    failure.error()
                );
                StagedCleanupFailure {
                    path: failure.path().to_path_buf(),
                    error,
                }
            });

            Ok(DeleteTaskOutcome::DeletedDirectory {
                task_id: task_id.to_string(),
                task_path: task_to_delete.path.clone(),
                cleanup_failure,
            })
        }
    }
}

fn cleanup_failure_from_report(
    report: crate::package_mutation::PackageMutationReport,
) -> Option<crate::package_mutation::StagedCleanupFailure> {
    report.into_cleanup_failures().into_iter().next()
}

fn stage_task_metadata_for_chart_removal(metadata_dir: &Path) -> Result<PathBuf, DeleteTaskError> {
    let metadata_parent = metadata_dir
        .parent()
        .expect("task metadata directory should have a parent task directory");
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    for attempt in 0..100 {
        let staged_path = metadata_parent.join(format!(
            ".spielgantt-delete-staged-{}-{unique_suffix}-{attempt}",
            std::process::id()
        ));
        if staged_path.exists() {
            continue;
        }

        return std::fs::rename(metadata_dir, &staged_path)
            .map(|()| staged_path.clone())
            .map_err(|source| DeleteTaskError::StageMetadata {
                from: metadata_dir.to_path_buf(),
                to: staged_path,
                source,
            });
    }

    Err(DeleteTaskError::StagedMetadataPathUnavailable {
        parent: metadata_parent.to_path_buf(),
    })
}

fn stage_task_directory_for_deletion(task_dir: &Path) -> Result<PathBuf, DeleteTaskError> {
    let task_parent = task_dir
        .parent()
        .expect("task directory should have a parent directory");
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    for attempt in 0..100 {
        let staged_path = task_parent.join(format!(
            ".spielgantt-delete-staged-task-{}-{unique_suffix}-{attempt}",
            std::process::id()
        ));
        if staged_path.exists() {
            continue;
        }

        return std::fs::rename(task_dir, &staged_path)
            .map(|()| staged_path.clone())
            .map_err(|source| DeleteTaskError::StageTaskDirectory {
                from: task_dir.to_path_buf(),
                to: staged_path,
                source,
            });
    }

    Err(DeleteTaskError::StagedTaskDirectoryPathUnavailable {
        parent: task_parent.to_path_buf(),
    })
}

pub fn list(start: &Path) -> Result<Vec<TaskListEntry>, ListTasksError> {
    let index = TaskPackageIndex::load(start)
        .map_err(project_graph::map_load_error_with_scan_and_metadata)?;
    let mut tasks = index
        .loaded_tasks()
        .iter()
        .map(|task| TaskListEntry {
            id: task.metadata.id.clone(),
            status: task.metadata.status.clone(),
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(tasks)
}

pub fn resolve_task_path(start: &Path, task_id: &str) -> Result<PathBuf, ResolveTaskPathError> {
    let index = TaskPackageIndex::load(start).map_err(project_graph::map_load_error_with_scan)?;
    let task = index
        .task(task_id)
        .ok_or_else(|| ResolveTaskPathError::TaskNotFound(task_id.to_string()))?;

    std::fs::canonicalize(&task.path).map_err(|source| ResolveTaskPathError::Canonicalize {
        path: task.path.clone(),
        source,
    })
}

pub fn show(start: &Path, task_id: &str) -> Result<TaskDetails, ShowTaskError> {
    let snapshot = project_snapshot::load(start).map_err(ShowTaskError::LoadProjectSnapshot)?;
    let task = snapshot
        .task(task_id)
        .ok_or_else(|| ShowTaskError::TaskNotFound(task_id.to_string()))?;

    Ok(TaskDetails {
        id: task.id().to_string(),
        path: task.path().to_path_buf(),
        dependencies: task.dependencies().to_vec(),
        status: task.status().cloned(),
    })
}

pub(crate) fn read_task_metadata(target: &Path) -> Result<Option<TaskMetadata>, AdoptTaskError> {
    let task_metadata_path = target.join(".spielgantt").join("task.json");
    if !task_metadata_path.is_file() {
        return Ok(None);
    }

    let task_metadata_contents =
        std::fs::read_to_string(&task_metadata_path).map_err(|source| {
            AdoptTaskError::ReadMetadata {
                path: task_metadata_path.clone(),
                source,
            }
        })?;
    let task_metadata = TaskMetadata::from_json(&task_metadata_contents).map_err(|source| {
        AdoptTaskError::ParseMetadata {
            path: task_metadata_path,
            source,
        }
    })?;

    Ok(Some(task_metadata))
}

fn commit_new_task_metadata(
    path: &Path,
    contents: String,
) -> Result<(), WriteTaskMetadataFileError> {
    crate::package_mutation::PackageMutation::metadata_rewrites([crate::MetadataRewrite::new(
        path, "", contents,
    )])
    .commit()
    .map(|_| ())
    .map_err(crate::package_mutation::MetadataCommitError::into_source)
    .map_err(WriteTaskMetadataFileError::from)
}

pub(crate) fn create_prepared_task_bucket(
    task_path: &Path,
    task_metadata: &TaskMetadata,
) -> Result<(), CreateTaskBucketError> {
    std::fs::create_dir(task_path).map_err(|source| CreateTaskBucketError::CreateTaskDir {
        path: task_path.to_path_buf(),
        source,
    })?;

    if let Err(error) = create_prepared_task_bucket_after_task_dir_exists(task_path, task_metadata)
    {
        let _ = std::fs::remove_dir_all(task_path);
        return Err(error);
    }

    Ok(())
}

fn create_prepared_task_bucket_after_task_dir_exists(
    task_path: &Path,
    task_metadata: &TaskMetadata,
) -> Result<(), CreateTaskBucketError> {
    let spielgantt_dir = task_path.join(".spielgantt");
    std::fs::create_dir(&spielgantt_dir).map_err(|source| CreateTaskBucketError::CreateDir {
        path: spielgantt_dir.clone(),
        source,
    })?;

    let readme_path = task_path.join("README.md");
    std::fs::write(&readme_path, format!("# {}\n", task_metadata.id)).map_err(|source| {
        CreateTaskBucketError::WriteReadme {
            path: readme_path,
            source,
        }
    })?;

    let task_metadata_path = spielgantt_dir.join("task.json");
    commit_new_task_metadata(
        &task_metadata_path,
        task_metadata
            .to_json()
            .map_err(CreateTaskBucketError::SerializeMetadata)?,
    )
    .map_err(|source| CreateTaskBucketError::WriteMetadata {
        path: task_metadata_path,
        source,
    })?;

    Ok(())
}

fn create_task_error_from_bucket(error: CreateTaskBucketError) -> CreateTaskError {
    match error {
        CreateTaskBucketError::CreateTaskDir { path, source } => {
            CreateTaskError::CreateTaskDir { path, source }
        }
        CreateTaskBucketError::CreateDir { path, source } => {
            CreateTaskError::CreateDir { path, source }
        }
        CreateTaskBucketError::WriteReadme { path, source } => {
            CreateTaskError::WriteReadme { path, source }
        }
        CreateTaskBucketError::SerializeMetadata(source) => CreateTaskError::InvalidTaskId(source),
        CreateTaskBucketError::WriteMetadata { path, source } => {
            CreateTaskError::WriteMetadata { path, source }
        }
    }
}

fn map_namespace_error_to_adopt_task(error: ProjectNamespaceError) -> AdoptTaskError {
    match error {
        ProjectNamespaceError::InvalidTaskId(source) => AdoptTaskError::InvalidTaskMetadata(source),
        ProjectNamespaceError::TaskIdCollidesWithProjectEvent { id } => {
            AdoptTaskError::TaskIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::TaskIdCollidesWithProjectTask { id, existing_path } => {
            AdoptTaskError::DuplicateTaskId { id, existing_path }
        }
        ProjectNamespaceError::InvalidEventId(_)
        | ProjectNamespaceError::EventIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::EventIdCollidesWithProjectEvent { .. } => {
            unreachable!("task namespace validation should only return task namespace errors")
        }
    }
}

fn map_namespace_error_to_create_task(error: ProjectNamespaceError) -> CreateTaskError {
    match error {
        ProjectNamespaceError::InvalidTaskId(source) => CreateTaskError::InvalidTaskId(source),
        ProjectNamespaceError::TaskIdCollidesWithProjectEvent { id } => {
            CreateTaskError::TaskIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::TaskIdCollidesWithProjectTask { id, existing_path } => {
            CreateTaskError::DuplicateTaskId { id, existing_path }
        }
        ProjectNamespaceError::InvalidEventId(_)
        | ProjectNamespaceError::EventIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::EventIdCollidesWithProjectEvent { .. } => {
            unreachable!("task namespace validation should only return task namespace errors")
        }
    }
}

fn map_namespace_error_to_rename_task(error: ProjectNamespaceError) -> RenameTaskError {
    match error {
        ProjectNamespaceError::InvalidTaskId(source) => RenameTaskError::InvalidTaskId(source),
        ProjectNamespaceError::TaskIdCollidesWithProjectEvent { id } => {
            RenameTaskError::TaskIdCollidesWithProjectEvent { id }
        }
        ProjectNamespaceError::TaskIdCollidesWithProjectTask { id, existing_path } => {
            RenameTaskError::DuplicateTaskId { id, existing_path }
        }
        ProjectNamespaceError::InvalidEventId(_)
        | ProjectNamespaceError::EventIdCollidesWithProjectTask { .. }
        | ProjectNamespaceError::EventIdCollidesWithProjectEvent { .. } => {
            unreachable!("task namespace validation should only return task namespace errors")
        }
    }
}

pub(crate) fn write_task_metadata_file(
    path: &Path,
    contents: &str,
) -> Result<(), WriteTaskMetadataFileError> {
    crate::package_mutation::write_json_metadata_file(path, contents)
        .map_err(WriteTaskMetadataFileError::from)
}

pub(crate) fn normalized_task_folder_name(
    folder_naming: &FolderNamingPolicy,
    task_id: &str,
) -> String {
    match folder_naming {
        FolderNamingPolicy::TaskId => task_id.to_string(),
    }
}

fn final_normalized_task_path(
    task_path: &Path,
    task_id: &str,
    project_root: &Path,
    folder_naming: &FolderNamingPolicy,
    task_ids_by_path: &HashMap<PathBuf, String>,
    final_paths_by_path: &mut HashMap<PathBuf, PathBuf>,
) -> PathBuf {
    if let Some(final_path) = final_paths_by_path.get(task_path) {
        return final_path.clone();
    }

    let parent = task_path.parent().unwrap_or(project_root);
    let final_parent = nearest_task_ancestor(task_path, task_ids_by_path)
        .map(|ancestor_path| {
            let ancestor_id = task_ids_by_path
                .get(&ancestor_path)
                .expect("nearest task ancestor should have a task id");
            let ancestor_final_path = final_normalized_task_path(
                &ancestor_path,
                ancestor_id,
                project_root,
                folder_naming,
                task_ids_by_path,
                final_paths_by_path,
            );
            let relative_parent = parent
                .strip_prefix(&ancestor_path)
                .expect("task parent should be under nearest task ancestor");
            ancestor_final_path.join(relative_parent)
        })
        .unwrap_or_else(|| parent.to_path_buf());

    let final_path = final_parent.join(normalized_task_folder_name(folder_naming, task_id));
    final_paths_by_path.insert(task_path.to_path_buf(), final_path.clone());
    final_path
}

fn nearest_task_ancestor(
    task_path: &Path,
    task_ids_by_path: &HashMap<PathBuf, String>,
) -> Option<PathBuf> {
    let mut candidate = task_path.parent();
    while let Some(path) = candidate {
        if task_ids_by_path.contains_key(path) {
            return Some(path.to_path_buf());
        }
        candidate = path.parent();
    }

    None
}

pub(crate) fn read_loaded_task_metadata(
    project_root: &Path,
) -> Result<Vec<project_graph::LoadedProjectGraphTask>, ScanTasksError> {
    crate::task_package_index::read_loaded_tasks(project_root)
        .map_err(project_graph::map_load_error)
}

fn scanned_tasks_from_loaded(
    loaded_tasks: &[project_graph::LoadedProjectGraphTask],
) -> Vec<ScannedTask> {
    let mut tasks = loaded_tasks
        .iter()
        .map(|task| ScannedTask {
            id: task.metadata.id.clone(),
            path: task.path.clone(),
        })
        .collect::<Vec<_>>();
    tasks.sort_by(|left, right| left.path.cmp(&right.path));
    tasks
}

#[derive(Debug)]
pub enum ScanTasksError {
    WalkProject {
        path: PathBuf,
        source: walkdir::Error,
    },
    DuplicateTaskId {
        id: String,
        first_path: PathBuf,
        second_path: PathBuf,
    },
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl std::fmt::Display for ScanTasksError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WalkProject { path, source } => {
                write!(
                    formatter,
                    "failed to scan project '{}' for task metadata: {source}",
                    path.display()
                )
            }
            Self::DuplicateTaskId {
                id,
                first_path,
                second_path,
            } => {
                write!(
                    formatter,
                    "duplicate task id '{id}' found in '{}' and '{}'",
                    first_path.display(),
                    second_path.display()
                )
            }
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

impl std::error::Error for ScanTasksError {}

impl project_graph::FromLoadProjectGraphError for ScanTasksError {
    fn project_not_found(_path: PathBuf) -> Self {
        unreachable!("task metadata loading should not resolve project roots")
    }

    fn read_project_metadata(_source: project::ReadProjectMetadataError) -> Self {
        unreachable!("task metadata loading should not read project metadata")
    }

    fn walk_project(path: PathBuf, source: walkdir::Error) -> Self {
        Self::WalkProject { path, source }
    }

    fn duplicate_task_id(id: String, first_path: PathBuf, second_path: PathBuf) -> Self {
        Self::DuplicateTaskId {
            id,
            first_path,
            second_path,
        }
    }

    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadMetadata { path, source }
    }

    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self {
        Self::ParseMetadata { path, source }
    }
}

#[derive(Debug)]
pub enum AdoptTaskError {
    InvalidTargetPath {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    CurrentDirectory(std::io::Error),
    TargetIsNotDirectory(std::path::PathBuf),
    ProjectNotFound(std::path::PathBuf),
    InvalidTaskMetadata(MetadataError),
    AlreadyAdoptedWithDifferentId {
        path: std::path::PathBuf,
        existing_id: String,
        requested_id: String,
    },
    DuplicateTaskId {
        id: String,
        existing_path: std::path::PathBuf,
    },
    DuplicateTaskIdsInProject {
        id: String,
        first_path: std::path::PathBuf,
        second_path: std::path::PathBuf,
    },
    ReadProjectMetadata(project::ReadProjectMetadataError),
    TaskIdCollidesWithProjectEvent {
        id: String,
    },
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    WalkProject {
        path: std::path::PathBuf,
        source: walkdir::Error,
    },
    ReadMetadata {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: std::path::PathBuf,
        source: MetadataError,
    },
    WriteMetadata {
        path: std::path::PathBuf,
        source: WriteTaskMetadataFileError,
    },
}

impl std::fmt::Display for AdoptTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTargetPath { path, source } => {
                write!(
                    formatter,
                    "invalid task path '{}': {source}",
                    path.display()
                )
            }
            Self::CurrentDirectory(source) => {
                write!(
                    formatter,
                    "failed to resolve the current directory: {source}"
                )
            }
            Self::TargetIsNotDirectory(path) => {
                write!(
                    formatter,
                    "invalid task path '{}': target is not a directory",
                    path.display()
                )
            }
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot adopt '{}': folder is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::InvalidTaskMetadata(source) => write!(formatter, "{source}"),
            Self::AlreadyAdoptedWithDifferentId {
                path,
                existing_id,
                requested_id,
            } => {
                write!(
                    formatter,
                    "task folder '{}' is already adopted as '{existing_id}' and cannot be re-adopted as '{requested_id}'",
                    path.display()
                )
            }
            Self::DuplicateTaskId { id, existing_path } => {
                write!(
                    formatter,
                    "task id '{id}' already exists in '{}'",
                    existing_path.display()
                )
            }
            Self::DuplicateTaskIdsInProject {
                id,
                first_path,
                second_path,
            } => {
                write!(
                    formatter,
                    "duplicate task id '{id}' found in '{}' and '{}'",
                    first_path.display(),
                    second_path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::TaskIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "task id '{id}' collides with project event id '{id}'"
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create metadata directory '{}': {source}",
                    path.display()
                )
            }
            Self::WalkProject { path, source } => {
                write!(
                    formatter,
                    "failed to scan project '{}' for existing tasks: {source}",
                    path.display()
                )
            }
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
            Self::WriteMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to write task metadata '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for AdoptTaskError {}

impl project_graph::FromLoadProjectGraphError for AdoptTaskError {
    fn project_not_found(path: PathBuf) -> Self {
        Self::ProjectNotFound(path)
    }

    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self {
        Self::ReadProjectMetadata(source)
    }

    fn walk_project(path: PathBuf, source: walkdir::Error) -> Self {
        Self::WalkProject { path, source }
    }

    fn duplicate_task_id(id: String, first_path: PathBuf, second_path: PathBuf) -> Self {
        Self::DuplicateTaskIdsInProject {
            id,
            first_path,
            second_path,
        }
    }

    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadMetadata { path, source }
    }

    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self {
        Self::ParseMetadata { path, source }
    }
}

#[derive(Debug)]
pub(crate) enum CreateTaskBucketError {
    CreateTaskDir {
        path: PathBuf,
        source: std::io::Error,
    },
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    WriteReadme {
        path: PathBuf,
        source: std::io::Error,
    },
    SerializeMetadata(MetadataError),
    WriteMetadata {
        path: PathBuf,
        source: WriteTaskMetadataFileError,
    },
}

#[derive(Debug)]
pub enum CreateTaskError {
    ProjectNotFound(std::path::PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    InvalidTaskId(MetadataError),
    TaskIdCollidesWithProjectEvent {
        id: String,
    },
    ScanExistingTasks(AdoptTaskError),
    DuplicateTaskId {
        id: String,
        existing_path: std::path::PathBuf,
    },
    FolderAlreadyExists(std::path::PathBuf),
    CreateTaskDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    CreateDir {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    WriteReadme {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    WriteMetadata {
        path: std::path::PathBuf,
        source: WriteTaskMetadataFileError,
    },
}

impl std::fmt::Display for CreateTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot create task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::InvalidTaskId(source) => write!(formatter, "{source}"),
            Self::TaskIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "task id '{id}' collides with project event id '{id}'"
                )
            }
            Self::ScanExistingTasks(source) => write!(formatter, "{source}"),
            Self::DuplicateTaskId { id, existing_path } => {
                write!(
                    formatter,
                    "task id '{id}' already exists in '{}'",
                    existing_path.display()
                )
            }
            Self::FolderAlreadyExists(path) => {
                write!(formatter, "task folder '{}' already exists", path.display())
            }
            Self::CreateTaskDir { path, source } => {
                write!(
                    formatter,
                    "failed to create task folder '{}': {source}",
                    path.display()
                )
            }
            Self::CreateDir { path, source } => {
                write!(
                    formatter,
                    "failed to create metadata directory '{}': {source}",
                    path.display()
                )
            }
            Self::WriteReadme { path, source } => {
                write!(
                    formatter,
                    "failed to write task README '{}': {source}",
                    path.display()
                )
            }
            Self::WriteMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to write task metadata '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for CreateTaskError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for CreateTaskError {
    type ScanError = AdoptTaskError;

    fn project_not_found(path: PathBuf) -> Self {
        Self::ProjectNotFound(path)
    }

    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self {
        Self::ReadProjectMetadata(source)
    }

    fn scan_error(source: Self::ScanError) -> Self {
        Self::ScanExistingTasks(source)
    }
}

#[derive(Debug)]
pub enum NormalizeTasksError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TargetAlreadyExists(PathBuf),
    RenameTaskFolder {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for NormalizeTasksError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot normalize from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TargetAlreadyExists(path) => {
                write!(
                    formatter,
                    "cannot normalize task folders: target folder '{}' already exists",
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

impl std::error::Error for NormalizeTasksError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for NormalizeTasksError {
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

#[derive(Debug)]
pub enum RenameTaskError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    InvalidTaskId(MetadataError),
    TaskIdCollidesWithProjectEvent {
        id: String,
    },
    ScanTasks(ScanTasksError),
    WalkProject {
        path: PathBuf,
        source: walkdir::Error,
    },
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
    SerializeMetadata(MetadataError),
    TaskNotFound(String),
    DuplicateTaskId {
        id: String,
        existing_path: PathBuf,
    },
    TargetAlreadyExists(PathBuf),
    WriteMetadata(WriteTaskMetadataFileError),
    RenameTaskFolder {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for RenameTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot rename task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::InvalidTaskId(source) => write!(formatter, "{source}"),
            Self::TaskIdCollidesWithProjectEvent { id } => {
                write!(
                    formatter,
                    "task id '{id}' collides with project event id '{id}'"
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::WalkProject { path, source } => {
                write!(
                    formatter,
                    "failed to scan project '{}' for task metadata: {source}",
                    path.display()
                )
            }
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
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::DuplicateTaskId { id, existing_path } => {
                write!(
                    formatter,
                    "task id '{id}' already exists in '{}'",
                    existing_path.display()
                )
            }
            Self::TargetAlreadyExists(path) => {
                write!(
                    formatter,
                    "cannot rename task: target folder '{}' already exists",
                    path.display()
                )
            }
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
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

impl std::error::Error for RenameTaskError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for RenameTaskError {
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

#[derive(Debug)]
pub enum DeleteTaskError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    SerializeMetadata(MetadataError),
    TaskNotFound(String),
    WriteMetadata(WriteTaskMetadataFileError),
    StageMetadata {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    StageTaskDirectory {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    StagedMetadataPathUnavailable {
        parent: PathBuf,
    },
    StagedTaskDirectoryPathUnavailable {
        parent: PathBuf,
    },
    RestoreStagedMetadataAfterWriteFailure {
        staged_path: PathBuf,
        metadata_path: PathBuf,
        write_source: WriteTaskMetadataFileError,
        restore_source: std::io::Error,
    },
    RestoreStagedTaskDirectoryAfterWriteFailure {
        staged_path: PathBuf,
        task_path: PathBuf,
        write_source: WriteTaskMetadataFileError,
        restore_source: std::io::Error,
    },
    RemoveMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    RemoveTaskDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for DeleteTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot delete task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::SerializeMetadata(source) => {
                write!(formatter, "failed to serialize task metadata: {source}")
            }
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::WriteMetadata(source) => write!(formatter, "{source}"),
            Self::StageMetadata { from, to, source } => {
                write!(
                    formatter,
                    "failed to stage task metadata '{}' at '{}' before deleting task from chart: {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::StageTaskDirectory { from, to, source } => {
                write!(
                    formatter,
                    "failed to stage task directory '{}' at '{}' before deleting task directory: {source}",
                    from.display(),
                    to.display()
                )
            }
            Self::StagedMetadataPathUnavailable { parent } => {
                write!(
                    formatter,
                    "failed to stage task metadata before deleting task from chart: no available staged metadata path under '{}'",
                    parent.display()
                )
            }
            Self::StagedTaskDirectoryPathUnavailable { parent } => {
                write!(
                    formatter,
                    "failed to stage task directory before deleting task directory: no available staged task directory path under '{}'",
                    parent.display()
                )
            }
            Self::RestoreStagedMetadataAfterWriteFailure {
                staged_path,
                metadata_path,
                write_source,
                restore_source,
            } => {
                write!(
                    formatter,
                    "{write_source}; additionally failed to restore staged task metadata '{}' to '{}': {restore_source}",
                    staged_path.display(),
                    metadata_path.display()
                )
            }
            Self::RestoreStagedTaskDirectoryAfterWriteFailure {
                staged_path,
                task_path,
                write_source,
                restore_source,
            } => {
                write!(
                    formatter,
                    "{write_source}; additionally failed to restore staged task directory '{}' to '{}': {restore_source}",
                    staged_path.display(),
                    task_path.display()
                )
            }
            Self::RemoveMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to remove task metadata '{}': {source}",
                    path.display()
                )
            }
            Self::RemoveTaskDirectory { path, source } => {
                write!(
                    formatter,
                    "failed to delete task directory '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DeleteTaskError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for DeleteTaskError {
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

#[derive(Debug)]
pub enum ListTasksError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl std::fmt::Display for ListTasksError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot list tasks from '{}': current directory is not inside a SpielGantt project",
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

impl std::error::Error for ListTasksError {}

impl project_graph::FromLoadProjectGraphErrorWithScanAndMetadata for ListTasksError {
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

    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadMetadata { path, source }
    }

    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self {
        Self::ParseMetadata { path, source }
    }
}

#[derive(Debug)]
pub enum ResolveTaskPathError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(ScanTasksError),
    TaskNotFound(String),
    Canonicalize {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ResolveTaskPathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot resolve task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ReadProjectMetadata(source) => write!(formatter, "{source}"),
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
            Self::Canonicalize { path, source } => {
                write!(
                    formatter,
                    "failed to resolve task path '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ResolveTaskPathError {}

impl project_graph::FromLoadProjectGraphErrorWithScan for ResolveTaskPathError {
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

#[derive(Debug)]
pub enum ReadmeTaskError {
    ResolveTask(ResolveTaskPathError),
    CreateReadme {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ReadmeTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResolveTask(source) => write!(formatter, "{source}"),
            Self::CreateReadme { path, source } => {
                write!(
                    formatter,
                    "failed to create README '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ReadmeTaskError {}

#[derive(Debug)]
pub enum ShowTaskError {
    ProjectNotFound(PathBuf),
    ScanTasks(ScanTasksError),
    LoadProjectSnapshot(project_snapshot::LoadProjectSnapshotError),
    TaskNotFound(String),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl std::fmt::Display for ShowTaskError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "cannot show task from '{}': current directory is not inside a SpielGantt project",
                    path.display()
                )
            }
            Self::ScanTasks(source) => write!(formatter, "{source}"),
            Self::LoadProjectSnapshot(source) => write!(formatter, "{source}"),
            Self::TaskNotFound(id) => write!(formatter, "task id '{id}' was not found"),
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

impl std::error::Error for ShowTaskError {}

#[derive(Debug)]
pub struct WriteTaskMetadataFileError(crate::package_mutation::AtomicJsonMetadataWriteError);

impl From<crate::package_mutation::AtomicJsonMetadataWriteError> for WriteTaskMetadataFileError {
    fn from(source: crate::package_mutation::AtomicJsonMetadataWriteError) -> Self {
        Self(source)
    }
}

impl std::fmt::Display for WriteTaskMetadataFileError {
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

impl std::error::Error for WriteTaskMetadataFileError {}
