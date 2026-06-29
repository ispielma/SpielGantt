use tempfile::tempdir;

#[test]
fn app_project_readme_edit_writes_content_creates_missing_file_and_rejects_stale_writes() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("application project open should include project README version");
    let result = spielgantt_lib::app_facade::edit_project_readme(
        &project_dir,
        spielgantt_lib::app_facade::ProjectReadmeEdit {
            readme_content: "# Experiment plan\n\n- first note\n".to_string(),
            expected_readme_version: opened.project_readme_version().to_string(),
        },
    )
    .expect("project README edit should create and write the root README");

    assert_eq!(
        std::fs::read_to_string(project_dir.join("README.md"))
            .expect("project README should be readable"),
        "# Experiment plan\n\n- first note\n",
        "project README edit should preserve ordinary Markdown exactly"
    );
    assert_eq!(
        result.project().project_readme_content(),
        "# Experiment plan\n\n- first note\n"
    );

    let stale_version = result.project().project_readme_version().to_string();
    std::fs::write(project_dir.join("README.md"), "# External update\n")
        .expect("external project README edit should be writable");

    let stale_result = spielgantt_lib::app_facade::edit_project_readme(
        &project_dir,
        spielgantt_lib::app_facade::ProjectReadmeEdit {
            readme_content: "# Stale application update\n".to_string(),
            expected_readme_version: stale_version,
        },
    );

    assert!(
        stale_result.is_err(),
        "stale project README edits should be rejected"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("README.md"))
            .expect("project README should remain readable"),
        "# External update\n",
        "stale application state should not overwrite external project README edits"
    );
}
