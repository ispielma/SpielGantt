use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn task_relationships_json_reports_dependency_relationships_and_event_blockers() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before querying dependency relationships: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    for task_id in ["prepare-samples", "analyze-results", "screen-data"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }

    fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"dependencies\": [\n    \"START\"\n  ],\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("prepare-samples metadata should be rewritten with event references");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("analyze-results metadata should be rewritten with mixed dependencies");

    let relationships_output =
        run_spielgantt_in(&project_dir, &["task", "relationships", "--json"]);
    assert!(
        relationships_output.status.success(),
        "task relationships --json should succeed: {relationships_output:?}"
    );

    let relationships: serde_json::Value =
        serde_json::from_str(&stdout(&relationships_output)).expect("stdout should be JSON");

    assert_eq!(relationships["schema_version"], 1);
    let events = support::json_array(&relationships, "events");
    let start_event = support::find_json_object_by_str(events, "id", "START");
    assert_eq!(
        start_event["references"],
        serde_json::json!([
            {"task_id": "analyze-results", "kind": "dependency"},
            {"task_id": "prepare-samples", "kind": "dependency"}
        ]),
        "event references should be provided by the domain JSON contract"
    );
    let bec_event = support::find_json_object_by_str(events, "id", "BEC");
    assert_eq!(
        bec_event["deletion_blockers"],
        serde_json::json!([
            {"task_id": "prepare-samples", "kind": "ends_at"}
        ]),
        "event deletion blockers should include task ends_at references"
    );

    let analyze_results = relationships["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "analyze-results")
        .expect("analyze-results should be included");
    assert_eq!(
        analyze_results["blockers"],
        serde_json::json!([
            {"id": "START", "kind": "event"},
            {"id": "prepare-samples", "kind": "task"}
        ]),
        "direct blockers should resolve task and event kinds"
    );
    assert_eq!(
        analyze_results["blocks"],
        serde_json::json!([]),
        "reverse blocks relationships should not require frontend scanning"
    );

    let prepare_samples = relationships["tasks"]
        .as_array()
        .expect("tasks should be an array")
        .iter()
        .find(|task| task["id"] == "prepare-samples")
        .expect("prepare-samples should be included");
    assert_eq!(
        prepare_samples["blocks"],
        serde_json::json!([
            {"id": "analyze-results", "kind": "task"}
        ]),
        "reverse blocks relationships should identify dependent tasks"
    );
    assert!(
        prepare_samples["valid_dependency_targets"]
            .as_array()
            .expect("valid targets should be an array")
            .iter()
            .all(|target| target["id"] != "analyze-results"),
        "valid targets should exclude choices that would create cycles"
    );
    assert!(
        prepare_samples["valid_dependency_targets"]
            .as_array()
            .expect("valid targets should be an array")
            .iter()
            .all(|target| target["id"] != "prepare-samples"),
        "valid targets should exclude self-dependencies"
    );
}

#[test]
fn task_depend_adds_a_blocker_by_task_id() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adding a dependency: {init_output:?}"
    );

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_blocker_output.status.success(),
        "blocker task create should succeed before adding a dependency: {create_blocker_output:?}"
    );
    let create_dependent_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_dependent_output.status.success(),
        "dependent task create should succeed before adding a dependency: {create_dependent_output:?}"
    );

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "calibrate-laser"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed when both task ids exist: {depend_output:?}"
    );

    let dependent_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("dependent task metadata should be readable");
    assert!(
        dependent_metadata.contains("\"dependencies\""),
        "task depend should persist a dependencies list: {dependent_metadata}"
    );
    assert!(
        dependent_metadata.contains("\"calibrate-laser\""),
        "task depend should store the blocker id in metadata: {dependent_metadata}"
    );
    assert!(
        stdout(&depend_output)
            .contains("Added blocker 'calibrate-laser' to task 'analyze-results'"),
        "task depend should confirm the added blocker: {}",
        stdout(&depend_output)
    );
}

#[test]
fn task_depend_adds_a_blocker_by_event_id() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adding an event dependency: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    let create_task_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_task_output.status.success(),
        "task create should succeed before adding an event dependency: {create_task_output:?}"
    );

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "START"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should accept an existing event blocker: {depend_output:?}"
    );

    let dependent_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("dependent task metadata should be readable");
    assert!(
        dependent_metadata.contains("\"START\""),
        "task depend should persist the event blocker id in metadata: {dependent_metadata}"
    );
    assert!(
        stdout(&depend_output).contains("Added blocker 'START' to task 'analyze-results'"),
        "task depend should confirm the added event blocker: {}",
        stdout(&depend_output)
    );
}

#[test]
fn task_dependency_remove_removes_a_blocker_by_cli() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before removing a dependency: {init_output:?}"
    );

    for task_id in ["calibrate-laser", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "calibrate-laser"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before removing dependency: {depend_output:?}"
    );

    let remove_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "dependency",
            "remove",
            "analyze-results",
            "calibrate-laser",
        ],
    );
    assert!(
        remove_output.status.success(),
        "task dependency remove should succeed: {remove_output:?}"
    );

    let dependent_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("dependent task metadata should be readable");
    assert!(
        !dependent_metadata.contains("calibrate-laser"),
        "task dependency remove should persist removal from task metadata: {dependent_metadata}"
    );
    assert!(
        stdout(&remove_output)
            .contains("Removed blocker 'calibrate-laser' from task 'analyze-results'"),
        "task dependency remove should confirm the removed blocker: {}",
        stdout(&remove_output)
    );
}

#[test]
fn task_dependency_remove_rejects_missing_tasks_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before removing a dependency: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before removing dependency from a missing task: {create_output:?}"
    );

    let task_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let remove_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "dependency",
            "remove",
            "missing-task",
            "analyze-results",
        ],
    );
    assert!(
        !remove_output.status.success(),
        "task dependency remove should reject missing tasks: {remove_output:?}"
    );
    assert!(
        stderr(&remove_output).contains("task id 'missing-task' was not found"),
        "missing task rejection should identify the missing task: {}",
        stderr(&remove_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after missing task rejection"),
        original_metadata,
        "missing task rejection should not rewrite existing task metadata"
    );
}

#[test]
fn task_show_lists_a_tasks_blockers() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before showing blockers: {init_output:?}"
    );

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_blocker_output.status.success(),
        "blocker task create should succeed before showing blockers: {create_blocker_output:?}"
    );
    let create_dependent_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_dependent_output.status.success(),
        "dependent task create should succeed before showing blockers: {create_dependent_output:?}"
    );
    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "calibrate-laser"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before showing blockers: {depend_output:?}"
    );

    let show_output = run_spielgantt_in(&project_dir, &["task", "show", "analyze-results"]);
    assert!(
        show_output.status.success(),
        "task show should succeed for an existing task: {show_output:?}"
    );

    let shown_task = stdout(&show_output);
    assert!(
        shown_task.contains("Task: analyze-results"),
        "task show should identify the requested task: {shown_task}"
    );
    assert!(
        shown_task.contains("Blockers:\n- calibrate-laser"),
        "task show should list blocker ids: {shown_task}"
    );
}

#[test]
fn task_depend_rejects_missing_task_or_event_blockers_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before missing blocker rejection: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    let create_task_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_task_output.status.success(),
        "task create should succeed before missing blocker rejection: {create_task_output:?}"
    );

    let task_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "MISSING"],
    );
    assert!(
        !depend_output.status.success(),
        "task depend should reject a missing task or event blocker: {depend_output:?}"
    );
    assert!(
        stderr(&depend_output).contains("task or event id 'MISSING' was not found"),
        "missing blocker rejection should mention the shared task/event namespace: {}",
        stderr(&depend_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after missing blocker rejection"),
        original_metadata,
        "missing blocker rejection should not rewrite task metadata"
    );
}

#[test]
fn task_depend_rejects_cycles_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before cycle rejection: {init_output:?}"
    );

    let create_first_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_first_output.status.success(),
        "first task create should succeed before cycle rejection: {create_first_output:?}"
    );
    let create_second_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_second_output.status.success(),
        "second task create should succeed before cycle rejection: {create_second_output:?}"
    );

    let first_dependency_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "calibrate-laser"],
    );
    assert!(
        first_dependency_output.status.success(),
        "first dependency should be accepted before cycle rejection: {first_dependency_output:?}"
    );

    let calibrate_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_calibrate_metadata = fs::read_to_string(&calibrate_metadata_path)
        .expect("task metadata should be readable before rejected cycle");

    let cycle_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "calibrate-laser", "analyze-results"],
    );
    assert!(
        !cycle_output.status.success(),
        "task depend should reject a cycle before writing metadata: {cycle_output:?}"
    );
    assert!(
        stderr(&cycle_output).contains("dependency cycle"),
        "cycle rejection should explain the graph problem: {}",
        stderr(&cycle_output)
    );
    assert_eq!(
        fs::read_to_string(&calibrate_metadata_path)
            .expect("task metadata should remain readable after rejected cycle"),
        original_calibrate_metadata,
        "cycle rejection should not rewrite task metadata"
    );
}

#[test]
fn task_depend_rejects_self_dependency_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before self-dependency rejection: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before self-dependency rejection: {create_output:?}"
    );

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let self_dependency_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "calibrate-laser", "calibrate-laser"],
    );
    assert!(
        !self_dependency_output.status.success(),
        "task depend should reject a self-dependency: {self_dependency_output:?}"
    );
    assert!(
        stderr(&self_dependency_output).contains("dependency cycle"),
        "self-dependency rejection should explain the graph problem: {}",
        stderr(&self_dependency_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after rejected self-dependency"),
        original_metadata,
        "self-dependency rejection should not rewrite task metadata"
    );
}

#[test]
fn task_depend_rejects_blockers_that_end_at_events_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before ends_at blocker rejection: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        create_blocker_output.status.success(),
        "blocker task create should succeed before ends_at blocker rejection: {create_blocker_output:?}"
    );
    let create_task_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_task_output.status.success(),
        "task create should succeed before ends_at blocker rejection: {create_task_output:?}"
    );

    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("blocker task metadata should be rewritten with ends_at");

    let task_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "prepare-sample"],
    );
    assert!(
        !depend_output.status.success(),
        "task depend should reject blockers that end at an event: {depend_output:?}"
    );
    assert!(
        stderr(&depend_output)
            .contains("task 'prepare-sample' ends at event 'BEC'; depend on the event instead"),
        "ends_at blocker rejection should steer the user to the event: {}",
        stderr(&depend_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after ends_at blocker rejection"),
        original_metadata,
        "ends_at blocker rejection should not rewrite task metadata"
    );
}

#[test]
fn task_depend_rejects_cycles_that_traverse_events_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before event cycle rejection: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    let create_first_output = run_spielgantt_in(&project_dir, &["task", "create", "task-a"]);
    assert!(
        create_first_output.status.success(),
        "first task create should succeed before event cycle rejection: {create_first_output:?}"
    );
    let create_second_output = run_spielgantt_in(&project_dir, &["task", "create", "task-b"]);
    assert!(
        create_second_output.status.success(),
        "second task create should succeed before event cycle rejection: {create_second_output:?}"
    );

    fs::write(
        project_dir.join("task-b").join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"task-b\",\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("task metadata should be rewritten with ends_at");

    let first_dependency_output =
        run_spielgantt_in(&project_dir, &["task", "depend", "task-a", "START"]);
    assert!(
        first_dependency_output.status.success(),
        "event dependency should be accepted before event cycle rejection: {first_dependency_output:?}"
    );

    let task_metadata_path = project_dir.join("task-b").join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let cycle_output = run_spielgantt_in(&project_dir, &["task", "depend", "task-b", "task-a"]);
    assert!(
        !cycle_output.status.success(),
        "task depend should reject cycles that traverse events: {cycle_output:?}"
    );
    assert!(
        stderr(&cycle_output).contains("dependency cycle"),
        "event cycle rejection should explain the graph problem: {}",
        stderr(&cycle_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after rejected event cycle"),
        original_metadata,
        "event cycle rejection should not rewrite task metadata"
    );
}

#[test]
fn task_depend_rejects_event_blockers_after_the_current_end_event_before_writing() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before event-order dependency rejection: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"start\",\n    \"samples ready\",\n    \"data ready\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    support::write_task_metadata(&project_dir.join("analysis-run"), "analysis-run");
    let task_metadata_path = project_dir.join("analysis-run/.spielgantt/task.json");
    fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"start\"\n  ],\n  \"ends_at\": \"samples ready\"\n}\n",
    )
    .expect("task metadata should be written with an end event");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analysis-run", "data ready"],
    );
    assert!(
        !depend_output.status.success(),
        "task depend should reject event blockers after the task end event: {depend_output:?}"
    );
    assert!(
        stderr(&depend_output).contains(
            "cannot add event blocker 'data ready' to task 'analysis-run' after ends_at event 'samples ready'"
        ),
        "event-order rejection should identify the blocker and end event: {}",
        stderr(&depend_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after event-order rejection"),
        original_metadata,
        "event-order rejection should not rewrite task metadata"
    );
}
