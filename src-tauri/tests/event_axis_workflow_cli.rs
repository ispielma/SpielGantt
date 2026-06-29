use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, run_spielgantt_json_in, stdout_text as stdout};

#[test]
fn task_workflow_json_exposes_durable_boundary_event_roles() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying boundary semantics: {init_output:?}"
    );

    let initial_workflow = run_spielgantt_json_in(&project_dir, &["task", "workflow", "--json"]);
    assert_eq!(
        initial_workflow["event_nodes"],
        serde_json::json!([
            {
                "id": "start",
                "boundary_role": "start_boundary",
                "chart_order": 0,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            },
            {
                "id": "finished",
                "boundary_role": "finish_boundary",
                "chart_order": 1,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            }
        ]),
        "new projects should expose boundary roles without frontend display-name inference"
    );

    let rename_start_output =
        run_spielgantt_in(&project_dir, &["event", "rename", "start", "Kickoff"]);
    assert!(
        rename_start_output.status.success(),
        "start boundary rename should succeed: {rename_start_output:?}"
    );
    let rename_finish_output =
        run_spielgantt_in(&project_dir, &["event", "rename", "finished", "Released"]);
    assert!(
        rename_finish_output.status.success(),
        "finish boundary rename should succeed: {rename_finish_output:?}"
    );

    let renamed_workflow = run_spielgantt_json_in(&project_dir, &["task", "workflow", "--json"]);
    assert_eq!(
        renamed_workflow["event_nodes"],
        serde_json::json!([
            {
                "id": "Kickoff",
                "boundary_role": "start_boundary",
                "chart_order": 0,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            },
            {
                "id": "Released",
                "boundary_role": "finish_boundary",
                "chart_order": 1,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            }
        ]),
        "renaming either boundary event should preserve its package-domain role"
    );

    let legacy_project_dir = workspace_dir.path().join("legacy-project");
    fs::create_dir(&legacy_project_dir).expect("legacy project directory should be created");
    support::write_project_metadata(
        &legacy_project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"LEGACY-START\",\n    \"middle\",\n    \"LEGACY-FINISH\"\n  ]\n}\n",
    );

    let legacy_workflow =
        run_spielgantt_json_in(&legacy_project_dir, &["task", "workflow", "--json"]);
    assert_eq!(
        legacy_workflow["event_nodes"],
        serde_json::json!([
            {
                "id": "LEGACY-START",
                "boundary_role": "start_boundary",
                "chart_order": 0,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            },
            {
                "id": "middle",
                "boundary_role": "ordinary",
                "chart_order": 1,
                "placement_ready": false,
                "placement_status": "incomplete",
                "placement_messages": [
                    "event 'middle' has no task placement ending at this event",
                    "event 'middle' has no task placement starting after this event"
                ]
            },
            {
                "id": "LEGACY-FINISH",
                "boundary_role": "finish_boundary",
                "chart_order": 2,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            }
        ]),
        "legacy string event lists should load with first/last boundary interpretation"
    );
}

#[test]
fn task_workflow_json_reports_event_axis_semantics_without_visual_layout() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying workflow semantics: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\",\n    \"END\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-samples"), "prepare-samples");
    fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare task metadata should be rewritten with event references");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-samples\",\n    \"MISSING-BLOCKER\"\n  ],\n  \"ends_at\": \"MISSING-EVENT\"\n}\n",
    )
    .expect("analyze task metadata should be rewritten with invalid references");

    support::write_task_metadata(&project_dir.join("screen-data"), "screen-data");
    fs::write(
        project_dir.join("screen-data").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"screen-data\",\n  \"dependencies\": [\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("screen task metadata should be rewritten with an event dependency");

    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow --json should succeed even when workflow references are invalid: {workflow_output:?}"
    );

    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("stdout should be JSON");

    assert_eq!(workflow["schema_version"], 1);
    assert_eq!(
        workflow["events"],
        serde_json::json!(["START", "BEC", "END"])
    );
    let edges = workflow["edges"]
        .as_array()
        .expect("edges should be an array");
    assert_eq!(
        edges.len(),
        5,
        "workflow should expose each domain edge once"
    );
    for expected_edge in [
        serde_json::json!({
            "from": {"id": "analyze-results", "kind": "task"},
            "to": {"id": "prepare-samples", "kind": "task"},
            "kind": "dependency"
        }),
        serde_json::json!({
            "from": {"id": "analyze-results", "kind": "task"},
            "to": {"id": "MISSING-BLOCKER", "kind": "task"},
            "kind": "dependency"
        }),
        serde_json::json!({
            "from": {"id": "BEC", "kind": "event"},
            "to": {"id": "prepare-samples", "kind": "task"},
            "kind": "ends_at"
        }),
        serde_json::json!({
            "from": {"id": "prepare-samples", "kind": "task"},
            "to": {"id": "START", "kind": "event"},
            "kind": "dependency"
        }),
        serde_json::json!({
            "from": {"id": "screen-data", "kind": "task"},
            "to": {"id": "BEC", "kind": "event"},
            "kind": "dependency"
        }),
    ] {
        assert!(
            edges.contains(&expected_edge),
            "workflow JSON should expose task/event dependency edge {expected_edge}"
        );
    }

    let analyze_results = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("analyze-results should be included");
    assert_eq!(
        analyze_results["dependency_references"],
        serde_json::json!([
            {
                "id": "prepare-samples",
                "kind": "task",
                "valid": false,
                "diagnostic": "task 'analyze-results' depends on task 'prepare-samples' which ends at event 'BEC'; depend on the event instead"
            },
            {
                "id": "MISSING-BLOCKER",
                "kind": "task",
                "valid": false,
                "diagnostic": "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'"
            }
        ]),
        "dependency validity should come from Rust-owned domain JSON"
    );
    assert_eq!(
        analyze_results["ends_at_reference"],
        serde_json::json!({
            "id": "MISSING-EVENT",
            "valid": false,
            "diagnostic": "task 'analyze-results' ends_at references missing event id 'MISSING-EVENT'"
        }),
        "ends_at reference validity should come from Rust-owned domain JSON"
    );
    assert_eq!(
        analyze_results["unresolved_references"],
        serde_json::json!([
            {"id": "MISSING-BLOCKER", "kind": "dependency"},
            {"id": "MISSING-EVENT", "kind": "ends_at"}
        ]),
        "unresolved references should not require frontend scanning"
    );
    assert_eq!(
        analyze_results["invalid_references"],
        serde_json::json!([
            {"id": "prepare-samples", "kind": "dependency"},
            {"id": "MISSING-BLOCKER", "kind": "dependency"},
            {"id": "MISSING-EVENT", "kind": "ends_at"}
        ]),
        "invalid references should be explicit domain data"
    );
    assert_eq!(
        analyze_results["validation_diagnostics"],
        serde_json::json!([
            "task 'analyze-results' depends on task 'prepare-samples' which ends at event 'BEC'; depend on the event instead",
            "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'",
            "task 'analyze-results' ends_at references missing event id 'MISSING-EVENT'"
        ]),
        "task-level validation diagnostics should be included"
    );
    assert!(
        analyze_results["valid_dependency_targets"]
            .as_array()
            .expect("valid dependency targets should be an array")
            .iter()
            .any(|target| target == &serde_json::json!({"id": "END", "kind": "event"})),
        "valid event dependency targets should be computed by Rust"
    );
    assert!(
        analyze_results["valid_dependency_targets"]
            .as_array()
            .expect("valid dependency targets should be an array")
            .iter()
            .any(|target| target == &serde_json::json!({"id": "screen-data", "kind": "task"})),
        "valid task dependency targets should be computed by Rust"
    );
    assert_eq!(
        analyze_results["valid_ends_at_targets"],
        serde_json::json!(["START", "BEC", "END"]),
        "valid ends_at event targets should be explicit domain data"
    );
    assert_eq!(
        analyze_results["determination_status"],
        serde_json::json!("undetermined"),
        "task event-axis determination should be explicit backend contract data"
    );
    assert_eq!(
        analyze_results["placement_ready"],
        serde_json::json!(false),
        "task placement readiness should be explicit backend contract data"
    );
    assert_eq!(
        analyze_results["placement_status"],
        serde_json::json!("diagnostic"),
        "invalid references should make task placement diagnostic in the backend contract"
    );
    assert_eq!(
        analyze_results["placement_messages"],
        serde_json::json!([
            "task 'analyze-results' depends on task 'prepare-samples' which ends at event 'BEC'; depend on the event instead",
            "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'",
            "task 'analyze-results' ends_at references missing event id 'MISSING-EVENT'",
            "task 'analyze-results' cannot be placed on the event axis without upstream or downstream event anchors"
        ]),
        "task placement messages should expose backend placement and validation diagnostics directly"
    );
    assert_eq!(
        analyze_results["effective_anchors"],
        serde_json::json!({
            "upstream": null,
            "downstream": null,
            "diagnostics": [
                "task 'analyze-results' depends on task 'prepare-samples' which ends at event 'BEC'; depend on the event instead",
                "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'",
                "task 'analyze-results' ends_at references missing event id 'MISSING-EVENT'",
                "task 'analyze-results' cannot be placed on the event axis without upstream or downstream event anchors"
            ]
        }),
        "invalid effective anchors should carry backend placement diagnostics"
    );

    let prepare_samples = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "prepare-samples")
        .expect("prepare-samples should be included");
    assert_eq!(
        prepare_samples["effective_anchors"],
        serde_json::json!({
            "upstream": "START",
            "downstream": "BEC",
            "diagnostics": []
        }),
        "tasks blocked by an event and ending at an event should report effective anchors"
    );
    assert_eq!(
        prepare_samples["determination_status"],
        serde_json::json!("fully_determined"),
        "tasks with upstream and downstream event-axis anchors should be fully determined"
    );
    assert_eq!(prepare_samples["placement_ready"], true);
    assert_eq!(prepare_samples["placement_status"], "ready");
    assert_eq!(prepare_samples["placement_messages"], serde_json::json!([]));

    let screen_data = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "screen-data")
        .expect("screen-data should be included");
    assert_eq!(
        screen_data["effective_anchors"],
        serde_json::json!({
            "upstream": "BEC",
            "downstream": null,
            "diagnostics": [
                "task 'screen-data' cannot be placed on the event axis without a downstream event anchor"
            ]
        }),
        "effective timeline anchors should come from Rust workflow semantics"
    );
    assert_eq!(
        screen_data["determination_status"],
        serde_json::json!("undetermined"),
        "tasks missing either event-axis anchor should be undetermined"
    );
    assert_eq!(screen_data["placement_ready"], false);
    assert_eq!(screen_data["placement_status"], "incomplete");
    assert_eq!(
        screen_data["placement_messages"],
        serde_json::json!([
            "task 'screen-data' cannot be placed on the event axis without a downstream event anchor"
        ]),
        "missing-anchor placement copy should come from the Rust workflow contract"
    );

    assert_eq!(
        workflow["event_nodes"],
        serde_json::json!([
            {
                "id": "START",
                "boundary_role": "start_boundary",
                "chart_order": 0,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            },
            {
                "id": "BEC",
                "boundary_role": "ordinary",
                "chart_order": 1,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            },
            {
                "id": "END",
                "boundary_role": "finish_boundary",
                "chart_order": 2,
                "placement_ready": true,
                "placement_status": "ready",
                "placement_messages": []
            }
        ]),
        "event placement readiness should be part of the backend workflow contract"
    );

    assert_eq!(workflow["validation"]["valid"], false);
    assert!(
        workflow["validation"]["diagnostics"]
            .as_array()
            .expect("validation diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic
                == "task 'analyze-results' depends on missing task or event id 'MISSING-BLOCKER'"),
        "workflow JSON should include project-level validation diagnostics"
    );

    assert_no_visual_layout_fields(&workflow);
}

#[test]
fn task_workflow_json_reports_reversed_event_span_as_domain_validation() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying workflow validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("conflicted-run"), "conflicted-run");
    fs::write(
        project_dir
            .join("conflicted-run")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"conflicted-run\",\n  \"dependencies\": [\n    \"BEC\"\n  ],\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("conflicted task metadata should be rewritten with reversed event span");

    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow --json should succeed while reporting validation diagnostics: {workflow_output:?}"
    );

    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("stdout should be JSON");
    let conflicted_run = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "conflicted-run")
        .expect("conflicted-run should be included");

    assert_eq!(
        conflicted_run["validation_diagnostics"],
        serde_json::json!([
            "task 'conflicted-run' depends on event 'BEC' after ends_at event 'START'"
        ]),
        "event-span validation policy should be owned by the Rust workflow contract"
    );
    assert_eq!(
        conflicted_run["effective_anchors"],
        serde_json::json!({
            "upstream": "BEC",
            "downstream": "START",
            "diagnostics": [
                "task 'conflicted-run' depends on event 'BEC' after ends_at event 'START'"
            ]
        }),
        "conflicting anchors should be explicit in Rust workflow JSON"
    );
    assert_eq!(workflow["validation"]["valid"], false);
    assert!(
        workflow["validation"]["diagnostics"]
            .as_array()
            .expect("workflow diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic
                == "task 'conflicted-run' depends on event 'BEC' after ends_at event 'START'"),
        "project-level validation diagnostics should include reversed event spans"
    );
}

#[test]
fn task_workflow_json_offers_end_events_at_or_after_existing_event_blockers() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying workflow end-event targets: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"start\",\n    \"finished\",\n    \"samples ready\",\n    \"data ready\",\n    \"analysis complete\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analysis-run"), "analysis-run");
    fs::write(
        project_dir
            .join("analysis-run")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"data ready\"\n  ]\n}\n",
    )
    .expect("analysis task metadata should be rewritten with event blocker");

    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow --json should succeed while reporting valid end-event targets: {workflow_output:?}"
    );

    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("stdout should be JSON");
    let analysis_run = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analysis-run")
        .expect("analysis-run should be included");

    assert_eq!(
        analysis_run["valid_ends_at_targets"],
        serde_json::json!(["data ready", "analysis complete"]),
        "Rust workflow JSON should only offer end events at or after existing event blockers"
    );
}

#[test]
fn task_workflow_json_reports_done_tasks_blocked_by_blocked_tasks() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying workflow validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"FLUORESCENCE\",\n    \"DONE\"\n  ]\n}\n",
    );

    support::write_task_metadata(
        &project_dir.join("collect-fluorescence"),
        "collect-fluorescence",
    );
    fs::write(
        project_dir
            .join("collect-fluorescence")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"collect-fluorescence\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"FLUORESCENCE\",\n  \"status\": \"blocked\"\n}\n",
    )
    .expect("blocked task metadata should be rewritten with status");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"FLUORESCENCE\"\n  ],\n  \"ends_at\": \"DONE\",\n  \"status\": \"done\"\n}\n",
    )
    .expect("done task metadata should be rewritten with status");

    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow --json should succeed while reporting sanity diagnostics: {workflow_output:?}"
    );

    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("stdout should be JSON");
    let analyze_results = workflow["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("analyze-results should be included");

    assert_eq!(
        analyze_results["validation_diagnostics"],
        serde_json::json!([
            "task 'analyze-results' is done but depends on event 'FLUORESCENCE' reached by blocked task 'collect-fluorescence'"
        ]),
        "status sanity diagnostics should be owned by Rust workflow JSON"
    );
    assert_eq!(
        analyze_results["effective_anchors"],
        serde_json::json!({
            "upstream": "FLUORESCENCE",
            "downstream": "DONE",
            "diagnostics": [
                "task 'analyze-results' is done but depends on event 'FLUORESCENCE' reached by blocked task 'collect-fluorescence'"
            ]
        }),
        "tasks blocked by events should carry effective anchors alongside status diagnostics"
    );
    assert_eq!(workflow["validation"]["valid"], false);
    assert!(
        workflow["validation"]["diagnostics"]
            .as_array()
            .expect("workflow diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic
                == "task 'analyze-results' is done but depends on event 'FLUORESCENCE' reached by blocked task 'collect-fluorescence'"),
        "project-level validation diagnostics should include status sanity checks"
    );
}

#[test]
fn task_workflow_json_reports_effective_anchors_for_chained_task_blockers() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying workflow validation: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"Workflow started\",\n    \"Protocol selected\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("make chart"), "make chart");
    fs::write(
        project_dir.join("make chart").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"make chart\",\n  \"dependencies\": [\n    \"Workflow started\"\n  ]\n}\n",
    )
    .expect("upstream task metadata should be rewritten with event dependency");

    support::write_task_metadata(&project_dir.join("literature-review"), "literature-review");
    fs::write(
        project_dir
            .join("literature-review")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"literature-review\",\n  \"dependencies\": [\n    \"make chart\"\n  ],\n  \"ends_at\": \"Protocol selected\"\n}\n",
    )
    .expect("downstream task metadata should be rewritten with task dependency");

    let workflow_output = run_spielgantt_in(&project_dir, &["task", "workflow", "--json"]);
    assert!(
        workflow_output.status.success(),
        "task workflow --json should succeed while reporting effective anchors: {workflow_output:?}"
    );

    let workflow: serde_json::Value =
        serde_json::from_str(&stdout(&workflow_output)).expect("stdout should be JSON");
    let tasks = workflow["tasks"]
        .as_array()
        .expect("workflow tasks should be an array");
    let make_chart = tasks
        .iter()
        .find(|task| task["id"] == "make chart")
        .expect("make chart should be included");
    let literature_review = tasks
        .iter()
        .find(|task| task["id"] == "literature-review")
        .expect("literature-review should be included");

    assert_eq!(
        make_chart["effective_anchors"],
        serde_json::json!({
            "upstream": "Workflow started",
            "downstream": "Protocol selected",
            "diagnostics": []
        }),
        "Rust should infer a downstream anchor through dependent tasks"
    );
    assert_eq!(
        literature_review["effective_anchors"],
        serde_json::json!({
            "upstream": "Workflow started",
            "downstream": "Protocol selected",
            "diagnostics": []
        }),
        "Rust should infer an upstream anchor through task blockers"
    );
}

fn assert_no_visual_layout_fields(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                assert!(
                    !matches!(
                        key.as_str(),
                        "grid_column"
                            | "gridColumn"
                            | "lane"
                            | "row"
                            | "pixel"
                            | "css"
                            | "connector"
                    ),
                    "workflow domain JSON should not prescribe visual layout fields; found {key}"
                );
                assert_no_visual_layout_fields(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                assert_no_visual_layout_fields(value);
            }
        }
        _ => {}
    }
}
