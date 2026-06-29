use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn task_update_sets_status_and_task_show_reads_it() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before updating task metadata: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before updating task metadata: {create_output:?}"
    );

    let update_output = run_spielgantt_in(
        &project_dir,
        &["task", "update", "calibrate-laser", "--status", "unblocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed with valid task metadata: {update_output:?}"
    );

    let task_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(
            project_dir
                .join("calibrate-laser")
                .join(".spielgantt/task.json"),
        )
        .expect("task metadata should remain readable after task update"),
    )
    .expect("task metadata should remain valid JSON");
    assert_eq!(
        task_metadata.status,
        Some(spielgantt_lib::metadata::TaskStatus::Unblocked),
        "task update should persist the status"
    );

    let show_output = run_spielgantt_in(&project_dir, &["task", "show", "calibrate-laser"]);
    assert!(
        show_output.status.success(),
        "task show should succeed after task update: {show_output:?}"
    );

    let shown_task = stdout(&show_output);
    assert!(
        shown_task.contains("Status: unblocked"),
        "task show should display the status: {shown_task}"
    );
    assert!(
        !shown_task.contains("Progress:"),
        "task show should not display removed progress metadata: {shown_task}"
    );
}

#[test]
fn task_list_displays_status_for_all_tasks() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before listing tasks: {init_output:?}"
    );

    let unblocked_create_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        unblocked_create_output.status.success(),
        "task create should succeed before listing tasks: {unblocked_create_output:?}"
    );
    let planned_create_output =
        run_spielgantt_in(&project_dir, &["task", "create", "prepare-sample"]);
    assert!(
        planned_create_output.status.success(),
        "task create should succeed before listing tasks: {planned_create_output:?}"
    );

    let update_output = run_spielgantt_in(
        &project_dir,
        &["task", "update", "calibrate-laser", "--status", "unblocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed before listing tasks: {update_output:?}"
    );

    let list_output = run_spielgantt_in(&project_dir, &["task", "list"]);
    assert!(
        list_output.status.success(),
        "task list should succeed for a valid project: {list_output:?}"
    );
    let listed_tasks = stdout(&list_output);
    assert!(
        listed_tasks.contains("ID\tStatus"),
        "task list should include task state headings: {listed_tasks}"
    );
    assert!(
        listed_tasks.contains("calibrate-laser\tunblocked"),
        "task list should display status metadata: {listed_tasks}"
    );
    assert!(
        listed_tasks.contains("prepare-sample\t-"),
        "task list should keep tasks without state visible: {listed_tasks}"
    );
}

#[test]
fn task_update_rejects_missing_status_before_rewriting_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before invalid task updates: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before invalid task updates: {create_output:?}"
    );

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let missing_status_output =
        run_spielgantt_in(&project_dir, &["task", "update", "calibrate-laser"]);
    assert!(
        !missing_status_output.status.success(),
        "task update should reject calls without a status: {missing_status_output:?}"
    );
    assert!(
        stderr(&missing_status_output).contains("--status"),
        "task update should explain the required status option: {}",
        stderr(&missing_status_output)
    );

    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after rejected updates"),
        original_metadata,
        "rejected task updates should not rewrite task metadata"
    );
}

#[test]
fn task_update_rejects_missing_tasks_before_rewriting_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before missing task update rejection: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before missing task update rejection: {create_output:?}"
    );

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let missing_task_output = run_spielgantt_in(
        &project_dir,
        &["task", "update", "missing-task", "--status", "blocked"],
    );
    assert!(
        !missing_task_output.status.success(),
        "task update should reject missing tasks: {missing_task_output:?}"
    );
    assert!(
        stderr(&missing_task_output).contains("task id 'missing-task' was not found"),
        "missing task rejection should identify the missing task: {}",
        stderr(&missing_task_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after missing task update rejection"),
        original_metadata,
        "missing task update rejection should not rewrite existing task metadata"
    );
}
