use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

#[test]
fn event_list_shows_project_events_in_metadata_order_from_any_project_path() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let nested_dir = project_dir.join("analysis").join("notes");

    fs::create_dir_all(&nested_dir).expect("nested directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before listing events: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    );

    let list_output = run_spielgantt_in(&nested_dir, &["event", "list"]);
    assert!(
        list_output.status.success(),
        "event list should succeed from any directory inside a project: {list_output:?}"
    );

    let listed_events = stdout(&list_output);
    assert!(
        listed_events.contains("Events:"),
        "event list should label the output as an event list: {listed_events}"
    );
    assert!(
        listed_events.contains("1. START")
            && listed_events.contains("2. MOT")
            && listed_events.contains("3. BEC"),
        "event list should preserve project metadata order: {listed_events}"
    );
}

#[test]
fn event_list_reports_default_project_events_after_init() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    fs::create_dir(&project_dir).expect("project directory should be created");
    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before listing empty events: {init_output:?}"
    );

    let list_output = run_spielgantt_in(&project_dir, &["event", "list"]);
    assert!(
        list_output.status.success(),
        "event list should succeed for a newly initialized project: {list_output:?}"
    );

    let listed_events = stdout(&list_output);
    assert!(
        listed_events.contains("Events:")
            && listed_events.contains("1. start")
            && listed_events.contains("2. finished"),
        "event list should expose default boundary events for new projects: {listed_events}"
    );
}
