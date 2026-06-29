use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn task_ends_at_sets_an_event_and_heals_duplicate_downstream_dependencies() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before setting ends_at: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    let create_prepare_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        create_prepare_output.status.success(),
        "task create should succeed for the ending task: {create_prepare_output:?}"
    );
    let create_analyze_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_analyze_output.status.success(),
        "task create should succeed for the downstream task: {create_analyze_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-sample\",\n    \"START\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be written");

    let ends_at_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "prepare-sample", "START"],
    );
    assert!(
        ends_at_output.status.success(),
        "task ends-at should succeed when the event exists: {ends_at_output:?}"
    );
    assert!(
        stdout(&ends_at_output).contains("Set task 'prepare-sample' to end at event 'START'"),
        "task ends-at should confirm the new event anchor: {}",
        stdout(&ends_at_output)
    );

    let ending_metadata = fs::read_to_string(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
    )
    .expect("ending task metadata should be readable");
    assert!(
        ending_metadata.contains("\"ends_at\": \"START\""),
        "task ends-at should persist the event anchor: {ending_metadata}"
    );

    let downstream_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("downstream task metadata should be readable");
    assert!(
        downstream_metadata.contains("\"START\""),
        "task ends-at should heal downstream dependencies to the event: {downstream_metadata}"
    );
    assert!(
        !downstream_metadata.contains("\"prepare-sample\""),
        "task ends-at should rewrite task dependencies away from the ending task: {downstream_metadata}"
    );
    assert_eq!(
        downstream_metadata.matches("\"START\"").count(),
        1,
        "task ends-at should deduplicate dependencies created by healing: {downstream_metadata}"
    );
}

#[test]
fn task_ends_at_clear_removes_the_event_anchor_without_rewriting_dependencies() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before clearing ends_at: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    let create_prepare_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        create_prepare_output.status.success(),
        "task create should succeed for the ending task: {create_prepare_output:?}"
    );
    let create_analyze_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_analyze_output.status.success(),
        "task create should succeed for the downstream task: {create_analyze_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-sample\",\n    \"START\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be written");

    let set_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "prepare-sample", "START"],
    );
    assert!(
        set_output.status.success(),
        "task ends-at should succeed before clearing: {set_output:?}"
    );

    let clear_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "prepare-sample", "--clear"],
    );
    assert!(
        clear_output.status.success(),
        "task ends-at --clear should succeed: {clear_output:?}"
    );
    assert!(
        stdout(&clear_output).contains("Cleared ends_at for task 'prepare-sample'"),
        "task ends-at --clear should confirm the cleared anchor: {}",
        stdout(&clear_output)
    );

    let ending_metadata = fs::read_to_string(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
    )
    .expect("ending task metadata should be readable");
    assert!(
        !ending_metadata.contains("\"ends_at\""),
        "task ends-at --clear should remove the event anchor: {ending_metadata}"
    );

    let downstream_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("downstream task metadata should be readable");
    assert!(
        downstream_metadata.contains("\"START\""),
        "task ends-at --clear should preserve healed event dependencies: {downstream_metadata}"
    );
    assert!(
        !downstream_metadata.contains("\"prepare-sample\""),
        "task ends-at --clear should leave the healed dependency in place: {downstream_metadata}"
    );
}

#[test]
fn task_ends_at_rolls_back_healed_dependencies_when_a_later_write_fails() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before ends_at rollback checks: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    let create_prepare_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        create_prepare_output.status.success(),
        "task create should succeed for the ending task: {create_prepare_output:?}"
    );
    let create_analyze_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_analyze_output.status.success(),
        "task create should succeed for the downstream task: {create_analyze_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"prepare-sample\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be written");

    let original_prepare_metadata = fs::read_to_string(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
    )
    .expect("ending task metadata should be readable before the failed mutation");
    let original_analyze_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("downstream task metadata should be readable before the failed mutation");

    fs::create_dir(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json.tmp"),
    )
    .expect("temporary metadata collision directory should be created");

    let ends_at_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "prepare-sample", "START"],
    );
    assert!(
        !ends_at_output.status.success(),
        "task ends-at should fail when a later task metadata write fails: {ends_at_output:?}"
    );
    assert!(
        stderr(&ends_at_output).contains("failed to write temporary task metadata"),
        "task ends-at should surface the later write failure: {}",
        stderr(&ends_at_output)
    );

    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("prepare-sample")
                .join(".spielgantt/task.json"),
        )
        .expect("ending task metadata should remain readable after the failed mutation"),
        original_prepare_metadata,
        "failed task ends-at should preserve the ending task metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("analyze-results")
                .join(".spielgantt/task.json"),
        )
        .expect("downstream task metadata should remain readable after the failed mutation"),
        original_analyze_metadata,
        "failed task ends-at should restore already-written healed dependencies"
    );
}

#[test]
fn task_ends_at_rejects_invalid_events_missing_tasks_and_missing_events_before_writing() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rejection checks: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    let create_task_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_task_output.status.success(),
        "task create should succeed before rejection checks: {create_task_output:?}"
    );

    let task_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let invalid_event_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "analyze-results", "phase/name"],
    );
    assert!(
        !invalid_event_output.status.success(),
        "task ends-at should reject invalid event ids: {invalid_event_output:?}"
    );
    assert!(
        stdout(&invalid_event_output).is_empty(),
        "invalid event rejection should not write success output: {}",
        stdout(&invalid_event_output)
    );
    assert!(
        stderr(&invalid_event_output).contains("event id must not contain filesystem separator"),
        "invalid event rejection should explain the path rule: {}",
        stderr(&invalid_event_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after invalid event rejection"),
        original_metadata,
        "invalid event rejection should not rewrite task metadata"
    );

    let missing_task_output =
        run_spielgantt_in(&project_dir, &["task", "ends-at", "missing-task", "START"]);
    assert!(
        !missing_task_output.status.success(),
        "task ends-at should reject missing tasks: {missing_task_output:?}"
    );
    assert!(
        stderr(&missing_task_output).contains("task id 'missing-task' was not found"),
        "missing task rejection should identify the missing task: {}",
        stderr(&missing_task_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after missing task rejection"),
        original_metadata,
        "missing task rejection should not rewrite task metadata"
    );

    let missing_event_output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "analyze-results", "MISSING"],
    );
    assert!(
        !missing_event_output.status.success(),
        "task ends-at should reject missing events: {missing_event_output:?}"
    );
    assert!(
        stderr(&missing_event_output).contains("event id 'MISSING' was not found"),
        "missing event rejection should identify the missing event: {}",
        stderr(&missing_event_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after missing event rejection"),
        original_metadata,
        "missing event rejection should not rewrite task metadata"
    );
}

#[test]
fn task_ends_at_rejects_an_event_before_an_existing_event_blocker() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before reversed span rejection checks: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"start\",\n    \"samples ready\",\n    \"data ready\",\n    \"analysis complete\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");

    support::write_task_metadata(&project_dir.join("analysis-run"), "analysis-run");
    let task_metadata_path = project_dir.join("analysis-run/.spielgantt/task.json");
    fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analysis-run\",\n  \"dependencies\": [\n    \"data ready\"\n  ]\n}\n",
    )
    .expect("task metadata should be written with an event blocker");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let output = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "analysis-run", "samples ready"],
    );
    assert!(
        !output.status.success(),
        "task ends-at should reject reversed event spans: {output:?}"
    );
    assert!(
        stdout(&output).is_empty(),
        "reversed span rejection should not write success output: {}",
        stdout(&output)
    );
    assert!(
        stderr(&output).contains(
            "cannot set task 'analysis-run' to end at event 'samples ready' before existing event blocker 'data ready'"
        ),
        "rejection should identify the requested end event and blocking event: {}",
        stderr(&output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after reversed span rejection"),
        original_metadata,
        "reversed span rejection should not rewrite task metadata"
    );
}
