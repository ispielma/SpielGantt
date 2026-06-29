use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn init_creates_project_metadata_and_agent_scaffolding_in_the_target_directory() {
    let project_dir = tempdir().expect("temporary directory should be created");

    let output = run_spielgantt_in(project_dir.path(), &["init"]);
    assert!(
        output.status.success(),
        "init should succeed in an empty directory: {output:?}"
    );

    let spielgantt_dir = project_dir.path().join(".spielgantt");
    let project_metadata_path = spielgantt_dir.join("project.json");

    assert!(
        project_metadata_path.is_file(),
        "init should create .spielgantt/project.json"
    );
    assert!(
        !spielgantt_dir.join("project.yaml").exists(),
        "init should not create .spielgantt/project.yaml"
    );
    let project_metadata: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&project_metadata_path)
            .expect("project metadata should remain readable"),
    )
    .expect("project metadata should be valid JSON");
    assert_eq!(
        project_metadata["events"],
        serde_json::json!(["start", "finished"]),
        "new projects should include default timeline boundary events"
    );
    support::assert_agent_ready_project(project_dir.path());
    assert!(
        !project_dir.path().join("tasks").exists(),
        "init should not create sample task folders"
    );
}

#[test]
fn init_is_safe_to_rerun_in_an_existing_project_directory() {
    let project_dir = tempdir().expect("temporary directory should be created");
    let user_file_path = project_dir.path().join("notes.txt");
    fs::write(&user_file_path, "keep this file").expect("user file should be created");

    let first_run = run_spielgantt_in(project_dir.path(), &["init"]);
    assert!(
        first_run.status.success(),
        "first init should succeed: {first_run:?}"
    );

    let second_run = run_spielgantt_in(project_dir.path(), &["init"]);
    assert!(
        second_run.status.success(),
        "second init should also succeed: {second_run:?}"
    );
    assert!(
        stdout(&second_run).contains("already exists"),
        "second init should report that the project already exists: {}",
        stdout(&second_run)
    );

    assert_eq!(
        fs::read_to_string(&user_file_path).expect("user file should remain readable"),
        "keep this file",
        "init should not rewrite existing user files"
    );
}

#[test]
fn init_rejects_non_directory_targets_with_a_clear_error() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let file_path = workspace_dir.path().join("not-a-directory.txt");
    fs::write(&file_path, "not a project directory").expect("file target should be created");

    let output = run_spielgantt_in(workspace_dir.path(), &["init", "not-a-directory.txt"]);
    assert!(
        !output.status.success(),
        "init should fail for a non-directory target: {output:?}"
    );
    assert!(
        stderr(&output).contains("target is not a directory"),
        "init should explain why the target is invalid: {}",
        stderr(&output)
    );
}
