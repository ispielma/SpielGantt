use std::fs;

use tempfile::tempdir;

mod support;

#[test]
fn task_adoptable_folders_json_lists_only_eligible_project_child_folders() {
    let workspace_dir = tempdir().expect("workspace should be created");
    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "project init should succeed: {init_output:?}"
    );
    let project_dir = workspace_dir.path().join("project");

    fs::create_dir(project_dir.join("zeta notes")).expect("candidate folder should be created");
    fs::create_dir(project_dir.join("Alpha Samples")).expect("candidate folder should be created");
    fs::create_dir(project_dir.join(".hidden")).expect("hidden folder should be created");
    fs::create_dir(project_dir.join("invalid*name"))
        .expect("invalid candidate folder should be created");
    let existing_task_output = support::create_task(&project_dir, "existing-task");
    assert!(
        existing_task_output.status.success(),
        "existing task fixture should be created: {existing_task_output:?}"
    );

    let output = support::run_spielgantt_in(&project_dir, &["task", "adoptable-folders", "--json"]);
    assert!(
        output.status.success(),
        "adoptable folder JSON command should succeed: {output:?}"
    );

    let json = support::stdout_json(&output);
    assert_eq!(json["schema_version"], 1);
    let candidates = support::json_array(&json, "folders");
    assert_eq!(
        candidates.len(),
        2,
        "only adoptable folders should be listed: {json}"
    );
    assert_eq!(candidates[0]["projectRelativePath"], "Alpha Samples");
    assert_eq!(candidates[0]["taskId"], "Alpha Samples");
    assert!(candidates[0]["folderPath"]
        .as_str()
        .expect("folderPath should be a string")
        .ends_with("/Alpha Samples"));
    assert_eq!(candidates[1]["projectRelativePath"], "zeta notes");
    assert_eq!(candidates[1]["taskId"], "zeta notes");
}
