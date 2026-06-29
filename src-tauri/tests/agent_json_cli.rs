use std::fs;

use serde_json::Value;
use tempfile::tempdir;

mod support;

#[test]
fn agent_inspection_commands_emit_stable_json_payloads() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before JSON inspection: {init_output:?}"
    );
    let canonical_project_dir =
        fs::canonicalize(&project_dir).expect("project directory should canonicalize");

    let create_task_output = support::create_task(&project_dir, "analyze-results");
    assert!(
        create_task_output.status.success(),
        "task create should succeed before JSON inspection: {create_task_output:?}"
    );

    let create_event_output =
        support::run_spielgantt_in(&project_dir, &["event", "create", "START"]);
    assert!(
        create_event_output.status.success(),
        "event create should succeed before JSON inspection: {create_event_output:?}"
    );

    let update_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "update", "analyze-results", "--status", "unblocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed before JSON inspection: {update_output:?}"
    );

    let depend_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "START"],
    );
    assert!(
        depend_output.status.success(),
        "event dependency should be added before JSON inspection: {depend_output:?}"
    );

    let validate_json = support::run_spielgantt_json_in(&project_dir, &["validate", "--json"]);
    assert_eq!(validate_json["schema_version"], 1);
    assert_eq!(validate_json["valid"], true);
    assert_eq!(
        validate_json["project_root"],
        canonical_project_dir.display().to_string()
    );
    assert_eq!(validate_json["issues"], Value::Array(Vec::new()));

    let task_list_json = support::run_spielgantt_json_in(&project_dir, &["task", "list", "--json"]);
    assert_eq!(
        task_list_json,
        serde_json::json!({
            "schema_version": 1,
            "tasks": [
                {
                    "id": "analyze-results",
                    "status": "unblocked"
                }
            ]
        })
    );

    let task_show_json = support::run_spielgantt_json_in(
        &project_dir,
        &["task", "show", "--json", "analyze-results"],
    );
    assert_eq!(task_show_json["schema_version"], 1);
    assert_eq!(task_show_json["id"], "analyze-results");
    assert_eq!(
        task_show_json["path"],
        canonical_project_dir
            .join("analyze-results")
            .display()
            .to_string()
    );
    assert_eq!(task_show_json["dependencies"], serde_json::json!(["START"]));
    assert_eq!(task_show_json["status"], "unblocked");
    assert_eq!(task_show_json.get("progress"), None);

    let event_list_json =
        support::run_spielgantt_json_in(&project_dir, &["event", "list", "--json"]);
    assert_eq!(
        event_list_json,
        serde_json::json!({
            "schema_version": 1,
            "events": ["start", "START", "finished"]
        })
    );
}

#[test]
fn validate_json_reports_invalid_projects_with_machine_readable_issues() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before invalid validation JSON: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    );
    support::write_task_metadata(&project_dir.join("collision"), "START");

    let validate_output = support::run_spielgantt_in(&project_dir, &["validate", "--json"]);
    assert!(
        !validate_output.status.success(),
        "validate --json should keep a failing exit status for invalid projects: {validate_output:?}"
    );
    assert!(
        support::stderr_text(&validate_output).is_empty(),
        "validation issues should be carried by the JSON report when JSON is requested"
    );

    let validate_json = support::stdout_json(&validate_output);
    assert_eq!(validate_json["schema_version"], 1);
    assert_eq!(validate_json["valid"], false);
    assert_eq!(
        validate_json["issues"],
        serde_json::json!(["task id 'START' collides with project event id 'START'"])
    );
}
