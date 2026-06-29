use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    metadata::{ProjectMetadata, TaskMetadata},
    project, project_graph, MetadataRewrite,
};

#[derive(Debug, Clone)]
pub(crate) struct TaskPackageIndex {
    project_root: PathBuf,
    project_metadata: ProjectMetadata,
    loaded_tasks: Vec<project_graph::LoadedProjectGraphTask>,
    task_indexes_by_id: HashMap<String, usize>,
    graph: project_graph::ProjectGraph,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskPackageRead {
    selected_path: PathBuf,
    project_root: Option<PathBuf>,
    project_metadata: Option<ProjectMetadata>,
    issues: Vec<TaskPackageReadIssue>,
    loaded_tasks: Vec<project_graph::LoadedProjectGraphTask>,
    graph: Option<project_graph::ProjectGraph>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskPackageReadIssue {
    message: String,
}

impl TaskPackageIndex {
    pub(crate) fn load(start: &Path) -> Result<Self, project_graph::LoadProjectGraphError> {
        let project_root = project::find_root(start).ok_or_else(|| {
            project_graph::LoadProjectGraphError::ProjectNotFound(start.to_path_buf())
        })?;
        let project_metadata = project::read_metadata(&project_root)
            .map_err(project_graph::LoadProjectGraphError::ReadProjectMetadata)?;
        let loaded_tasks =
            read_task_packages(&project_root, TaskPackageReadPolicy::Strict)?.loaded_tasks;
        let task_indexes_by_id = task_indexes_by_id(&loaded_tasks);

        let mut graph_tasks = loaded_tasks.clone();
        graph_tasks.sort_by(|left, right| left.metadata.id.cmp(&right.metadata.id));
        let graph = project_graph::from_loaded_tasks(
            project_root.clone(),
            project_metadata.chart_events(),
            project_metadata.resolved_boundary_events(),
            graph_tasks,
        );

        Ok(Self {
            project_root,
            project_metadata,
            loaded_tasks,
            task_indexes_by_id,
            graph,
        })
    }

    pub(crate) fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub(crate) fn project_metadata(&self) -> &ProjectMetadata {
        &self.project_metadata
    }

    pub(crate) fn loaded_tasks(&self) -> &[project_graph::LoadedProjectGraphTask] {
        &self.loaded_tasks
    }

    pub(crate) fn task(&self, id: &str) -> Option<&project_graph::LoadedProjectGraphTask> {
        self.task_indexes_by_id
            .get(id)
            .map(|index| &self.loaded_tasks[*index])
    }

    pub(crate) fn graph(&self) -> &project_graph::ProjectGraph {
        &self.graph
    }

    pub(crate) fn plan_task_metadata_rewrites<E, F, S>(
        &self,
        edit_task_metadata: F,
        serialize_error: S,
    ) -> Result<Vec<MetadataRewrite>, E>
    where
        F: FnMut(
            &project_graph::LoadedProjectGraphTask,
            &mut TaskMetadata,
        ) -> Result<Option<PathBuf>, E>,
        S: FnMut(crate::metadata::MetadataError) -> E,
    {
        crate::plan_loaded_task_metadata_rewrites(
            &self.loaded_tasks,
            edit_task_metadata,
            serialize_error,
        )
    }

    pub(crate) fn read(start: &Path) -> Result<TaskPackageRead, ReadTaskPackageIndexError> {
        let selected_path = if start.is_absolute() {
            start.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(ReadTaskPackageIndexError::CurrentDirectory)?
                .join(start)
        };

        let Some(project_root) = project::find_root(&selected_path) else {
            return Ok(TaskPackageRead {
                selected_path,
                project_root: None,
                project_metadata: None,
                issues: Vec::new(),
                loaded_tasks: Vec::new(),
                graph: None,
            });
        };

        let mut issues = Vec::new();
        let project_metadata = match project::read_metadata(&project_root) {
            Ok(project_metadata) => Some(project_metadata),
            Err(error) => {
                issues.push(TaskPackageReadIssue::new(error.to_string()));
                None
            }
        };
        let (project_events, boundary_events) = project_metadata
            .as_ref()
            .map(|metadata| (metadata.chart_events(), metadata.resolved_boundary_events()))
            .unwrap_or((Vec::new(), None));

        let diagnostic_read = read_task_packages(&project_root, TaskPackageReadPolicy::Diagnostic)?;
        issues.extend(diagnostic_read.issues);
        let loaded_tasks = diagnostic_read.loaded_tasks;
        let graph = project_graph::from_loaded_tasks(
            project_root.clone(),
            project_events,
            boundary_events,
            loaded_tasks.clone(),
        );

        Ok(TaskPackageRead {
            selected_path,
            project_root: Some(project_root),
            project_metadata,
            issues,
            loaded_tasks,
            graph: Some(graph),
        })
    }
}

impl TaskPackageRead {
    pub(crate) fn selected_path(&self) -> &Path {
        &self.selected_path
    }

    pub(crate) fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub(crate) fn project_metadata(&self) -> Option<&ProjectMetadata> {
        self.project_metadata.as_ref()
    }

    pub(crate) fn issues(&self) -> &[TaskPackageReadIssue] {
        &self.issues
    }

    pub(crate) fn loaded_tasks(&self) -> &[project_graph::LoadedProjectGraphTask] {
        &self.loaded_tasks
    }

    pub(crate) fn graph(&self) -> Option<&project_graph::ProjectGraph> {
        self.graph.as_ref()
    }
}

impl TaskPackageReadIssue {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

pub(crate) fn read_loaded_tasks(
    project_root: &Path,
) -> Result<Vec<project_graph::LoadedProjectGraphTask>, project_graph::LoadProjectGraphError> {
    Ok(read_task_packages(project_root, TaskPackageReadPolicy::Strict)?.loaded_tasks)
}

fn read_task_packages(
    project_root: &Path,
    policy: TaskPackageReadPolicy,
) -> Result<TaskPackageReadResult, project_graph::LoadProjectGraphError> {
    let mut task_ids = HashMap::<String, PathBuf>::new();
    let mut tasks = Vec::new();
    let mut issues = Vec::new();

    for task_metadata_path in task_metadata_paths(project_root)? {
        let task_path = task_path_from_metadata_path(&task_metadata_path);
        let task_metadata_contents = match std::fs::read_to_string(&task_metadata_path) {
            Ok(task_metadata_contents) => task_metadata_contents,
            Err(source) => match policy {
                TaskPackageReadPolicy::Strict => {
                    return Err(project_graph::LoadProjectGraphError::ReadMetadata {
                        path: task_metadata_path,
                        source,
                    });
                }
                TaskPackageReadPolicy::Diagnostic => {
                    issues.push(TaskPackageReadIssue::new(format!(
                        "failed to read task metadata '{}': {source}",
                        task_metadata_path.display()
                    )));
                    continue;
                }
            },
        };

        let task_metadata = match TaskMetadata::from_json(&task_metadata_contents) {
            Ok(task_metadata) => task_metadata,
            Err(source) => match policy {
                TaskPackageReadPolicy::Strict => {
                    return Err(project_graph::LoadProjectGraphError::ParseMetadata {
                        path: task_metadata_path,
                        source,
                    });
                }
                TaskPackageReadPolicy::Diagnostic => {
                    issues.push(TaskPackageReadIssue::new(format!(
                        "failed to parse task metadata '{}': {source}",
                        task_metadata_path.display()
                    )));
                    continue;
                }
            },
        };

        if let Some(existing_path) = task_ids.insert(task_metadata.id.clone(), task_path.clone()) {
            match policy {
                TaskPackageReadPolicy::Strict => {
                    return Err(project_graph::LoadProjectGraphError::DuplicateTaskId {
                        id: task_metadata.id,
                        first_path: existing_path,
                        second_path: task_path,
                    });
                }
                TaskPackageReadPolicy::Diagnostic => {
                    issues.push(TaskPackageReadIssue::new(format!(
                        "duplicate task id '{}' found in '{}' and '{}'",
                        task_metadata.id,
                        existing_path.display(),
                        task_path.display()
                    )));
                }
            }
        }

        tasks.push(project_graph::LoadedProjectGraphTask {
            path: task_path,
            metadata: task_metadata,
            original_metadata_contents: task_metadata_contents,
        });
    }

    Ok(TaskPackageReadResult {
        issues,
        loaded_tasks: tasks,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskPackageReadPolicy {
    Strict,
    Diagnostic,
}

#[derive(Debug, Clone)]
struct TaskPackageReadResult {
    issues: Vec<TaskPackageReadIssue>,
    loaded_tasks: Vec<project_graph::LoadedProjectGraphTask>,
}

fn task_metadata_paths(
    project_root: &Path,
) -> Result<Vec<PathBuf>, project_graph::LoadProjectGraphError> {
    let mut paths = Vec::new();

    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_entry(|entry| entry.depth() == 0 || !is_staged_delete_path(entry.path()))
    {
        let entry = entry.map_err(|source| project_graph::LoadProjectGraphError::WalkProject {
            path: project_root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name() else {
            continue;
        };
        if file_name != "task.json" {
            continue;
        }

        let Some(metadata_dir) = path.parent() else {
            continue;
        };
        let Some(metadata_dir_name) = metadata_dir.file_name() else {
            continue;
        };
        if metadata_dir_name != ".spielgantt" {
            continue;
        }

        paths.push(path.to_path_buf());
    }

    paths.sort();
    Ok(paths)
}

fn task_path_from_metadata_path(task_metadata_path: &Path) -> PathBuf {
    task_metadata_path
        .parent()
        .and_then(Path::parent)
        .expect("task metadata directory should have a parent")
        .to_path_buf()
}

pub(crate) fn is_staged_delete_path(path: &Path) -> bool {
    path.file_name()
        .is_some_and(is_staged_delete_path_component)
}

pub(crate) fn is_staged_delete_path_component(name: &OsStr) -> bool {
    name.to_str()
        .is_some_and(|name| name.starts_with(".spielgantt-delete-staged-"))
}

fn task_indexes_by_id(
    loaded_tasks: &[project_graph::LoadedProjectGraphTask],
) -> HashMap<String, usize> {
    loaded_tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.metadata.id.clone(), index))
        .collect()
}

#[derive(Debug)]
pub enum ReadTaskPackageIndexError {
    CurrentDirectory(std::io::Error),
    WalkProject {
        path: PathBuf,
        source: walkdir::Error,
    },
    ReadTaskMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl From<project_graph::LoadProjectGraphError> for ReadTaskPackageIndexError {
    fn from(error: project_graph::LoadProjectGraphError) -> Self {
        match error {
            project_graph::LoadProjectGraphError::WalkProject { path, source } => {
                Self::WalkProject { path, source }
            }
            project_graph::LoadProjectGraphError::ReadMetadata { path, source } => {
                Self::ReadTaskMetadata { path, source }
            }
            project_graph::LoadProjectGraphError::ProjectNotFound(_)
            | project_graph::LoadProjectGraphError::ReadProjectMetadata(_)
            | project_graph::LoadProjectGraphError::DuplicateTaskId { .. }
            | project_graph::LoadProjectGraphError::ParseMetadata { .. } => {
                unreachable!("diagnostic package reads should not convert this load error")
            }
        }
    }
}

impl std::fmt::Display for ReadTaskPackageIndexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentDirectory(source) => {
                write!(formatter, "failed to read current directory: {source}")
            }
            Self::WalkProject { path, source } => {
                write!(
                    formatter,
                    "failed to scan project '{}' for task metadata: {source}",
                    path.display()
                )
            }
            Self::ReadTaskMetadata { path, source } => {
                write!(
                    formatter,
                    "failed to read task metadata '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ReadTaskPackageIndexError {}
