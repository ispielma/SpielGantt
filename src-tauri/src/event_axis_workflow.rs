use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    metadata::{MetadataError, TaskStatus},
    project, project_graph, task,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflow {
    schema_version: u32,
    project_root: String,
    events: Vec<String>,
    event_nodes: Vec<EventAxisWorkflowEvent>,
    tasks: Vec<EventAxisWorkflowTask>,
    edges: Vec<EventAxisWorkflowEdge>,
    validation: EventAxisWorkflowValidation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowEvent {
    id: String,
    boundary_role: EventAxisWorkflowBoundaryRole,
    chart_order: usize,
    placement_ready: bool,
    placement_status: EventAxisWorkflowPlacementStatus,
    placement_messages: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowBoundaryRole {
    StartBoundary,
    Ordinary,
    FinishBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowTask {
    id: String,
    determination_status: EventAxisWorkflowDeterminationStatus,
    placement_ready: bool,
    placement_status: EventAxisWorkflowPlacementStatus,
    placement_messages: Vec<String>,
    dependency_references: Vec<EventAxisWorkflowDependencyReference>,
    ends_at_reference: Option<EventAxisWorkflowEndsAtReference>,
    effective_anchors: EventAxisWorkflowEffectiveAnchors,
    valid_dependency_targets: Vec<EventAxisWorkflowNode>,
    valid_ends_at_targets: Vec<String>,
    unresolved_references: Vec<EventAxisWorkflowReferenceIssue>,
    invalid_references: Vec<EventAxisWorkflowReferenceIssue>,
    validation_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowDeterminationStatus {
    FullyDetermined,
    Undetermined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowPlacementStatus {
    Ready,
    Incomplete,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowDependencyReference {
    id: String,
    kind: EventAxisWorkflowNodeKind,
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowEndsAtReference {
    id: String,
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowEffectiveAnchors {
    upstream: Option<String>,
    downstream: Option<String>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowNode {
    id: String,
    kind: EventAxisWorkflowNodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowNodeKind {
    Task,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowEdge {
    from: EventAxisWorkflowNode,
    to: EventAxisWorkflowNode,
    kind: EventAxisWorkflowEdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowEdgeKind {
    Dependency,
    EndsAt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowReferenceIssue {
    id: String,
    kind: EventAxisWorkflowReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAxisWorkflowReferenceKind {
    Dependency,
    EndsAt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventAxisWorkflowValidation {
    valid: bool,
    diagnostics: Vec<String>,
}

impl EventAxisWorkflow {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn events(&self) -> &[String] {
        &self.events
    }

    pub fn event_nodes(&self) -> &[EventAxisWorkflowEvent] {
        &self.event_nodes
    }

    pub fn tasks(&self) -> &[EventAxisWorkflowTask] {
        &self.tasks
    }

    pub fn validation(&self) -> &EventAxisWorkflowValidation {
        &self.validation
    }
}

impl EventAxisWorkflowTask {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn validation_diagnostics(&self) -> &[String] {
        &self.validation_diagnostics
    }
}

impl EventAxisWorkflowValidation {
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }
}

pub fn load(start: &Path) -> Result<EventAxisWorkflow, LoadEventAxisWorkflowError> {
    let graph =
        project_graph::load(start).map_err(project_graph::map_load_error_with_scan_and_metadata)?;
    Ok(from_graph(&graph))
}

pub fn from_graph(graph: &project_graph::ProjectGraph) -> EventAxisWorkflow {
    let mut validation_diagnostics = Vec::new();
    let mut tasks = Vec::new();
    let effective_anchor_context = EffectiveAnchorContext::new(graph);

    for graph_task in graph.tasks() {
        let mut task_diagnostics = Vec::new();
        let mut unresolved_references = Vec::new();
        let mut invalid_references = Vec::new();

        let dependency_references = graph_task
            .dependency_references()
            .iter()
            .map(|dependency| {
                let diagnostic = dependency_diagnostic(&graph, graph_task.id(), dependency.id());
                if let Some(diagnostic) = diagnostic.as_ref() {
                    task_diagnostics.push(diagnostic.clone());
                    invalid_references.push(EventAxisWorkflowReferenceIssue {
                        id: dependency.id().to_string(),
                        kind: EventAxisWorkflowReferenceKind::Dependency,
                    });
                    if graph.task(dependency.id()).is_none() && !graph.has_event(dependency.id()) {
                        unresolved_references.push(EventAxisWorkflowReferenceIssue {
                            id: dependency.id().to_string(),
                            kind: EventAxisWorkflowReferenceKind::Dependency,
                        });
                    }
                }

                EventAxisWorkflowDependencyReference {
                    id: dependency.id().to_string(),
                    kind: node_kind_from_graph(dependency.kind()),
                    valid: diagnostic.is_none(),
                    diagnostic,
                }
            })
            .collect::<Vec<_>>();

        let ends_at_reference = graph_task.ends_at().map(|ends_at| {
            let diagnostic = ends_at_diagnostic(&graph, graph_task.id(), ends_at);
            if let Some(diagnostic) = diagnostic.as_ref() {
                task_diagnostics.push(diagnostic.clone());
                invalid_references.push(EventAxisWorkflowReferenceIssue {
                    id: ends_at.to_string(),
                    kind: EventAxisWorkflowReferenceKind::EndsAt,
                });
                if !graph.has_event(ends_at) {
                    unresolved_references.push(EventAxisWorkflowReferenceIssue {
                        id: ends_at.to_string(),
                        kind: EventAxisWorkflowReferenceKind::EndsAt,
                    });
                }
            }

            EventAxisWorkflowEndsAtReference {
                id: ends_at.to_string(),
                valid: diagnostic.is_none(),
                diagnostic,
            }
        });

        task_diagnostics.extend(event_span_diagnostics(&graph, graph_task));
        task_diagnostics.extend(status_diagnostics(&graph, graph_task));
        let effective_anchors =
            effective_anchor_context.effective_anchors(graph_task, &task_diagnostics);
        validation_diagnostics.extend(task_diagnostics.iter().cloned());

        let valid_dependency_targets = graph
            .valid_dependency_targets(graph_task.id())
            .unwrap_or_default()
            .into_iter()
            .map(|target| EventAxisWorkflowNode {
                id: target.id().to_string(),
                kind: node_kind_from_graph(target.kind()),
            })
            .collect::<Vec<_>>();

        let placement_messages = unique_messages(&effective_anchors.diagnostics);
        let placement_status =
            placement_status(&effective_anchors, &task_diagnostics, &placement_messages);
        tasks.push(EventAxisWorkflowTask {
            id: graph_task.id().to_string(),
            determination_status: determination_status(&effective_anchors),
            placement_ready: placement_status == EventAxisWorkflowPlacementStatus::Ready,
            placement_status,
            placement_messages,
            dependency_references,
            ends_at_reference,
            effective_anchors,
            valid_dependency_targets,
            valid_ends_at_targets: valid_ends_at_targets(graph, graph_task),
            unresolved_references,
            invalid_references,
            validation_diagnostics: task_diagnostics,
        });
    }

    if let Some(cycle) = graph.dependency_cycle() {
        validation_diagnostics.push(format!("dependency cycle detected: {}", cycle.join(" -> ")));
    }

    let mut edges = graph
        .edges()
        .iter()
        .map(|edge| EventAxisWorkflowEdge {
            from: workflow_node(edge.from()),
            to: workflow_node(edge.to()),
            kind: match edge.kind() {
                project_graph::ProjectGraphEdgeKind::Dependency => {
                    EventAxisWorkflowEdgeKind::Dependency
                }
                project_graph::ProjectGraphEdgeKind::EndsAt => EventAxisWorkflowEdgeKind::EndsAt,
            },
        })
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.from
            .id
            .cmp(&right.from.id)
            .then_with(|| left.to.id.cmp(&right.to.id))
            .then_with(|| edge_kind_label(left.kind).cmp(edge_kind_label(right.kind)))
    });

    EventAxisWorkflow {
        schema_version: 1,
        project_root: graph.project_root().display().to_string(),
        events: graph.events().to_vec(),
        event_nodes: graph
            .event_nodes()
            .iter()
            .map(|event| workflow_event(event, &tasks))
            .collect::<Vec<_>>(),
        tasks,
        edges,
        validation: EventAxisWorkflowValidation {
            valid: validation_diagnostics.is_empty(),
            diagnostics: validation_diagnostics,
        },
    }
}

fn determination_status(
    anchors: &EventAxisWorkflowEffectiveAnchors,
) -> EventAxisWorkflowDeterminationStatus {
    if anchors.upstream.is_some() && anchors.downstream.is_some() {
        EventAxisWorkflowDeterminationStatus::FullyDetermined
    } else {
        EventAxisWorkflowDeterminationStatus::Undetermined
    }
}

fn placement_status(
    anchors: &EventAxisWorkflowEffectiveAnchors,
    validation_diagnostics: &[String],
    placement_messages: &[String],
) -> EventAxisWorkflowPlacementStatus {
    if !validation_diagnostics.is_empty() {
        return EventAxisWorkflowPlacementStatus::Diagnostic;
    }

    if anchors.upstream.is_some() && anchors.downstream.is_some() {
        return if placement_messages.is_empty() {
            EventAxisWorkflowPlacementStatus::Ready
        } else {
            EventAxisWorkflowPlacementStatus::Diagnostic
        };
    }

    EventAxisWorkflowPlacementStatus::Incomplete
}

fn workflow_event(
    event: &project_graph::ProjectGraphEvent,
    tasks: &[EventAxisWorkflowTask],
) -> EventAxisWorkflowEvent {
    let boundary_role = match event.boundary_role() {
        project_graph::ProjectGraphEventBoundaryRole::StartBoundary => {
            EventAxisWorkflowBoundaryRole::StartBoundary
        }
        project_graph::ProjectGraphEventBoundaryRole::Ordinary => {
            EventAxisWorkflowBoundaryRole::Ordinary
        }
        project_graph::ProjectGraphEventBoundaryRole::FinishBoundary => {
            EventAxisWorkflowBoundaryRole::FinishBoundary
        }
    };
    let placement_messages = event_placement_messages(event.id(), boundary_role, tasks);
    let placement_status = if placement_messages.is_empty() {
        EventAxisWorkflowPlacementStatus::Ready
    } else {
        EventAxisWorkflowPlacementStatus::Incomplete
    };

    EventAxisWorkflowEvent {
        id: event.id().to_string(),
        boundary_role,
        chart_order: event.chart_order(),
        placement_ready: placement_status == EventAxisWorkflowPlacementStatus::Ready,
        placement_status,
        placement_messages,
    }
}

fn event_placement_messages(
    event_id: &str,
    boundary_role: EventAxisWorkflowBoundaryRole,
    tasks: &[EventAxisWorkflowTask],
) -> Vec<String> {
    if boundary_role != EventAxisWorkflowBoundaryRole::Ordinary {
        return Vec::new();
    }

    let has_incoming_task = tasks
        .iter()
        .any(|task| task.effective_anchors.downstream.as_deref() == Some(event_id));
    let has_outgoing_task = tasks
        .iter()
        .any(|task| task.effective_anchors.upstream.as_deref() == Some(event_id));
    let mut messages = Vec::new();

    if !has_incoming_task {
        messages.push(format!(
            "event '{event_id}' has no task placement ending at this event"
        ));
    }
    if !has_outgoing_task {
        messages.push(format!(
            "event '{event_id}' has no task placement starting after this event"
        ));
    }

    messages
}

fn unique_messages(messages: &[String]) -> Vec<String> {
    let mut seen_messages = HashSet::new();
    let mut unique = Vec::new();
    for message in messages {
        if seen_messages.insert(message.clone()) {
            unique.push(message.clone());
        }
    }
    unique
}

fn dependency_diagnostic(
    graph: &project_graph::ProjectGraph,
    task_id: &str,
    dependency_id: &str,
) -> Option<String> {
    if let Some(dependency_ends_at) = graph.task(dependency_id).and_then(|task| task.ends_at()) {
        return Some(format!(
            "task '{task_id}' depends on task '{dependency_id}' which ends at event '{dependency_ends_at}'; depend on the event instead"
        ));
    }

    if graph.task(dependency_id).is_none() && !graph.has_event(dependency_id) {
        return Some(format!(
            "task '{task_id}' depends on missing task or event id '{dependency_id}'"
        ));
    }

    None
}

fn event_span_diagnostics(
    graph: &project_graph::ProjectGraph,
    task: &project_graph::ProjectGraphTask,
) -> Vec<String> {
    let Some(ends_at) = task.ends_at() else {
        return Vec::new();
    };
    let Some(ends_at_index) = event_index(graph, ends_at) else {
        return Vec::new();
    };

    task.dependency_references()
        .iter()
        .filter(|dependency| {
            dependency.kind() == project_graph::ProjectGraphDependencyKind::Event
                && event_index(graph, dependency.id())
                    .is_some_and(|dependency_index| dependency_index > ends_at_index)
        })
        .map(|dependency| {
            format!(
                "task '{}' depends on event '{}' after ends_at event '{}'",
                task.id(),
                dependency.id(),
                ends_at
            )
        })
        .collect()
}

fn status_diagnostics(
    graph: &project_graph::ProjectGraph,
    task: &project_graph::ProjectGraphTask,
) -> Vec<String> {
    if task.status() != Some(&TaskStatus::Done) {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    for dependency in task.dependency_references() {
        match dependency.kind() {
            project_graph::ProjectGraphDependencyKind::Task => {
                if graph
                    .task(dependency.id())
                    .and_then(|blocked_task| blocked_task.status())
                    == Some(&TaskStatus::Blocked)
                {
                    diagnostics.push(format!(
                        "task '{}' is done but depends on blocked task '{}'",
                        task.id(),
                        dependency.id()
                    ));
                }
            }
            project_graph::ProjectGraphDependencyKind::Event => {
                for blocked_task in graph.tasks_ending_at(dependency.id()) {
                    if blocked_task.status() == Some(&TaskStatus::Blocked) {
                        diagnostics.push(format!(
                            "task '{}' is done but depends on event '{}' reached by blocked task '{}'",
                            task.id(),
                            dependency.id(),
                            blocked_task.id()
                        ));
                    }
                }
            }
        }
    }

    diagnostics
}

fn valid_ends_at_targets(
    graph: &project_graph::ProjectGraph,
    task: &project_graph::ProjectGraphTask,
) -> Vec<String> {
    let earliest_allowed_index = task
        .dependency_references()
        .iter()
        .filter(|dependency| dependency.kind() == project_graph::ProjectGraphDependencyKind::Event)
        .filter_map(|dependency| event_index(graph, dependency.id()))
        .max()
        .unwrap_or(0);

    graph
        .events()
        .iter()
        .enumerate()
        .filter(|(index, _)| *index >= earliest_allowed_index)
        .map(|(_, event_id)| event_id.clone())
        .collect()
}

struct EffectiveAnchorContext<'graph> {
    graph: &'graph project_graph::ProjectGraph,
    event_index_by_id: HashMap<String, usize>,
    downstream_task_ids_by_task_id: HashMap<String, Vec<String>>,
}

impl<'graph> EffectiveAnchorContext<'graph> {
    fn new(graph: &'graph project_graph::ProjectGraph) -> Self {
        let event_index_by_id = graph
            .events()
            .iter()
            .enumerate()
            .map(|(index, event_id)| (event_id.clone(), index))
            .collect::<HashMap<_, _>>();
        let mut downstream_task_ids_by_task_id = HashMap::<String, Vec<String>>::new();

        for task in graph.tasks() {
            for dependency_id in valid_task_dependency_ids(graph, task) {
                downstream_task_ids_by_task_id
                    .entry(dependency_id)
                    .or_default()
                    .push(task.id().to_string());
            }
        }

        Self {
            graph,
            event_index_by_id,
            downstream_task_ids_by_task_id,
        }
    }

    fn effective_anchors(
        &self,
        task: &project_graph::ProjectGraphTask,
        validation_diagnostics: &[String],
    ) -> EventAxisWorkflowEffectiveAnchors {
        let upstream =
            self.latest_event_id(self.upstream_event_ids(task.id(), &mut HashSet::new()));
        let downstream =
            valid_ends_at_id(self.graph, task, &self.event_index_by_id).or_else(|| {
                self.earliest_event_id(
                    self.downstream_end_event_ids(task.id(), &mut HashSet::new()),
                )
            });
        let mut diagnostics = validation_diagnostics.to_vec();

        match (
            upstream
                .as_ref()
                .and_then(|event_id| self.event_index_by_id.get(event_id)),
            downstream
                .as_ref()
                .and_then(|event_id| self.event_index_by_id.get(event_id)),
        ) {
            (None, None) => diagnostics.push(format!(
                "task '{}' cannot be placed on the event axis without upstream or downstream event anchors",
                task.id()
            )),
            (None, Some(_)) => diagnostics.push(format!(
                "task '{}' cannot be placed on the event axis without an upstream event anchor",
                task.id()
            )),
            (Some(_), None) => diagnostics.push(format!(
                "task '{}' cannot be placed on the event axis without a downstream event anchor",
                task.id()
            )),
            (Some(upstream_index), Some(downstream_index)) if upstream_index == downstream_index => {
                diagnostics.push(format!(
                    "task '{}' cannot be placed on the event axis because upstream and downstream event anchors are both '{}'",
                    task.id(),
                    upstream.as_deref().unwrap_or("unknown")
                ));
            }
            _ => {}
        }

        EventAxisWorkflowEffectiveAnchors {
            upstream,
            downstream,
            diagnostics,
        }
    }

    fn upstream_event_ids(
        &self,
        task_id: &str,
        seen_task_ids: &mut HashSet<String>,
    ) -> Vec<String> {
        if !seen_task_ids.insert(task_id.to_string()) {
            return Vec::new();
        }

        let Some(task) = self.graph.task(task_id) else {
            return Vec::new();
        };

        let mut event_ids = valid_event_dependency_ids(task, &self.event_index_by_id);
        for dependency_id in valid_task_dependency_ids(self.graph, task) {
            event_ids.extend(self.upstream_event_ids(&dependency_id, seen_task_ids));
        }
        event_ids
    }

    fn downstream_end_event_ids(
        &self,
        task_id: &str,
        seen_task_ids: &mut HashSet<String>,
    ) -> Vec<String> {
        if !seen_task_ids.insert(task_id.to_string()) {
            return Vec::new();
        }

        let mut event_ids = Vec::new();
        for downstream_task_id in self
            .downstream_task_ids_by_task_id
            .get(task_id)
            .into_iter()
            .flatten()
        {
            if let Some(downstream_task) = self.graph.task(downstream_task_id) {
                if let Some(event_id) =
                    valid_ends_at_id(self.graph, downstream_task, &self.event_index_by_id)
                {
                    event_ids.push(event_id);
                }
            }
            event_ids.extend(self.downstream_end_event_ids(downstream_task_id, seen_task_ids));
        }
        event_ids
    }

    fn latest_event_id(&self, event_ids: Vec<String>) -> Option<String> {
        event_ids.into_iter().max_by_key(|event_id| {
            self.event_index_by_id
                .get(event_id)
                .copied()
                .unwrap_or_default()
        })
    }

    fn earliest_event_id(&self, event_ids: Vec<String>) -> Option<String> {
        event_ids.into_iter().min_by_key(|event_id| {
            self.event_index_by_id
                .get(event_id)
                .copied()
                .unwrap_or(usize::MAX)
        })
    }
}

fn valid_event_dependency_ids(
    task: &project_graph::ProjectGraphTask,
    event_index_by_id: &HashMap<String, usize>,
) -> Vec<String> {
    task.dependency_references()
        .iter()
        .filter(|dependency| {
            dependency.kind() == project_graph::ProjectGraphDependencyKind::Event
                && event_index_by_id.contains_key(dependency.id())
        })
        .map(|dependency| dependency.id().to_string())
        .collect()
}

fn valid_task_dependency_ids(
    graph: &project_graph::ProjectGraph,
    task: &project_graph::ProjectGraphTask,
) -> Vec<String> {
    task.dependency_references()
        .iter()
        .filter(|dependency| {
            dependency.kind() == project_graph::ProjectGraphDependencyKind::Task
                && dependency_diagnostic(graph, task.id(), dependency.id()).is_none()
        })
        .map(|dependency| dependency.id().to_string())
        .collect()
}

fn valid_ends_at_id(
    graph: &project_graph::ProjectGraph,
    task: &project_graph::ProjectGraphTask,
    event_index_by_id: &HashMap<String, usize>,
) -> Option<String> {
    let ends_at = task.ends_at()?;
    if ends_at_diagnostic(graph, task.id(), ends_at).is_none()
        && event_index_by_id.contains_key(ends_at)
    {
        Some(ends_at.to_string())
    } else {
        None
    }
}

fn event_index(graph: &project_graph::ProjectGraph, event_id: &str) -> Option<usize> {
    graph.events().iter().position(|event| event == event_id)
}

fn ends_at_diagnostic(
    graph: &project_graph::ProjectGraph,
    task_id: &str,
    ends_at: &str,
) -> Option<String> {
    if graph.task(ends_at).is_some() {
        return Some(format!(
            "task '{task_id}' ends_at must reference an event id, not task id '{ends_at}'"
        ));
    }

    if !graph.has_event(ends_at) {
        return Some(format!(
            "task '{task_id}' ends_at references missing event id '{ends_at}'"
        ));
    }

    None
}

fn workflow_node(node: project_graph::ProjectGraphNode) -> EventAxisWorkflowNode {
    match node {
        project_graph::ProjectGraphNode::Task(id) => EventAxisWorkflowNode {
            id,
            kind: EventAxisWorkflowNodeKind::Task,
        },
        project_graph::ProjectGraphNode::Event(id) => EventAxisWorkflowNode {
            id,
            kind: EventAxisWorkflowNodeKind::Event,
        },
    }
}

fn node_kind_from_graph(
    kind: project_graph::ProjectGraphDependencyKind,
) -> EventAxisWorkflowNodeKind {
    match kind {
        project_graph::ProjectGraphDependencyKind::Task => EventAxisWorkflowNodeKind::Task,
        project_graph::ProjectGraphDependencyKind::Event => EventAxisWorkflowNodeKind::Event,
    }
}

fn edge_kind_label(kind: EventAxisWorkflowEdgeKind) -> &'static str {
    match kind {
        EventAxisWorkflowEdgeKind::Dependency => "dependency",
        EventAxisWorkflowEdgeKind::EndsAt => "ends_at",
    }
}

#[derive(Debug)]
pub enum LoadEventAxisWorkflowError {
    ProjectNotFound(PathBuf),
    ReadProjectMetadata(project::ReadProjectMetadataError),
    ScanTasks(task::ScanTasksError),
    ReadMetadata {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseMetadata {
        path: PathBuf,
        source: MetadataError,
    },
}

impl std::fmt::Display for LoadEventAxisWorkflowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectNotFound(path) => {
                write!(
                    formatter,
                    "no SpielGantt project found at or above '{}'",
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

impl std::error::Error for LoadEventAxisWorkflowError {}

impl project_graph::FromLoadProjectGraphErrorWithScanAndMetadata for LoadEventAxisWorkflowError {
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

    fn read_metadata(path: PathBuf, source: std::io::Error) -> Self {
        Self::ReadMetadata { path, source }
    }

    fn parse_metadata(path: PathBuf, source: MetadataError) -> Self {
        Self::ParseMetadata { path, source }
    }
}
