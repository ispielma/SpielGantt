use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn task_adopt_adds_only_hidden_task_metadata_to_an_existing_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a task: {init_output:?}"
    );

    let task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&task_dir).expect("existing task directory should be created");
    let user_file_path = task_dir.join("notes.txt");
    fs::write(&user_file_path, "calibration checklist")
        .expect("existing user file should be written");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed for an existing folder in a project: {adopt_output:?}"
    );

    assert!(
        task_dir.join(".spielgantt/task.json").is_file(),
        "task adopt should add hidden task metadata"
    );
    assert_eq!(
        fs::read_to_string(&user_file_path).expect("existing user file should remain readable"),
        "calibration checklist",
        "task adopt should leave existing user files unchanged"
    );

    let entries = fs::read_dir(&task_dir)
        .expect("task directory should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    let mut entries = entries;
    entries.sort();
    assert_eq!(
        entries,
        vec![".spielgantt".to_string(), "notes.txt".to_string()],
        "task adopt should add only hidden metadata beside existing files"
    );

    assert!(
        stdout(&adopt_output).contains("Adopted task 'calibrate-laser'"),
        "task adopt should confirm the adopted task id: {}",
        stdout(&adopt_output)
    );
}

#[test]
fn task_adopt_preserves_a_human_task_name_for_an_existing_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a human-named task: {init_output:?}"
    );

    let task_dir = project_dir.join("Existing Folder");
    fs::create_dir(&task_dir).expect("existing task directory should be created");
    fs::write(task_dir.join("notes.txt"), "leave this file alone")
        .expect("existing user file should be written");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "adopt",
            "Existing Folder",
            "--id",
            "RbK New Experiment",
        ],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should accept a human task name: {adopt_output:?}"
    );

    let task_metadata = fs::read_to_string(task_dir.join(".spielgantt/task.json"))
        .expect("task metadata should be readable after adoption");
    assert!(
        task_metadata.contains("RbK New Experiment"),
        "task adopt should write the human task name into metadata: {task_metadata}"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("notes.txt"))
            .expect("existing user file should remain readable"),
        "leave this file alone",
        "task adopt should preserve existing user files"
    );
}

#[test]
fn task_adopt_rejects_duplicate_task_ids_within_a_project() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting tasks: {init_output:?}"
    );

    let first_task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&first_task_dir).expect("first task directory should be created");
    let first_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        first_adopt_output.status.success(),
        "first task adoption should succeed: {first_adopt_output:?}"
    );

    let second_task_dir = project_dir.join("results");
    fs::create_dir(&second_task_dir).expect("second task directory should be created");
    let second_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "results", "--id", "calibrate-laser"],
    );
    assert!(
        !second_adopt_output.status.success(),
        "duplicate task ids should be rejected: {second_adopt_output:?}"
    );
    assert!(
        stderr(&second_adopt_output).contains("already exists"),
        "duplicate rejection should explain the conflicting task id: {}",
        stderr(&second_adopt_output)
    );
    assert!(
        !second_task_dir.join(".spielgantt/task.json").exists(),
        "duplicate task adoption should not create metadata in the rejected folder"
    );
}

#[test]
fn task_adopt_rejects_a_name_that_collides_with_a_project_event() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a task: {init_output:?}"
    );
    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    let task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&task_dir).expect("existing task directory should be created");
    fs::write(task_dir.join("notes.txt"), "leave this alone")
        .expect("existing user file should be written");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "MOT"],
    );
    assert!(
        !adopt_output.status.success(),
        "task adopt should reject a task name that collides with a project event: {adopt_output:?}"
    );
    let adopt_error = stderr(&adopt_output);
    assert!(
        adopt_error.contains("task id 'MOT' collides with project event id 'MOT'"),
        "task adopt should explain the event collision: {adopt_error}"
    );
    assert!(
        !task_dir.join(".spielgantt/task.json").exists(),
        "task adopt should not add metadata for a colliding event id"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("notes.txt"))
            .expect("existing user file should remain readable"),
        "leave this alone",
        "task adopt should not rewrite existing user files after an event collision"
    );
}

#[test]
fn task_adopt_fails_when_the_folder_is_not_inside_a_spielgantt_project() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let task_dir = workspace_dir.path().join("analysis-notes");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "calibration checklist")
        .expect("user file should be created");

    let output = run_spielgantt_in(
        workspace_dir.path(),
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        !output.status.success(),
        "task adopt should fail outside a SpielGantt project: {output:?}"
    );
    assert!(
        stderr(&output).contains("not inside a SpielGantt project"),
        "task adopt should explain the project requirement: {}",
        stderr(&output)
    );
    assert!(
        !task_dir.join(".spielgantt/task.json").exists(),
        "task adopt should not create metadata when the folder is outside a project"
    );
}

#[test]
fn task_adopt_rejects_a_relative_path_that_resolves_outside_the_project() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let outside_dir = workspace_dir.path().join("outside-notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    fs::create_dir(&outside_dir).expect("outside directory should be created");
    fs::write(outside_dir.join("notes.txt"), "leave this alone")
        .expect("outside user file should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a task: {init_output:?}"
    );

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "adopt",
            "../outside-notes",
            "--id",
            "calibrate-laser",
        ],
    );
    assert!(
        !adopt_output.status.success(),
        "task adopt should reject a path that resolves outside the project: {adopt_output:?}"
    );
    assert!(
        stderr(&adopt_output).contains("not inside a SpielGantt project"),
        "task adopt should explain that the canonical target is outside the project: {}",
        stderr(&adopt_output)
    );
    assert!(
        !outside_dir.join(".spielgantt/task.json").exists(),
        "task adopt should not create metadata in a folder outside the project"
    );
    assert_eq!(
        fs::read_to_string(outside_dir.join("notes.txt"))
            .expect("outside user file should remain readable"),
        "leave this alone",
        "task adopt should not rewrite files in the rejected outside folder"
    );
}

#[test]
fn task_adopt_is_idempotent_when_the_same_folder_is_re_adopted_with_the_same_id() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis-notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    fs::create_dir(&task_dir).expect("task directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a task: {init_output:?}"
    );

    let first_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        first_adopt_output.status.success(),
        "first task adoption should succeed: {first_adopt_output:?}"
    );

    let task_metadata_path = task_dir.join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should exist after adoption");
    let task_dir_absolute = task_dir
        .canonicalize()
        .expect("task directory should have a canonical absolute path");

    let second_adopt_output = run_spielgantt_in(
        workspace_dir.path(),
        &[
            "task",
            "adopt",
            task_dir_absolute
                .to_str()
                .expect("canonical task path should be valid utf-8"),
            "--id",
            "calibrate-laser",
        ],
    );
    assert!(
        second_adopt_output.status.success(),
        "re-adopting the same folder with the same id should succeed: {second_adopt_output:?}"
    );
    assert!(
        stdout(&second_adopt_output).contains("already adopted"),
        "re-adopting the same folder should report idempotence: {}",
        stdout(&second_adopt_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after re-adoption"),
        original_metadata,
        "re-adoption with the same id should not rewrite task metadata"
    );
}

#[test]
fn task_adopt_rejects_re_adopting_the_same_folder_with_a_different_id() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis-notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    fs::create_dir(&task_dir).expect("task directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before adopting a task: {init_output:?}"
    );

    let first_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        first_adopt_output.status.success(),
        "first task adoption should succeed: {first_adopt_output:?}"
    );

    let task_metadata_path = task_dir.join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should exist after adoption");

    let second_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "capture-results"],
    );
    assert!(
        !second_adopt_output.status.success(),
        "re-adopting the same folder with a different id should fail: {second_adopt_output:?}"
    );
    assert!(
        stderr(&second_adopt_output).contains("already adopted"),
        "the error should explain that the folder already belongs to another task id: {}",
        stderr(&second_adopt_output)
    );
    assert!(
        stderr(&second_adopt_output).contains("calibrate-laser"),
        "the error should mention the existing task id: {}",
        stderr(&second_adopt_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path)
            .expect("task metadata should remain readable after the rejected re-adoption"),
        original_metadata,
        "re-adopting with a different id should not rewrite task metadata"
    );
}

#[test]
fn task_adopt_reports_existing_duplicate_ids_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before creating duplicate task metadata: {init_output:?}"
    );

    let first_task_dir = project_dir.join("phase-1");
    let second_task_dir = project_dir.join("phase-2");
    support::write_task_metadata(&first_task_dir, "calibrate-laser");
    support::write_task_metadata(&second_task_dir, "calibrate-laser");

    let target_task_dir = project_dir.join("new-notes");
    fs::create_dir(&target_task_dir).expect("target task directory should be created");
    fs::write(target_task_dir.join("notes.txt"), "leave this alone")
        .expect("target user file should be created");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "new-notes", "--id", "prepare-sample"],
    );
    assert!(
        !adopt_output.status.success(),
        "task adopt should reject a project that already has duplicate task ids: {adopt_output:?}"
    );

    let adopt_error = stderr(&adopt_output);
    assert!(
        adopt_error.contains("duplicate task id 'calibrate-laser'"),
        "task adopt should explain the corrupt duplicate id: {adopt_error}"
    );
    assert!(
        adopt_error.contains(&first_task_dir.display().to_string()),
        "task adopt should mention the first conflicting task path: {adopt_error}"
    );
    assert!(
        adopt_error.contains(&second_task_dir.display().to_string()),
        "task adopt should mention the second conflicting task path: {adopt_error}"
    );
    assert!(
        !target_task_dir.join(".spielgantt/task.json").exists(),
        "task adopt should not write metadata after finding project duplicate ids"
    );
    assert_eq!(
        fs::read_to_string(target_task_dir.join("notes.txt"))
            .expect("target user file should remain readable"),
        "leave this alone",
        "task adopt should not rewrite target user files after finding duplicate ids"
    );
}
