use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn normalize_reports_a_dry_run_plan_before_changing_task_folders() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis-notes");

    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before normalize: {init_output:?}"
    );

    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed before normalize: {adopt_output:?}"
    );

    let normalize_output = run_spielgantt_in(&project_dir, &["normalize"]);
    assert!(
        normalize_output.status.success(),
        "normalize dry-run should succeed: {normalize_output:?}"
    );

    let normalize_stdout = stdout(&normalize_output);
    assert!(
        normalize_stdout.contains("Dry run"),
        "normalize should label the default mode as a dry run: {normalize_stdout}"
    );
    assert!(
        normalize_stdout.contains("analysis-notes"),
        "normalize should mention the current folder name in the plan: {normalize_stdout}"
    );
    assert!(
        normalize_stdout.contains("calibrate-laser"),
        "normalize should mention the normalized folder name in the plan: {normalize_stdout}"
    );
    assert!(
        task_dir.is_dir(),
        "normalize dry-run should leave the original task folder in place"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "normalize dry-run should not rename the task folder"
    );
}

#[test]
fn normalize_apply_renames_task_folders_and_keeps_user_files_intact() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis-notes");

    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before normalize apply: {init_output:?}"
    );

    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");

    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed before normalize apply: {adopt_output:?}"
    );

    let normalize_output = run_spielgantt_in(&project_dir, &["normalize", "--apply"]);
    assert!(
        normalize_output.status.success(),
        "normalize apply should succeed: {normalize_output:?}"
    );

    let normalized_task_dir = project_dir.join("calibrate-laser");
    assert!(
        normalized_task_dir.is_dir(),
        "normalize apply should rename the task folder to the task id"
    );
    assert!(
        !task_dir.exists(),
        "normalize apply should remove the old task folder path"
    );
    assert_eq!(
        fs::read_to_string(normalized_task_dir.join("notes.txt"))
            .expect("user file should still exist after the rename"),
        "keep this file",
        "normalize apply should preserve user files inside the renamed folder"
    );
    assert!(
        normalized_task_dir.join(".spielgantt/task.json").is_file(),
        "normalize apply should preserve task metadata in the renamed folder"
    );
    assert!(
        stdout(&normalize_output).contains("Renamed"),
        "normalize apply should report the rename: {}",
        stdout(&normalize_output)
    );
}

#[test]
fn normalize_apply_detects_collisions_before_renaming_any_task_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before collision test: {init_output:?}"
    );

    let first_task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&first_task_dir).expect("first task directory should be created");
    fs::write(first_task_dir.join("notes.txt"), "keep this file")
        .expect("first task user file should be written");
    let first_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        first_adopt_output.status.success(),
        "first task adopt should succeed: {first_adopt_output:?}"
    );

    let second_task_dir = project_dir.join("scratch-results");
    fs::create_dir(&second_task_dir).expect("second task directory should be created");
    let second_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "scratch-results", "--id", "prepare-sample"],
    );
    assert!(
        second_adopt_output.status.success(),
        "second task adopt should succeed: {second_adopt_output:?}"
    );

    let colliding_dir = project_dir.join("calibrate-laser");
    fs::create_dir(&colliding_dir).expect("colliding directory should be created");
    fs::write(colliding_dir.join("notes.txt"), "do not touch")
        .expect("colliding directory file should be written");

    let normalize_output = run_spielgantt_in(&project_dir, &["normalize", "--apply"]);
    assert!(
        !normalize_output.status.success(),
        "normalize apply should fail when a target folder already exists: {normalize_output:?}"
    );

    let normalize_stderr = stderr(&normalize_output);
    assert!(
        normalize_stderr.contains("already exists"),
        "normalize apply should explain the colliding target path: {normalize_stderr}"
    );
    assert!(
        first_task_dir.is_dir(),
        "normalize apply should not rename the first task when any collision exists"
    );
    assert!(
        second_task_dir.is_dir(),
        "normalize apply should not rename unrelated tasks when any collision exists"
    );
    assert_eq!(
        fs::read_to_string(colliding_dir.join("notes.txt"))
            .expect("colliding directory file should remain readable"),
        "do not touch",
        "normalize apply should leave the colliding directory unchanged"
    );
}

#[test]
fn normalize_dry_run_reports_preflight_collisions_without_renaming_any_task_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before dry-run collision test: {init_output:?}"
    );

    let task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file")
        .expect("task user file should be written");
    let adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "analysis-notes", "--id", "calibrate-laser"],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed: {adopt_output:?}"
    );

    let colliding_dir = project_dir.join("calibrate-laser");
    fs::create_dir(&colliding_dir).expect("colliding directory should be created");
    fs::write(colliding_dir.join("notes.txt"), "do not touch")
        .expect("colliding directory file should be written");

    let normalize_output = run_spielgantt_in(&project_dir, &["normalize"]);
    assert!(
        !normalize_output.status.success(),
        "normalize dry-run should fail when the planned rename is unsafe: {normalize_output:?}"
    );

    let normalize_stdout = stdout(&normalize_output);
    assert!(
        normalize_stdout.contains("Dry run: planned task folder renames"),
        "normalize dry-run should still show the planned rename before reporting preflight issues: {normalize_stdout}"
    );
    let normalize_stderr = stderr(&normalize_output);
    assert!(
        normalize_stderr.contains("already exists"),
        "normalize dry-run should explain the colliding target path: {normalize_stderr}"
    );
    assert!(
        task_dir.is_dir(),
        "normalize dry-run should not rename the task folder"
    );
    assert_eq!(
        fs::read_to_string(colliding_dir.join("notes.txt"))
            .expect("colliding directory file should remain readable"),
        "do not touch",
        "normalize dry-run should leave the colliding directory unchanged"
    );
}

#[test]
fn normalize_reports_final_paths_for_nested_task_renames() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let parent_task_dir = project_dir.join("phase notes");
    let nested_task_dir = parent_task_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before nested normalize: {init_output:?}"
    );

    fs::create_dir(&parent_task_dir).expect("parent task directory should be created");
    fs::write(parent_task_dir.join("overview.txt"), "parent user file")
        .expect("parent user file should be written");
    let parent_adopt_output = run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "phase notes", "--id", "phase-1"],
    );
    assert!(
        parent_adopt_output.status.success(),
        "parent task adopt should succeed before nested normalize: {parent_adopt_output:?}"
    );

    fs::create_dir(&nested_task_dir).expect("nested task directory should be created");
    fs::write(nested_task_dir.join("notes.txt"), "nested user file")
        .expect("nested user file should be written");
    let nested_adopt_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "adopt",
            "phase notes/analysis notes",
            "--id",
            "analyze-results",
        ],
    );
    assert!(
        nested_adopt_output.status.success(),
        "nested task adopt should succeed before nested normalize: {nested_adopt_output:?}"
    );

    let dry_run_output = run_spielgantt_in(&project_dir, &["normalize"]);
    assert!(
        dry_run_output.status.success(),
        "nested normalize dry-run should succeed: {dry_run_output:?}"
    );

    let dry_run_stdout = stdout(&dry_run_output);
    let final_nested_path = project_dir.join("phase-1").join("analyze-results");
    assert!(
        dry_run_stdout.contains(&final_nested_path.display().to_string()),
        "nested dry-run should report the nested task's final path after parent normalization: {dry_run_stdout}"
    );

    let apply_output = run_spielgantt_in(&project_dir, &["normalize", "--apply"]);
    assert!(
        apply_output.status.success(),
        "nested normalize apply should succeed: {apply_output:?}"
    );

    assert!(
        final_nested_path.join(".spielgantt/task.json").is_file(),
        "nested normalize apply should leave nested task metadata at the final normalized path"
    );
    assert_eq!(
        fs::read_to_string(final_nested_path.join("notes.txt"))
            .expect("nested user file should remain readable"),
        "nested user file",
        "nested normalize apply should preserve user files"
    );
    assert!(
        project_dir.join("phase-1").join("overview.txt").is_file(),
        "nested normalize apply should preserve parent task user files"
    );
}
