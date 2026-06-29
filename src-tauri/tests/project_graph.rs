use std::fs;

use spielgantt_lib::project_graph::{
    self, ProjectGraphDependencyKind, ProjectGraphEdgeKind, ProjectGraphNode,
};
use tempfile::tempdir;

mod support;

#[test]
fn project_graph_loads_typed_dependencies_and_ends_at_edges() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading a graph: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare task metadata should be rewritten with event references");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-sample\",\n    \"START\"\n  ]\n}\n",
    )
    .expect("analyze task metadata should be rewritten with mixed dependencies");

    let graph = project_graph::load(&project_dir).expect("project graph should load");

    assert_eq!(graph.events(), &["START".to_string(), "BEC".to_string()]);
    assert_eq!(
        graph
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        ["analyze-results", "prepare-sample"]
    );

    let analyze_results = graph
        .task("analyze-results")
        .expect("graph should expose analyze-results");
    assert_eq!(
        analyze_results
            .dependency_references()
            .iter()
            .map(|reference| (reference.id(), reference.kind()))
            .collect::<Vec<_>>(),
        vec![
            ("prepare-sample", ProjectGraphDependencyKind::Task),
            ("START", ProjectGraphDependencyKind::Event),
        ]
    );

    let edges = graph
        .edges()
        .iter()
        .map(|edge| (edge.from(), edge.to(), edge.kind()))
        .collect::<Vec<_>>();
    assert!(
        edges.contains(&(
            ProjectGraphNode::Task("analyze-results".to_string()),
            ProjectGraphNode::Task("prepare-sample".to_string()),
            ProjectGraphEdgeKind::Dependency,
        )),
        "graph should include task dependency edges: {edges:?}"
    );
    assert!(
        edges.contains(&(
            ProjectGraphNode::Task("analyze-results".to_string()),
            ProjectGraphNode::Event("START".to_string()),
            ProjectGraphEdgeKind::Dependency,
        )),
        "graph should include event dependency edges: {edges:?}"
    );
    assert!(
        edges.contains(&(
            ProjectGraphNode::Event("BEC".to_string()),
            ProjectGraphNode::Task("prepare-sample".to_string()),
            ProjectGraphEdgeKind::EndsAt,
        )),
        "graph should include ends_at edges in dependency traversal direction: {edges:?}"
    );
}

#[test]
fn project_graph_rejects_malformed_task_metadata_with_metadata_path_context() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading a graph: {init_output:?}"
    );

    let task_metadata_path = project_dir
        .join("broken-task")
        .join(".spielgantt/task.json");
    support::write_task_metadata(&project_dir.join("broken-task"), "broken-task");
    fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": [\n    \"not-a-string\"\n  ]\n}\n",
    )
    .expect("malformed task metadata should be written");

    let error = project_graph::load(&project_dir)
        .expect_err("strict graph load should reject bad metadata");
    let rendered_error = error.to_string();

    let canonical_task_metadata_path =
        fs::canonicalize(&task_metadata_path).expect("task metadata path should canonicalize");
    match error {
        project_graph::LoadProjectGraphError::ParseMetadata { path, .. } => {
            assert_eq!(path, canonical_task_metadata_path);
        }
        other => panic!("unexpected graph load error: {other}"),
    }
    assert!(
        rendered_error.contains("failed to parse task metadata"),
        "strict graph load should explain the parse failure: {rendered_error}"
    );
    assert!(
        rendered_error.contains(&canonical_task_metadata_path.display().to_string()),
        "strict graph load should include the metadata path: {rendered_error}"
    );
}

#[test]
fn project_graph_computes_valid_dependency_targets_for_tasks_and_events() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading a graph: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("analyze task metadata should be rewritten with an existing event dependency");

    support::write_task_metadata(&project_dir.join("calibrate-laser"), "calibrate-laser");
    support::write_task_metadata(&project_dir.join("prepare-samples"), "prepare-samples");
    fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"analyze-results\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare task metadata should be rewritten with ends_at and dependency");

    support::write_task_metadata(&project_dir.join("screen-data"), "screen-data");
    fs::write(
        project_dir
            .join("screen-data")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"analyze-results\"\n  ]\n}\n",
    )
    .expect("screen task metadata should be rewritten with a dependency");

    let graph = project_graph::load(&project_dir).expect("project graph should load");
    let analyze_targets = graph
        .valid_dependency_targets("analyze-results")
        .expect("analyze-results should be present");

    assert_eq!(
        analyze_targets
            .iter()
            .map(|target| (target.id(), target.kind()))
            .collect::<Vec<_>>(),
        vec![
            ("calibrate-laser", ProjectGraphDependencyKind::Task),
            ("MOT", ProjectGraphDependencyKind::Event),
        ],
        "valid dependency targets should exclude self, existing blockers, tasks ending at events, and cycle-causing task/event targets"
    );
    assert!(
        graph.dependency_would_create_cycle("analyze-results", "screen-data"),
        "shared graph should detect proposed task dependency cycles"
    );
    assert!(
        graph.dependency_would_create_cycle("analyze-results", "BEC"),
        "shared graph should detect proposed cycles that traverse ends_at event edges"
    );
    assert!(
        !graph.dependency_would_create_cycle("analyze-results", "MOT"),
        "shared graph should allow proposed event dependencies that do not create cycles"
    );
}

#[test]
fn project_graph_allows_plain_task_as_blocker_for_task_that_ends_at_event() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading a graph: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"Workflow started\",\n    \"Protocol selected\",\n    \"Later analysis\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("make chart"), "make chart");
    fs::write(
        project_dir.join("make chart").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"make chart\",\n  \"dependencies\": [\n    \"Workflow started\"\n  ]\n}\n",
    )
    .expect("new task metadata should be rewritten with an event blocker");

    support::write_task_metadata(&project_dir.join("literature-review"), "literature-review");
    fs::write(
        project_dir
            .join("literature-review")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"literature-review\",\n  \"ends_at\": \"Protocol selected\"\n}\n",
    )
    .expect("selected task metadata should be rewritten with an end event");

    let graph = project_graph::load(&project_dir).expect("project graph should load");
    let literature_targets = graph
        .valid_dependency_targets("literature-review")
        .expect("literature-review should be present");

    assert!(
        literature_targets.iter().any(|target| {
            target.id() == "make chart" && target.kind() == ProjectGraphDependencyKind::Task
        }),
        "a plain task with an event blocker should be offered as a blocker for an ends_at task: {literature_targets:?}"
    );
    assert!(
        literature_targets.iter().all(|target| target.id() != "Later analysis"),
        "backend valid dependency targets should not offer event blockers after ends_at: {literature_targets:?}"
    );
}
