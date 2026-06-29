use std::fs;

mod support;

use tempfile::tempdir;

#[test]
fn task_create_makes_a_task_folder_with_hidden_metadata_and_a_readme() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating a task: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    let create_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        create_output.status.success(),
        "task create should succeed in a SpielGantt project: {create_output:?}"
    );

    let task_dir = project_dir.join("calibrate-laser");
    assert!(
        task_dir.is_dir(),
        "task create should derive the folder name from the task id"
    );
    assert!(
        task_dir.join(".spielgantt/task.json").is_file(),
        "task create should write hidden task metadata"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("README.md"))
            .expect("task README should be created and readable"),
        "# calibrate-laser\n",
        "task create should add a minimal README heading based on the task id"
    );
    assert!(
        support::stdout_text(&create_output).contains("Created task 'calibrate-laser'"),
        "task create should confirm the created task id: {}",
        support::stdout_text(&create_output)
    );
}

#[test]
fn task_create_preserves_a_human_task_name_in_the_folder_and_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating a human-named task: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    let task_name = "RbK New Experiment";
    let create_output = support::create_task(&project_dir, task_name);
    assert!(
        create_output.status.success(),
        "task create should accept a human task name: {create_output:?}"
    );

    let task_dir = project_dir.join(task_name);
    assert!(
        task_dir.is_dir(),
        "task create should use the task name as the folder name"
    );

    let task_metadata = fs::read_to_string(task_dir.join(".spielgantt/task.json"))
        .expect("task metadata should be readable");
    assert!(
        task_metadata.contains("RbK New Experiment"),
        "task create should write the human task name into metadata: {task_metadata}"
    );
    assert!(
        fs::read_to_string(task_dir.join("README.md"))
            .expect("task README should be readable")
            .contains("# RbK New Experiment"),
        "task create should use the human task name in the README heading"
    );
}

#[test]
fn task_create_rejects_a_duplicate_task_id_even_when_the_folder_name_is_free() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let legacy_task_dir = project_dir.join("legacy-notes");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating a task: {init_output:?}"
    );

    fs::create_dir(&legacy_task_dir).expect("legacy task directory should be created");

    let adopt_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "adopt", "legacy-notes", "--id", "calibrate-laser"],
    );
    assert!(
        adopt_output.status.success(),
        "task adopt should succeed before the duplicate-id check: {adopt_output:?}"
    );

    let create_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        !create_output.status.success(),
        "task create should reject a duplicate task id: {create_output:?}"
    );
    assert!(
        support::stderr_text(&create_output).contains("already exists"),
        "task create should explain the duplicate task id: {}",
        support::stderr_text(&create_output)
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "task create should not create a new folder for a duplicate task id"
    );
}

#[test]
fn task_create_rejects_a_name_that_collides_with_a_project_event() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating a task: {init_output:?}"
    );

    let project_dir = workspace_dir.path().join("project");
    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    let create_output = support::create_task(&project_dir, "MOT");
    assert!(
        !create_output.status.success(),
        "task create should reject a task name that collides with a project event: {create_output:?}"
    );
    let create_error = support::stderr_text(&create_output);
    assert!(
        create_error.contains("task id 'MOT' collides with project event id 'MOT'"),
        "task create should explain the event collision: {create_error}"
    );
    assert!(
        !project_dir.join("MOT").exists(),
        "task create should not create a folder for a colliding event id"
    );
}

#[test]
fn task_create_rejects_a_colliding_folder_name_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating a task: {init_output:?}"
    );

    let colliding_task_dir = project_dir.join("calibrate-laser");
    fs::create_dir(&colliding_task_dir).expect("colliding folder should be created");
    fs::write(colliding_task_dir.join("notes.txt"), "leave this alone")
        .expect("user file in colliding folder should be written");

    let create_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        !create_output.status.success(),
        "task create should reject an existing folder with the derived name: {create_output:?}"
    );
    assert!(
        support::stderr_text(&create_output).contains("already exists"),
        "task create should explain the folder name collision: {}",
        support::stderr_text(&create_output)
    );
    assert!(
        !colliding_task_dir.join(".spielgantt/task.json").exists(),
        "task create should not add metadata to a colliding existing folder"
    );
    assert_eq!(
        fs::read_to_string(colliding_task_dir.join("notes.txt"))
            .expect("user file in the colliding folder should remain readable"),
        "leave this alone",
        "task create should not rewrite files in a colliding existing folder"
    );
}

#[test]
fn task_create_reports_existing_duplicate_ids_before_creating_a_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating duplicate task metadata: {init_output:?}"
    );

    let first_task_dir = project_dir.join("phase-1");
    let second_task_dir = project_dir.join("phase-2");
    support::write_task_metadata(&first_task_dir, "calibrate-laser");
    support::write_task_metadata(&second_task_dir, "calibrate-laser");

    let create_output = support::create_task(&project_dir, "prepare-sample");
    assert!(
        !create_output.status.success(),
        "task create should reject a project that already has duplicate task ids: {create_output:?}"
    );

    let create_error = support::stderr_text(&create_output);
    assert!(
        create_error.contains("duplicate task id 'calibrate-laser'"),
        "task create should explain the corrupt duplicate id: {create_error}"
    );
    assert!(
        create_error.contains(&first_task_dir.display().to_string()),
        "task create should mention the first conflicting task path: {create_error}"
    );
    assert!(
        create_error.contains(&second_task_dir.display().to_string()),
        "task create should mention the second conflicting task path: {create_error}"
    );
    assert!(
        !project_dir.join("prepare-sample").exists(),
        "task create should not create a new task folder in a project with duplicate ids"
    );
}
