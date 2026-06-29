use std::fs;

use tempfile::tempdir;

mod support;

#[test]
fn app_facade_create_project_in_parent_creates_and_opens_a_project_in_that_destination() {
    let workspace = tempdir().expect("workspace should be created");
    let projects_root = workspace.path().join("chosen projects");
    fs::create_dir_all(&projects_root).expect("chosen parent should be created");

    let project_name = "Chosen Destination Experiment";
    let result = spielgantt_lib::app_facade::create_project_in_parent(project_name, &projects_root)
        .expect("app facade project creation should succeed in the chosen parent");
    let project_root = projects_root.join(project_name);
    let canonical_project_root =
        fs::canonicalize(&project_root).expect("project root should canonicalize");

    assert_eq!(result.selected_path(), project_root.as_path());
    assert_eq!(
        result.project_root(),
        Some(canonical_project_root.as_path()),
        "project creation should open the project created in the chosen parent"
    );
    assert!(result.is_valid());
    assert!(result.issues().is_empty());
    assert!(
        project_root.join(".spielgantt/project.json").is_file(),
        "app facade project creation should initialize metadata in the chosen destination"
    );
    support::assert_agent_ready_project(&project_root);
}

#[test]
fn app_facade_create_project_rejects_invalid_project_names() {
    let workspace = tempdir().expect("workspace should be created");
    let projects_root = workspace.path().join("chosen projects");
    fs::create_dir_all(&projects_root).expect("chosen parent should be created");

    let error = spielgantt_lib::app_facade::create_project_in_parent("phase/name", &projects_root)
        .expect_err("invalid project names should be rejected");

    assert!(
        error.to_string().contains("project name"),
        "invalid project names should explain the filesystem-name rule: {error}"
    );
}

#[test]
fn app_facade_create_project_rejects_existing_project_directories() {
    let workspace = tempdir().expect("workspace should be created");
    let projects_root = workspace.path().join("chosen projects");
    fs::create_dir_all(&projects_root).expect("chosen parent should be created");

    let project_name = "Experiment Plan";
    let created =
        spielgantt_lib::app_facade::create_project_in_parent(project_name, &projects_root)
            .expect("initial project creation should succeed");
    let project_root = projects_root.join(project_name);
    let canonical_project_root =
        fs::canonicalize(&project_root).expect("project root should canonicalize");

    assert_eq!(created.selected_path(), project_root.as_path());
    assert_eq!(
        created.project_root(),
        Some(canonical_project_root.as_path())
    );

    let error = spielgantt_lib::app_facade::create_project_in_parent(project_name, &projects_root)
        .expect_err("duplicate project creation should fail");
    assert!(
        error.to_string().contains("already exists"),
        "duplicate project creation should report the existing path: {error}"
    );
    assert!(
        project_root.join(".spielgantt/project.json").is_file(),
        "duplicate project creation should not remove the initialized project metadata"
    );
}

#[test]
fn app_facade_create_project_in_parent_rejects_existing_empty_project_directory_without_metadata() {
    let workspace = tempdir().expect("workspace should be created");
    let projects_root = workspace.path().join("chosen projects");
    let project_root = projects_root.join("Existing Folder");
    fs::create_dir_all(&project_root).expect("existing project folder should be created");

    let error =
        spielgantt_lib::app_facade::create_project_in_parent("Existing Folder", &projects_root)
            .expect_err("existing project folders should be rejected before metadata creation");

    assert!(
        error.to_string().contains("already exists"),
        "collision should report the existing destination: {error}"
    );
    assert!(
        !project_root.join(".spielgantt").exists(),
        "collision rejection should not create partial SpielGantt metadata"
    );
}

#[test]
fn app_facade_create_project_in_parent_reports_unusable_parent_without_metadata() {
    let workspace = tempdir().expect("workspace should be created");
    let unusable_parent = workspace.path().join("not a directory");
    fs::write(&unusable_parent, "not a directory").expect("unusable parent file should be created");

    let error =
        spielgantt_lib::app_facade::create_project_in_parent("Experiment Plan", &unusable_parent)
            .expect_err("unusable parent destinations should be rejected");
    let project_root = unusable_parent.join("Experiment Plan");

    assert!(
        error
            .to_string()
            .contains("failed to create projects directory"),
        "unusable parent should be reported before project initialization: {error}"
    );
    assert!(
        !project_root.join(".spielgantt").exists(),
        "access failure should not create partial SpielGantt metadata"
    );
}
