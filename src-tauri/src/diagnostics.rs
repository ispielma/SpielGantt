use std::path::{Path, PathBuf};

use crate::{
    metadata::{self, ProjectNamespaceKind},
    task_package_index,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnostics {
    project_root: Option<PathBuf>,
    issues: Vec<ProjectDiagnosticIssue>,
    tasks: Vec<ProjectDiagnosticTask>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnosticIssue {
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnosticTask {
    id: String,
    path: PathBuf,
    project_relative_path: String,
    dependencies: Vec<String>,
    ends_at: Option<String>,
}

impl ProjectDiagnostics {
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub fn issues(&self) -> &[ProjectDiagnosticIssue] {
        &self.issues
    }

    pub fn tasks(&self) -> &[ProjectDiagnosticTask] {
        &self.tasks
    }
}

impl ProjectDiagnosticIssue {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl ProjectDiagnosticTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn project_relative_path(&self) -> &str {
        &self.project_relative_path
    }
}

pub fn read(start: &Path) -> Result<ProjectDiagnostics, ReadProjectDiagnosticsError> {
    let package_read = task_package_index::TaskPackageIndex::read(start)
        .map_err(ReadProjectDiagnosticsError::ReadPackageIndex)?;

    let Some(project_root) = package_read.project_root() else {
        return Ok(ProjectDiagnostics {
            project_root: None,
            issues: vec![ProjectDiagnosticIssue::new(format!(
                "missing project metadata: no '.spielgantt/project.json' found at or above '{}'",
                package_read.selected_path().display()
            ))],
            tasks: Vec::new(),
        });
    };
    let project_root = project_root.to_path_buf();

    let mut issues = package_read
        .issues()
        .iter()
        .map(|issue| ProjectDiagnosticIssue::new(issue.message().to_string()))
        .collect::<Vec<_>>();
    let project_events = package_read
        .project_metadata()
        .map(|metadata| metadata.chart_events())
        .unwrap_or_default();

    let mut tasks = Vec::new();
    for loaded_task in package_read.loaded_tasks() {
        let task_metadata = &loaded_task.metadata;
        if let Err(error) = metadata::validate_project_namespace_entry(
            &task_metadata.id,
            ProjectNamespaceKind::Task,
            ProjectNamespaceKind::Event,
            &project_events,
        ) {
            issues.push(ProjectDiagnosticIssue::new(error.to_string()));
        }

        tasks.push(ProjectDiagnosticTask {
            id: task_metadata.id.clone(),
            project_relative_path: display_path_relative_to(&loaded_task.path, &project_root),
            path: loaded_task.path.clone(),
            dependencies: task_metadata.dependencies.clone(),
            ends_at: task_metadata.ends_at.clone(),
        });
    }

    let graph = package_read
        .graph()
        .expect("package reads with a project root should include a graph");
    for task in graph.tasks() {
        for dependency in task.dependency_references() {
            if let Some(dependency_ends_at) =
                graph.task(dependency.id()).and_then(|task| task.ends_at())
            {
                issues.push(ProjectDiagnosticIssue::new(format!(
                    "task '{}' depends on task '{}' which ends at event '{}'; depend on the event instead",
                    task.id(), dependency.id(), dependency_ends_at
                )));
            } else if graph.task(dependency.id()).is_none() && !graph.has_event(dependency.id()) {
                issues.push(ProjectDiagnosticIssue::new(format!(
                    "task '{}' depends on missing task or event id '{}'",
                    task.id(),
                    dependency.id()
                )));
            }
        }

        if let Some(ends_at) = task.ends_at() {
            if graph.task(ends_at).is_some() {
                issues.push(ProjectDiagnosticIssue::new(format!(
                    "task '{}' ends_at must reference an event id, not task id '{}'",
                    task.id(),
                    ends_at
                )));
            } else if !graph.has_event(ends_at) {
                issues.push(ProjectDiagnosticIssue::new(format!(
                    "task '{}' ends_at references missing event id '{}'",
                    task.id(),
                    ends_at
                )));
            }
        }
    }

    if let Some(cycle) = graph.dependency_cycle() {
        issues.push(ProjectDiagnosticIssue::new(format!(
            "dependency cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    Ok(ProjectDiagnostics {
        project_root: Some(project_root),
        issues,
        tasks,
    })
}

fn display_path_relative_to(path: &Path, project_root: &Path) -> String {
    path.strip_prefix(project_root)
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[derive(Debug)]
pub enum ReadProjectDiagnosticsError {
    ReadPackageIndex(task_package_index::ReadTaskPackageIndexError),
}

impl std::fmt::Display for ReadProjectDiagnosticsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadPackageIndex(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for ReadProjectDiagnosticsError {}
