use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

#[test]
fn task_readme_creates_a_missing_readme_and_reports_its_path() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before locating a README: {init_output:?}"
    );

    let task_dir = project_dir.join("calibrate-laser");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::create_dir(task_dir.join(".spielgantt"))
        .expect("task metadata directory should be created");
    fs::write(
        task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\"\n}\n",
    )
    .expect("task metadata should be written");

    let readme_output = run_spielgantt_in(&project_dir, &["task", "readme", "calibrate-laser"]);
    assert!(
        readme_output.status.success(),
        "task readme should succeed for a task without a README: {readme_output:?}"
    );

    let readme_path = task_dir.join("README.md");
    assert!(
        readme_path.is_file(),
        "task readme should create a README when it is missing"
    );
    assert!(
        stdout(&readme_output).contains(&readme_path.display().to_string()),
        "task readme should report the README path: {}",
        stdout(&readme_output)
    );
}

#[test]
fn task_readme_leaves_an_existing_readme_unchanged_and_supports_dry_run_open() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before locating a README: {init_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "calibrate-laser"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before locating a README: {create_output:?}"
    );

    let readme_path = project_dir.join("calibrate-laser/README.md");
    fs::write(&readme_path, "scientist-owned prose\n")
        .expect("existing README should be writable for the test");

    let readme_output = run_spielgantt_in(&project_dir, &["task", "readme", "calibrate-laser"]);
    assert!(
        readme_output.status.success(),
        "task readme should succeed for an existing README: {readme_output:?}"
    );
    assert_eq!(
        fs::read_to_string(&readme_path).expect("existing README should remain readable"),
        "scientist-owned prose\n",
        "task readme should leave an existing README unchanged"
    );
    assert!(
        stdout(&readme_output).contains(&readme_path.display().to_string()),
        "task readme should report the existing README path: {}",
        stdout(&readme_output)
    );

    let dry_run_output = run_spielgantt_in(
        &project_dir,
        &["task", "readme", "calibrate-laser", "--open", "--dry-run"],
    );
    assert!(
        dry_run_output.status.success(),
        "task readme dry-run open should succeed: {dry_run_output:?}"
    );
    let dry_run_stdout = stdout(&dry_run_output);
    assert!(
        dry_run_stdout.contains(&format!(
            "Would open README for task 'calibrate-laser': {}",
            fs::canonicalize(&readme_path)
                .expect("existing README should canonicalize")
                .display()
        )),
        "task readme dry-run open should report the README target: {dry_run_stdout}"
    );
    assert!(
        dry_run_stdout.contains("Command:"),
        "task readme dry-run open should report the platform open command: {dry_run_stdout}"
    );
}
