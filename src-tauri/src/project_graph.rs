use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::{
    metadata::{MetadataError, ProjectBoundaryEvents, TaskMetadata, TaskStatus},
    project, task_package_index,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraph {
    project_root: PathBuf,
    events: Vec<String>,
    boundary_events: Option<ProjectBoundaryEvents>,
    tasks: Vec<ProjectGraphTask>,
    task_indexes_by_id: HashMap<String, usize>,
    event_ids: HashSet<String>,
    edges: Vec<ProjectGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphTask {
    id: String,
    path: PathBuf,
    project_relative_path: PathBuf,
    metadata: TaskMetadata,
    dependency_references: Vec<ProjectGraphDependencyReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphDependencyReference {
    id: String,
    kind: ProjectGraphDependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphDependencyTarget {
    id: String,
    kind: ProjectGraphDependencyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphEvent {
    id: String,
    boundary_role: ProjectGraphEventBoundaryRole,
    chart_order: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectGraphEventBoundaryRole {
    StartBoundary,
    Ordinary,
    FinishBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectGraphDependencyKind {
    Task,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectGraphNode {
    Task(String),
    Event(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGraphEdge {
    from: ProjectGraphNode,
    to: ProjectGraphNode,
    kind: ProjectGraphEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectGraphEdgeKind {
    Dependency,
    EndsAt,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedProjectGraphTask {
    pub path: PathBuf,
    pub metadata: TaskMetadata,
    pub original_metadata_contents: String,
}

impl ProjectGraph {
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn events(&self) -> &[String] {
        &self.events
    }

    pub fn event_nodes(&self) -> Vec<ProjectGraphEvent> {
        self.events
            .iter()
            .enumerate()
            .map(|(index, event_id)| ProjectGraphEvent {
                id: event_id.clone(),
                boundary_role: self.boundary_role(event_id),
                chart_order: index,
            })
            .collect()
    }

    pub fn tasks(&self) -> &[ProjectGraphTask] {
        &self.tasks
    }

    pub fn task(&self, id: &str) -> Option<&ProjectGraphTask> {
        self.task_indexes_by_id
            .get(id)
            .map(|index| &self.tasks[*index])
    }

    pub fn has_event(&self, id: &str) -> bool {
        self.event_ids.contains(id)
    }

    pub fn edges(&self) -> &[ProjectGraphEdge] {
        &self.edges
    }

    pub fn tasks_ending_at(&self, event_id: &str) -> Vec<&ProjectGraphTask> {
        self.tasks
            .iter()
            .filter(|task| task.ends_at() == Some(event_id))
            .collect()
    }

    pub(crate) fn dependency_edges_by_node(&self) -> HashMap<String, Vec<String>> {
        let mut dependencies_by_node = HashMap::<String, Vec<String>>::new();

        for edge in &self.edges {
            dependencies_by_node
                .entry(edge.from.id().to_string())
                .or_default()
                .push(edge.to.id().to_string());
        }

        dependencies_by_node
    }

    pub(crate) fn dependency_cycle(&self) -> Option<Vec<String>> {
        let dependencies_by_id = self.dependency_edges_by_node();
        let mut states = HashMap::<String, VisitState>::new();
        let mut stack = Vec::<String>::new();

        for task in &self.tasks {
            if let Some(cycle) = visit_node(task.id(), &dependencies_by_id, &mut states, &mut stack)
            {
                return Some(cycle);
            }
        }

        for event in &self.events {
            if let Some(cycle) = visit_node(event, &dependencies_by_id, &mut states, &mut stack) {
                return Some(cycle);
            }
        }

        None
    }

    pub fn valid_dependency_targets(
        &self,
        task_id: &str,
    ) -> Option<Vec<ProjectGraphDependencyTarget>> {
        let task = self.task(task_id)?;
        let existing_dependencies = task
            .dependencies()
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let dependencies_by_node = self.dependency_edges_by_node();
        let mut targets = Vec::new();

        for candidate in &self.tasks {
            if candidate.id() == task.id()
                || existing_dependencies.contains(candidate.id())
                || candidate.ends_at().is_some()
                || dependency_path_exists(candidate.id(), task.id(), &dependencies_by_node)
            {
                continue;
            }

            targets.push(ProjectGraphDependencyTarget {
                id: candidate.id().to_string(),
                kind: ProjectGraphDependencyKind::Task,
            });
        }

        for event_id in &self.events {
            if event_id == task.id()
                || existing_dependencies.contains(event_id.as_str())
                || event_dependency_after_task_end(task, event_id, &self.events)
                || dependency_path_exists(event_id, task.id(), &dependencies_by_node)
            {
                continue;
            }

            targets.push(ProjectGraphDependencyTarget {
                id: event_id.clone(),
                kind: ProjectGraphDependencyKind::Event,
            });
        }

        Some(targets)
    }

    pub fn dependency_would_create_cycle(&self, task_id: &str, blocker_id: &str) -> bool {
        let mut dependencies_by_node = self.dependency_edges_by_node();
        dependencies_by_node
            .entry(task_id.to_string())
            .or_default()
            .push(blocker_id.to_string());
        dependency_path_exists(blocker_id, task_id, &dependencies_by_node)
    }

    fn boundary_role(&self, event_id: &str) -> ProjectGraphEventBoundaryRole {
        match self.boundary_events.as_ref() {
            Some(boundary_events) if boundary_events.start == event_id => {
                ProjectGraphEventBoundaryRole::StartBoundary
            }
            Some(boundary_events) if boundary_events.finish == event_id => {
                ProjectGraphEventBoundaryRole::FinishBoundary
            }
            _ => ProjectGraphEventBoundaryRole::Ordinary,
        }
    }
}

impl ProjectGraphTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn project_relative_path(&self) -> &Path {
        &self.project_relative_path
    }

    pub fn dependencies(&self) -> &[String] {
        &self.metadata.dependencies
    }

    pub fn dependency_references(&self) -> &[ProjectGraphDependencyReference] {
        &self.dependency_references
    }

    pub fn ends_at(&self) -> Option<&str> {
        self.metadata.ends_at.as_deref()
    }

    pub fn status(&self) -> Option<&TaskStatus> {
        self.metadata.status.as_ref()
    }
}

impl ProjectGraphDependencyReference {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ProjectGraphDependencyKind {
        self.kind
    }
}

impl ProjectGraphDependencyTarget {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> ProjectGraphDependencyKind {
        self.kind
    }
}

impl ProjectGraphEvent {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn boundary_role(&self) -> ProjectGraphEventBoundaryRole {
        self.boundary_role
    }

    pub fn chart_order(&self) -> usize {
        self.chart_order
    }
}

impl ProjectGraphNode {
    fn id(&self) -> &str {
        match self {
            Self::Task(id) | Self::Event(id) => id,
        }
    }
}

impl ProjectGraphEdge {
    pub fn from(&self) -> ProjectGraphNode {
        self.from.clone()
    }

    pub fn to(&self) -> ProjectGraphNode {
        self.to.clone()
    }

    pub fn kind(&self) -> ProjectGraphEdgeKind {
        self.kind
    }
}

pub fn load(start: &Path) -> Result<ProjectGraph, LoadProjectGraphError> {
    Ok(task_package_index::TaskPackageIndex::load(start)?
        .graph()
        .clone())
}

pub(crate) fn from_loaded_tasks(
    project_root: PathBuf,
    events: Vec<String>,
    boundary_events: Option<ProjectBoundaryEvents>,
    loaded_tasks: Vec<LoadedProjectGraphTask>,
) -> ProjectGraph {
    let event_ids = events.iter().cloned().collect::<HashSet<_>>();
    let mut tasks = Vec::new();
    let mut edges = Vec::new();

    for loaded_task in loaded_tasks {
        let dependency_references = loaded_task
            .metadata
            .dependencies
            .iter()
            .map(|dependency| {
                let kind = if event_ids.contains(dependency) {
                    ProjectGraphDependencyKind::Event
                } else {
                    ProjectGraphDependencyKind::Task
                };
                ProjectGraphDependencyReference {
                    id: dependency.clone(),
                    kind,
                }
            })
            .collect::<Vec<_>>();

        for dependency in &dependency_references {
            edges.push(ProjectGraphEdge {
                from: ProjectGraphNode::Task(loaded_task.metadata.id.clone()),
                to: match dependency.kind {
                    ProjectGraphDependencyKind::Task => {
                        ProjectGraphNode::Task(dependency.id.clone())
                    }
                    ProjectGraphDependencyKind::Event => {
                        ProjectGraphNode::Event(dependency.id.clone())
                    }
                },
                kind: ProjectGraphEdgeKind::Dependency,
            });
        }

        if let Some(ends_at) = loaded_task.metadata.ends_at.as_deref() {
            if event_ids.contains(ends_at) {
                edges.push(ProjectGraphEdge {
                    from: ProjectGraphNode::Event(ends_at.to_string()),
                    to: ProjectGraphNode::Task(loaded_task.metadata.id.clone()),
                    kind: ProjectGraphEdgeKind::EndsAt,
                });
            }
        }

        tasks.push(ProjectGraphTask {
            id: loaded_task.metadata.id.clone(),
            project_relative_path: display_path_relative_to(&loaded_task.path, &project_root),
            path: loaded_task.path,
            metadata: loaded_task.metadata,
            dependency_references,
        });
    }

    let task_indexes_by_id = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.clone(), index))
        .collect();

    ProjectGraph {
        project_root,
        events,
        boundary_events,
        tasks,
        task_indexes_by_id,
        event_ids,
        edges,
    }
}

fn display_path_relative_to(path: &Path, project_root: &Path) -> PathBuf {
    path.strip_prefix(project_root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn visit_node(
    node_id: &str,
    dependencies_by_id: &HashMap<String, Vec<String>>,
    states: &mut HashMap<String, VisitState>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    match states
        .get(node_id)
        .copied()
        .unwrap_or(VisitState::Unvisited)
    {
        VisitState::Done => return None,
        VisitState::Visiting => {
            let cycle_start = stack
                .iter()
                .position(|entry| entry == node_id)
                .expect("visiting node should exist in DFS stack");
            let mut cycle = stack[cycle_start..].to_vec();
            cycle.push(node_id.to_string());
            return Some(cycle);
        }
        VisitState::Unvisited => {}
    }

    states.insert(node_id.to_string(), VisitState::Visiting);
    stack.push(node_id.to_string());

    if let Some(dependencies) = dependencies_by_id.get(node_id) {
        for dependency in dependencies {
            if !dependencies_by_id.contains_key(dependency) {
                continue;
            }

            if let Some(cycle) = visit_node(dependency, dependencies_by_id, states, stack) {
                return Some(cycle);
            }
        }
    }

    stack.pop();
    states.insert(node_id.to_string(), VisitState::Done);
    None
}

fn dependency_path_exists(
    start_id: &str,
    target_id: &str,
    dependencies_by_id: &HashMap<String, Vec<String>>,
) -> bool {
    let mut stack = vec![start_id.to_string()];
    let mut visited = HashSet::<String>::new();

    while let Some(current_id) = stack.pop() {
        if current_id == target_id {
            return true;
        }

        if !visited.insert(current_id.clone()) {
            continue;
        }

        if let Some(dependencies) = dependencies_by_id.get(&current_id) {
            stack.extend(dependencies.iter().cloned());
        }
    }

    false
}

fn event_dependency_after_task_end(
    task: &ProjectGraphTask,
    event_id: &str,
    events: &[String],
) -> bool {
    let Some(ends_at) = task.ends_at() else {
        return false;
    };
    match (event_index(events, event_id), event_index(events, ends_at)) {
        (Some(dependency_index), Some(ends_at_index)) => dependency_index > ends_at_index,
        _ => false,
    }
}

fn event_index(events: &[String], event_id: &str) -> Option<usize> {
    events.iter().position(|candidate| candidate == event_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Done,
}

#[derive(Debug)]
pub enum LoadProjectGraphError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
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

pub(crate) trait FromLoadProjectGraphError: Sized {
    fn project_not_found(path: PathBuf) -> Self;
    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self;
    fn walk_project(path: PathBuf, source: walkdir::Error) -> Self;
    fn duplicate_task_id(id: String, first_path: PathBuf, second_path: PathBuf) -> Self;
    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self;
    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self;
}

pub(crate) trait FromLoadProjectGraphErrorWithScan: Sized {
    type ScanError: FromLoadProjectGraphError;

    fn project_not_found(path: PathBuf) -> Self;
    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self;
    fn scan_error(source: Self::ScanError) -> Self;
}

pub(crate) trait FromLoadProjectGraphErrorWithScanAndMetadata: Sized {
    type ScanError: FromLoadProjectGraphError;

    fn project_not_found(path: PathBuf) -> Self;
    fn read_project_metadata(source: project::ReadProjectMetadataError) -> Self;
    fn scan_error(source: Self::ScanError) -> Self;
    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self;
    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self;
}

pub(crate) fn map_load_error<T: FromLoadProjectGraphError>(error: LoadProjectGraphError) -> T {
    match error {
        LoadProjectGraphError::ProjectNotFound(path) => T::project_not_found(path),
        LoadProjectGraphError::ReadProjectMetadata(source) => T::read_project_metadata(source),
        LoadProjectGraphError::WalkProject { path, source } => T::walk_project(path, source),
        LoadProjectGraphError::DuplicateTaskId {
            id,
            first_path,
            second_path,
        } => T::duplicate_task_id(id, first_path, second_path),
        LoadProjectGraphError::ReadMetadata { path, source } => T::read_metadata(path, source),
        LoadProjectGraphError::ParseMetadata { path, source } => T::parse_metadata(path, source),
    }
}

pub(crate) fn map_load_error_with_scan<T: FromLoadProjectGraphErrorWithScan>(
    error: LoadProjectGraphError,
) -> T {
    map_load_error_with_scan_path(error, None)
}

pub(crate) fn map_load_error_with_scan_from_start<T: FromLoadProjectGraphErrorWithScan>(
    error: LoadProjectGraphError,
    start: &Path,
) -> T {
    map_load_error_with_scan_path(error, Some(start.to_path_buf()))
}

fn map_load_error_with_scan_path<T: FromLoadProjectGraphErrorWithScan>(
    error: LoadProjectGraphError,
    project_not_found_path: Option<PathBuf>,
) -> T {
    match error {
        LoadProjectGraphError::ProjectNotFound(path) => {
            T::project_not_found(project_not_found_path.unwrap_or(path))
        }
        LoadProjectGraphError::ReadProjectMetadata(source) => T::read_project_metadata(source),
        other => T::scan_error(map_load_error(other)),
    }
}

pub(crate) fn map_load_error_with_scan_and_metadata<
    T: FromLoadProjectGraphErrorWithScanAndMetadata,
>(
    error: LoadProjectGraphError,
) -> T {
    match error {
        LoadProjectGraphError::ProjectNotFound(path) => T::project_not_found(path),
        LoadProjectGraphError::ReadProjectMetadata(source) => T::read_project_metadata(source),
        LoadProjectGraphError::WalkProject { path, source } => {
            T::scan_error(map_load_error(LoadProjectGraphError::WalkProject {
                path,
                source,
            }))
        }
        LoadProjectGraphError::DuplicateTaskId {
            id,
            first_path,
            second_path,
        } => T::scan_error(map_load_error(LoadProjectGraphError::DuplicateTaskId {
            id,
            first_path,
            second_path,
        })),
        LoadProjectGraphError::ReadMetadata { path, source } => T::read_metadata(path, source),
        LoadProjectGraphError::ParseMetadata { path, source } => T::parse_metadata(path, source),
    }
}

impl FromLoadProjectGraphError for LoadProjectGraphError {
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

impl std::fmt::Display for LoadProjectGraphError {
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
            } => write!(
                formatter,
                "duplicate task id '{}' found in '{}' and '{}'",
                id,
                first_path.display(),
                second_path.display()
            ),
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

impl std::error::Error for LoadProjectGraphError {}
