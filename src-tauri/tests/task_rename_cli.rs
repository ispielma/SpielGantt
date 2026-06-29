use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn task_rename_updates_the_task_id_references_and_normalized_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rename: {init_output:?}"
    );

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_blocker_output.status.success(),
        "blocker task create should succeed before rename: {create_blocker_output:?}"
    );
    let create_dependent_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_dependent_output.status.success(),
        "dependent task create should succeed before rename: {create_dependent_output:?}"
    );

    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be rewritten with a dependency");
    fs::write(
        project_dir.join("calibrate-laser").join("notes.txt"),
        "do not rewrite user files",
    )
    .expect("user file should be written");

    let rename_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "align-optics"],
    );
    assert!(
        rename_output.status.success(),
        "task rename should succeed: {rename_output:?}"
    );

    let renamed_task_dir = project_dir.join("align-optics");
    assert!(
        renamed_task_dir.is_dir(),
        "task rename should normalize the renamed task folder"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "task rename should remove the old normalized task folder path"
    );
    assert_eq!(
        fs::read_to_string(renamed_task_dir.join("notes.txt"))
            .expect("user file should still exist after rename"),
        "do not rewrite user files",
        "task rename should preserve user files in the renamed task folder"
    );

    let renamed_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(renamed_task_dir.join(".spielgantt/task.json"))
            .expect("renamed task metadata should be readable"),
    )
    .expect("renamed task metadata should remain valid JSON");
    assert_eq!(
        renamed_metadata.id, "align-optics",
        "renamed task metadata should contain the new id"
    );

    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(
            project_dir
                .join("analyze-results")
                .join(".spielgantt/task.json"),
        )
        .expect("dependent task metadata should be readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert!(
        dependent_metadata
            .dependencies
            .contains(&"align-optics".to_string()),
        "dependent task metadata should reference the new id"
    );
    assert!(
        !dependent_metadata
            .dependencies
            .contains(&"calibrate-laser".to_string()),
        "dependent task metadata should no longer reference the old id"
    );
    assert!(
        stdout(&rename_output).contains("Renamed task 'calibrate-laser' to 'align-optics'"),
        "task rename should report the refactor: {}",
        stdout(&rename_output)
    );
}

#[test]
fn task_rename_rejects_invalid_or_duplicate_new_ids_before_writing_changes() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rename rejection tests: {init_output:?}"
    );

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_blocker_output.status.success(),
        "first task create should succeed before rename rejection tests: {create_blocker_output:?}"
    );
    let create_existing_output =
        run_spielgantt_in(&project_dir, &["task", "create", "align-optics"]);
    assert!(
        create_existing_output.status.success(),
        "second task create should succeed before duplicate rename test: {create_existing_output:?}"
    );

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let invalid_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "Align/Optics"],
    );
    assert!(
        !invalid_output.status.success(),
        "task rename should reject an invalid new id: {invalid_output:?}"
    );
    assert!(
        stderr(&invalid_output).contains("filesystem separator"),
        "invalid-id rejection should explain the id rule: {}",
        stderr(&invalid_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "invalid-id rejection should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "invalid-id rejection should leave the original task folder in place"
    );

    let duplicate_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "align-optics"],
    );
    assert!(
        !duplicate_output.status.success(),
        "task rename should reject a duplicate new id: {duplicate_output:?}"
    );
    assert!(
        stderr(&duplicate_output).contains("already exists"),
        "duplicate-id rejection should explain the conflict: {}",
        stderr(&duplicate_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "duplicate-id rejection should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "duplicate-id rejection should leave the original task folder in place"
    );
}

#[test]
fn task_rename_rejects_new_ids_that_collide_with_project_events_before_writing_changes() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before event-collision rename test: {init_output:?}"
    );

    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("project metadata should be rewritten with events");

    let create_task_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_task_output.status.success(),
        "task create should succeed before event-collision rename test: {create_task_output:?}"
    );

    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let rename_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "START"],
    );
    assert!(
        !rename_output.status.success(),
        "task rename should reject event id collisions: {rename_output:?}"
    );
    assert!(
        stderr(&rename_output).contains("collides with project event id 'START'"),
        "event-collision rejection should mention the shared namespace rule: {}",
        stderr(&rename_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "event-collision rejection should not rewrite task metadata"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "event-collision rejection should leave the original task folder in place"
    );
}

#[test]
fn task_rename_to_the_same_id_keeps_the_existing_task() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before same-id rename test: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before same-id rename test: {create_output:?}"
    );
    let task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_metadata =
        fs::read_to_string(&task_metadata_path).expect("task metadata should be readable");

    let rename_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "calibrate-laser"],
    );
    assert!(
        rename_output.status.success(),
        "task rename should allow keeping the same task id: {rename_output:?}"
    );
    assert!(
        stdout(&rename_output).contains("Renamed task 'calibrate-laser' to 'calibrate-laser'"),
        "same-id rename should keep the public rename report: {}",
        stdout(&rename_output)
    );
    assert_eq!(
        fs::read_to_string(&task_metadata_path).expect("task metadata should remain readable"),
        original_metadata,
        "same-id rename should leave task metadata unchanged"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "same-id rename should leave the task folder in place"
    );
}

#[test]
fn task_rename_folder_collision_does_not_leave_partial_reference_updates() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rename collision test: {init_output:?}"
    );

    let create_blocker_output =
        run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_blocker_output.status.success(),
        "blocker task create should succeed before rename collision test: {create_blocker_output:?}"
    );
    let create_dependent_output =
        run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_dependent_output.status.success(),
        "dependent task create should succeed before rename collision test: {create_dependent_output:?}"
    );

    let dependent_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/task.json");
    fs::write(
        &dependent_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"calibrate-laser\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be rewritten with a dependency");
    let original_dependent_metadata = fs::read_to_string(&dependent_metadata_path)
        .expect("dependent metadata should be readable");

    let renamed_task_metadata_path = project_dir
        .join("calibrate-laser")
        .join(".spielgantt/task.json");
    let original_renamed_task_metadata = fs::read_to_string(&renamed_task_metadata_path)
        .expect("renamed task metadata should be readable");

    let colliding_folder = project_dir.join("align-optics");
    fs::create_dir(&colliding_folder).expect("colliding folder should be created");
    fs::write(
        colliding_folder.join("notes.txt"),
        "leave this folder alone",
    )
    .expect("colliding folder user file should be written");

    let rename_output = run_spielgantt_in(
        &project_dir,
        &["task", "rename", "calibrate-laser", "align-optics"],
    );
    assert!(
        !rename_output.status.success(),
        "task rename should reject a normalized folder collision: {rename_output:?}"
    );
    assert!(
        stderr(&rename_output).contains("target folder"),
        "folder collision should explain the target conflict: {}",
        stderr(&rename_output)
    );
    assert_eq!(
        fs::read_to_string(&renamed_task_metadata_path)
            .expect("renamed task metadata should remain readable"),
        original_renamed_task_metadata,
        "folder collision should not rewrite the task's own metadata"
    );
    assert_eq!(
        fs::read_to_string(&dependent_metadata_path)
            .expect("dependent task metadata should remain readable"),
        original_dependent_metadata,
        "folder collision should not rewrite dependency references"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "folder collision should leave the original task folder in place"
    );
    assert_eq!(
        fs::read_to_string(colliding_folder.join("notes.txt"))
            .expect("colliding folder user file should remain readable"),
        "leave this folder alone",
        "folder collision should leave the colliding folder untouched"
    );
}

#[test]
fn task_rename_rolls_back_metadata_and_folder_when_later_reference_write_fails() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before rename rollback test: {init_output:?}"
    );

    let create_source_output = run_spielgantt_in(&project_dir, &["task", "create", "alpha"]);
    assert!(
        create_source_output.status.success(),
        "source task create should succeed before rename rollback test: {create_source_output:?}"
    );
    let create_dependent_output = run_spielgantt_in(&project_dir, &["task", "create", "zeta"]);
    assert!(
        create_dependent_output.status.success(),
        "dependent task create should succeed before rename rollback test: {create_dependent_output:?}"
    );

    let source_metadata_path = project_dir.join("alpha").join(".spielgantt/task.json");
    let dependent_metadata_path = project_dir.join("zeta").join(".spielgantt/task.json");
    fs::write(
        &dependent_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"zeta\",\n  \"dependencies\": [\n    \"alpha\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be rewritten with a dependency");
    let original_source_metadata =
        fs::read_to_string(&source_metadata_path).expect("source metadata should be readable");
    let original_dependent_metadata = fs::read_to_string(&dependent_metadata_path)
        .expect("dependent metadata should be readable");

    fs::create_dir(project_dir.join("zeta").join(".spielgantt/task.json.tmp"))
        .expect("temporary metadata collision directory should be created");

    let rename_output = run_spielgantt_in(&project_dir, &["task", "rename", "alpha", "beta"]);
    assert!(
        !rename_output.status.success(),
        "task rename should fail when a later dependency rewrite fails: {rename_output:?}"
    );
    assert!(
        stderr(&rename_output).contains("failed to write temporary task metadata"),
        "task rename failure should surface the failed metadata write: {}",
        stderr(&rename_output)
    );

    assert!(
        project_dir.join("alpha").is_dir(),
        "failed task rename should restore the original task folder"
    );
    assert!(
        !project_dir.join("beta").exists(),
        "failed task rename should not leave the renamed task folder behind"
    );
    assert_eq!(
        fs::read_to_string(&source_metadata_path)
            .expect("source metadata should remain readable after failed rename"),
        original_source_metadata,
        "failed task rename should restore the renamed task metadata"
    );
    assert_eq!(
        fs::read_to_string(&dependent_metadata_path)
            .expect("dependent metadata should remain readable after failed rename"),
        original_dependent_metadata,
        "failed task rename should preserve the dependent task metadata"
    );
}
