use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::{
    agent_scaffold, dependency_relationships, event_axis_workflow, metadata::TaskStatus,
    project_snapshot, semantic_projection, validation,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectResult {
    selected_path: PathBuf,
    project_root: Option<PathBuf>,
    valid: bool,
    issues: Vec<String>,
    agent_readiness: agent_scaffold::AgentReadinessStatus,
    project_readme_content: String,
    project_readme_version: String,
    events: Vec<String>,
    event_references: Vec<OpenProjectEventReferences>,
    workflow: Option<event_axis_workflow::EventAxisWorkflow>,
    tasks: Vec<OpenProjectTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectEventReferences {
    id: String,
    referenced_task_ids: Vec<String>,
    blocker_task_ids: Vec<String>,
    blocked_task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectTask {
    id: String,
    path: PathBuf,
    project_relative_path: String,
    dependencies: Vec<String>,
    blocks: Vec<project_snapshot::ProjectSnapshotDependencyTarget>,
    dependency_references: Vec<project_snapshot::ProjectSnapshotDependencyReference>,
    dependency_targets: Vec<project_snapshot::ProjectSnapshotDependencyTarget>,
    ends_at: Option<String>,
    status: Option<TaskStatus>,
    readme_content: String,
    readme_version: String,
}

impl OpenProjectResult {
    pub fn selected_path(&self) -> &Path {
        &self.selected_path
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }

    pub fn agent_readiness(&self) -> &agent_scaffold::AgentReadinessStatus {
        &self.agent_readiness
    }

    pub fn project_readme_content(&self) -> &str {
        &self.project_readme_content
    }

    pub fn project_readme_version(&self) -> &str {
        &self.project_readme_version
    }

    pub fn events(&self) -> &[String] {
        &self.events
    }

    pub fn event_references(&self) -> &[OpenProjectEventReferences] {
        &self.event_references
    }

    pub fn workflow(&self) -> Option<&event_axis_workflow::EventAxisWorkflow> {
        self.workflow.as_ref()
    }

    pub fn tasks(&self) -> &[OpenProjectTask] {
        &self.tasks
    }
}

impl OpenProjectTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project_relative_path(&self) -> &str {
        &self.project_relative_path
    }

    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    pub fn blocks(&self) -> &[project_snapshot::ProjectSnapshotDependencyTarget] {
        &self.blocks
    }

    pub fn dependency_references(&self) -> &[project_snapshot::ProjectSnapshotDependencyReference] {
        &self.dependency_references
    }

    pub fn dependency_targets(&self) -> &[project_snapshot::ProjectSnapshotDependencyTarget] {
        &self.dependency_targets
    }

    pub fn ends_at(&self) -> Option<&str> {
        self.ends_at.as_deref()
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.status.as_ref()
    }

    pub fn readme_content(&self) -> &str {
        &self.readme_content
    }

    pub fn readme_version(&self) -> &str {
        &self.readme_version
    }
}

impl OpenProjectEventReferences {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn referenced_task_ids(&self) -> &[String] {
        &self.referenced_task_ids
    }
}

pub fn open_project(path: &Path) -> Result<OpenProjectResult, OpenProjectError> {
    let report = validation::validate(path)?;
    let empty_project_readme_content = String::new();
    let empty_project_readme_version = content_version(&empty_project_readme_content);
    let (project_readme_content, project_readme_version, events, event_references, workflow, tasks) =
        match report.project_root() {
            Some(project_root) => {
                let project_readme_content = read_project_readme(project_root)
                    .map_err(OpenProjectError::LoadProjectReadme)?;
                let project_readme_version = content_version(&project_readme_content);
                let projections = match semantic_projection::load(project_root) {
                    Ok(projections) => projections,
                    Err(_error) if !report.is_valid() => {
                        return Ok(OpenProjectResult {
                            selected_path: path.to_path_buf(),
                            project_root: report.project_root().map(Path::to_path_buf),
                            valid: report.is_valid(),
                            issues: report
                                .issues()
                                .iter()
                                .map(|issue| issue.message().to_string())
                                .collect(),
                            agent_readiness: report
                                .project_root()
                                .map(agent_scaffold::readiness_status)
                                .unwrap_or_else(agent_scaffold::not_ready_status),
                            project_readme_content,
                            project_readme_version,
                            events: Vec::new(),
                            event_references: Vec::new(),
                            workflow: None,
                            tasks: Vec::new(),
                        });
                    }
                    Err(error) => {
                        return Err(OpenProjectError::LoadSemanticProjections(error));
                    }
                };
                let snapshot = projections.snapshot();
                let event_references =
                    load_open_project_event_references(projections.relationships());
                let tasks = load_open_project_tasks(snapshot)?;
                (
                    project_readme_content,
                    project_readme_version,
                    snapshot.events().to_vec(),
                    event_references,
                    Some(projections.workflow().clone()),
                    tasks,
                )
            }
            None => (
                empty_project_readme_content,
                empty_project_readme_version,
                Vec::new(),
                Vec::new(),
                None,
                Vec::new(),
            ),
        };

    Ok(OpenProjectResult {
        selected_path: path.to_path_buf(),
        project_root: report.project_root().map(Path::to_path_buf),
        valid: report.is_valid(),
        issues: report
            .issues()
            .iter()
            .map(|issue| issue.message().to_string())
            .collect(),
        agent_readiness: report
            .project_root()
            .map(agent_scaffold::readiness_status)
            .unwrap_or_else(agent_scaffold::not_ready_status),
        project_readme_content,
        project_readme_version,
        events,
        event_references,
        workflow,
        tasks,
    })
}

fn load_open_project_event_references(
    relationships: &dependency_relationships::DependencyRelationships,
) -> Vec<OpenProjectEventReferences> {
    relationships
        .events()
        .iter()
        .map(|event| {
            let references = event.references();
            let blocker_task_ids = references
                .iter()
                .filter(|reference| {
                    reference.kind() == dependency_relationships::EventReferenceKind::EndsAt
                })
                .map(|reference| reference.task_id().to_string())
                .collect();
            let blocked_task_ids = references
                .iter()
                .filter(|reference| {
                    reference.kind() == dependency_relationships::EventReferenceKind::Dependency
                })
                .map(|reference| reference.task_id().to_string())
                .collect();

            OpenProjectEventReferences {
                id: event.id().to_string(),
                referenced_task_ids: event
                    .deletion_blockers()
                    .iter()
                    .map(|reference| reference.task_id().to_string())
                    .collect(),
                blocker_task_ids,
                blocked_task_ids,
            }
        })
        .collect()
}

fn load_open_project_tasks(
    snapshot: &project_snapshot::ProjectSnapshot,
) -> Result<Vec<OpenProjectTask>, OpenProjectError> {
    Ok(snapshot
        .tasks()
        .iter()
        .map(|task| {
            let readme_content =
                read_task_readme(task.path()).map_err(OpenProjectError::LoadReadme)?;
            let readme_version = content_version(&readme_content);

            Ok::<OpenProjectTask, OpenProjectError>(OpenProjectTask {
                id: task.id().to_string(),
                path: task.path().to_path_buf(),
                project_relative_path: task.project_relative_path().display().to_string(),
                dependencies: task.dependencies().to_vec(),
                blocks: task.blocks().to_vec(),
                dependency_references: task.dependency_references().to_vec(),
                dependency_targets: task.dependency_targets().to_vec(),
                ends_at: task.ends_at().map(str::to_string),
                status: task.status().cloned(),
                readme_content,
                readme_version,
            })
        })
        .collect::<Result<Vec<_>, OpenProjectError>>()?)
}

fn read_task_readme(task_path: &Path) -> Result<String, ReadTaskReadmeError> {
    let readme_path = task_path.join("README.md");
    if !readme_path.is_file() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&readme_path).map_err(|source| ReadTaskReadmeError::Read {
        path: readme_path,
        source,
    })
}

fn read_project_readme(project_root: &Path) -> Result<String, ReadProjectReadmeError> {
    let readme_path = project_root.join("README.md");
    match std::fs::read_to_string(&readme_path) {
        Ok(content) => Ok(content),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(source) => Err(ReadProjectReadmeError::Read {
            path: readme_path,
            source,
        }),
    }
}

fn content_version(content: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}:{}", hash, content.len())
}

#[derive(Debug)]
pub enum ReadTaskReadmeError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ReadTaskReadmeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read task README '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ReadTaskReadmeError {}

#[derive(Debug)]
pub enum ReadProjectReadmeError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for ReadProjectReadmeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read project README '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ReadProjectReadmeError {}

#[derive(Debug)]
pub enum OpenProjectError {
    LoadProjectReadme(ReadProjectReadmeError),
    LoadReadme(ReadTaskReadmeError),
    Validate(validation::ValidateError),
    LoadSnapshot(project_snapshot::LoadProjectSnapshotError),
    LoadRelationships(dependency_relationships::LoadDependencyRelationshipsError),
    LoadWorkflow(event_axis_workflow::LoadEventAxisWorkflowError),
    LoadSemanticProjections(semantic_projection::LoadProjectSemanticProjectionsError),
}

impl std::fmt::Display for OpenProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadProjectReadme(source) => write!(formatter, "{source}"),
            Self::LoadReadme(source) => write!(formatter, "{source}"),
            Self::Validate(source) => write!(formatter, "{source}"),
            Self::LoadSnapshot(source) => write!(formatter, "{source}"),
            Self::LoadRelationships(source) => write!(formatter, "{source}"),
            Self::LoadWorkflow(source) => write!(formatter, "{source}"),
            Self::LoadSemanticProjections(source) => write!(formatter, "{source}"),
        }
    }
}

impl std::error::Error for OpenProjectError {}

impl From<validation::ValidateError> for OpenProjectError {
    fn from(error: validation::ValidateError) -> Self {
        Self::Validate(error)
    }
}

impl From<project_snapshot::LoadProjectSnapshotError> for OpenProjectError {
    fn from(error: project_snapshot::LoadProjectSnapshotError) -> Self {
        Self::LoadSnapshot(error)
    }
}

impl From<dependency_relationships::LoadDependencyRelationshipsError> for OpenProjectError {
    fn from(error: dependency_relationships::LoadDependencyRelationshipsError) -> Self {
        Self::LoadRelationships(error)
    }
}

impl From<event_axis_workflow::LoadEventAxisWorkflowError> for OpenProjectError {
    fn from(error: event_axis_workflow::LoadEventAxisWorkflowError) -> Self {
        Self::LoadWorkflow(error)
    }
}
