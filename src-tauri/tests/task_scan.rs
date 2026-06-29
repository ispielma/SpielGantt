use std::fs;

use tempfile::tempdir;

#[test]
fn task_scan_returns_task_ids_with_current_folder_paths_and_ignores_non_task_folders() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project initialization should succeed");

    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("task creation should succeed");

    let adopted_task_dir = project_dir.join("analysis-notes");
    fs::create_dir(&adopted_task_dir).expect("adopted task directory should be created");
    fs::write(adopted_task_dir.join("notes.txt"), "keep this file")
        .expect("user file in adopted task should be written");
    spielgantt_lib::task::adopt(&adopted_task_dir, "prepare-sample")
        .expect("task adoption should succeed");

    let ignored_dir = project_dir.join("scratch");
    fs::create_dir(&ignored_dir).expect("non-task directory should be created");
    fs::write(ignored_dir.join("notes.txt"), "not a task")
        .expect("non-task user file should be written");

    let scanned_tasks = spielgantt_lib::task::scan(&project_dir).expect("task scan should work");
    let mut scanned_tasks = scanned_tasks
        .iter()
        .map(|task| (task.id(), task.path()))
        .collect::<Vec<_>>();
    scanned_tasks.sort_by(|left, right| left.0.cmp(right.0));

    assert_eq!(
        scanned_tasks,
        vec![
            (
                "calibrate-laser",
                project_dir.join("calibrate-laser").as_path()
            ),
            ("prepare-sample", adopted_task_dir.as_path()),
        ],
        "scan should return each task id with its current folder path"
    );
}

#[test]
fn task_scan_supports_nested_task_buckets_and_reports_their_actual_paths() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project initialization should succeed");

    spielgantt_lib::task::create(&project_dir, "phase-1").expect("parent task should be created");

    let nested_task_dir = project_dir.join("phase-1").join("analysis");
    fs::create_dir(&nested_task_dir).expect("nested task directory should be created");
    fs::write(nested_task_dir.join("notes.txt"), "still a user file")
        .expect("nested task user file should be written");
    spielgantt_lib::task::adopt(&nested_task_dir, "analyze-results")
        .expect("nested task adoption should succeed");

    let scanned_tasks = spielgantt_lib::task::scan(&project_dir)
        .expect("task scan should support nested task buckets");
    let mut scanned_tasks = scanned_tasks
        .iter()
        .map(|task| (task.id(), task.path()))
        .collect::<Vec<_>>();
    scanned_tasks.sort_by(|left, right| left.0.cmp(right.0));

    assert_eq!(
        scanned_tasks,
        vec![
            ("analyze-results", nested_task_dir.as_path()),
            ("phase-1", project_dir.join("phase-1").as_path()),
        ],
        "scan should include both parent and nested task folders with their actual paths"
    );
}

#[test]
fn task_scan_rejects_duplicate_task_ids_with_both_conflicting_paths() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project initialization should succeed");

    let first_task_dir = project_dir.join("phase-1");
    fs::create_dir(&first_task_dir).expect("first task directory should be created");
    fs::create_dir(first_task_dir.join(".spielgantt"))
        .expect("first task metadata directory should be created");
    fs::write(
        first_task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("first task metadata should be written");

    let second_task_dir = project_dir.join("phase-2");
    fs::create_dir(&second_task_dir).expect("second task directory should be created");
    fs::create_dir(second_task_dir.join(".spielgantt"))
        .expect("second task metadata directory should be created");
    fs::write(
        second_task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("second task metadata should be written");

    let error = spielgantt_lib::task::scan(&project_dir)
        .expect_err("task scan should reject duplicate task ids");
    let rendered_error = error.to_string();

    match error {
        spielgantt_lib::task::ScanTasksError::DuplicateTaskId {
            id,
            first_path,
            second_path,
        } => {
            assert_eq!(id, "calibrate-laser");
            assert_eq!(first_path, first_task_dir);
            assert_eq!(second_path, second_task_dir);
        }
        other => panic!("unexpected scan error: {other}"),
    }
    assert!(
        rendered_error.contains("duplicate task id 'calibrate-laser'"),
        "scan should explain the duplicate task id: {rendered_error}"
    );
    assert!(
        rendered_error.contains(&first_task_dir.display().to_string()),
        "scan should mention the first conflicting task path: {rendered_error}"
    );
    assert!(
        rendered_error.contains(&second_task_dir.display().to_string()),
        "scan should mention the second conflicting task path: {rendered_error}"
    );
}
