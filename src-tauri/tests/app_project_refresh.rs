use tempfile::tempdir;

#[test]
fn app_project_refresh_reloads_external_metadata_and_readme_edits() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate-laser")
        .expect("fixture task creation should succeed");

    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include initial task state");
    let initial_task = opened
        .tasks()
        .iter()
        .find(|task| task.id() == "calibrate-laser")
        .expect("initial project should include calibrate-laser");
    assert_eq!(initial_task.status(), None);
    assert_eq!(initial_task.readme_content(), "# calibrate-laser\n");

    let task_metadata_path = project_dir.join("calibrate-laser/.spielgantt/task.json");
    std::fs::write(
        &task_metadata_path,
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\",\n  \"status\": \"blocked\"\n}\n",
    )
    .expect("external metadata edit should be writable");
    std::fs::write(
        project_dir.join("calibrate-laser/README.md"),
        "# Calibrate laser\n\nexternal notebook note\n",
    )
    .expect("external README edit should be writable");

    let refreshed = spielgantt_lib::app_facade::refresh_project(&project_dir)
        .expect("application project refresh should reload state from disk");
    let refreshed_task = refreshed
        .project()
        .tasks()
        .iter()
        .find(|task| task.id() == "calibrate-laser")
        .expect("refreshed project should include calibrate-laser");

    assert_eq!(
        refreshed_task.status(),
        Some(&spielgantt_lib::metadata::TaskStatus::Blocked)
    );
    assert_eq!(
        refreshed_task.readme_content(),
        "# Calibrate laser\n\nexternal notebook note\n"
    );
    assert_ne!(
        refreshed_task.readme_version(),
        initial_task.readme_version(),
        "README version should change when external README content changes"
    );
}

#[test]
fn app_project_refresh_reports_missing_paths_as_invalid_project_results() {
    let workspace = tempdir().expect("workspace should be created");
    let missing_dir = workspace.path().join("moved-project");

    let refreshed = spielgantt_lib::app_facade::refresh_project(&missing_dir)
        .expect("application project refresh should return invalid status for missing paths");

    assert_eq!(refreshed.project().selected_path(), missing_dir.as_path());
    assert_eq!(refreshed.project().project_root(), None);
    assert!(!refreshed.project().is_valid());
    assert!(refreshed.project().tasks().is_empty());
    assert!(refreshed.project().events().is_empty());
    assert!(
        refreshed
            .project()
            .issues()
            .iter()
            .any(|issue| issue.contains("missing project metadata")),
        "missing project should report the shared validation message: {refreshed:?}"
    );
}
