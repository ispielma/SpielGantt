use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stdout_text as stdout};

#[test]
fn task_open_supports_dry_run_without_launching_external_programs() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before dry-run open: {init_output:?}"
    );
    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "analyze-results"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before dry-run open: {create_output:?}"
    );

    let task_path = fs::canonicalize(project_dir.join("analyze-results"))
        .expect("task path should canonicalize");
    let task_dry_run_output = run_spielgantt_in(
        &project_dir,
        &["task", "open", "analyze-results", "--dry-run"],
    );
    assert!(
        task_dry_run_output.status.success(),
        "task open dry-run should succeed: {task_dry_run_output:?}"
    );
    let task_dry_run = stdout(&task_dry_run_output);
    assert!(
        task_dry_run.contains(&format!(
            "Would open task 'analyze-results': {}",
            task_path.display()
        )),
        "task open dry-run should report the resolved task path: {task_dry_run}"
    );
    assert!(
        task_dry_run.contains("Command:"),
        "task open dry-run should report the platform open command: {task_dry_run}"
    );
}
