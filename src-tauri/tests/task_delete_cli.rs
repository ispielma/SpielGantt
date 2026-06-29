use std::fs;

use tempfile::tempdir;

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_json, stdout_text as stdout};

#[test]
fn task_delete_remove_from_chart_preserves_user_files_and_heals_dependencies_by_cli() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before deleting a task: {init_output:?}"
    );

    for task_id in ["collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    fs::write(
        project_dir.join("collect-samples").join("notes.txt"),
        "keep user notes",
    )
    .expect("user task file should be written");
    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before task delete: {depend_output:?}"
    );

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "collect-samples", "--remove-from-chart"],
    );
    assert!(
        delete_output.status.success(),
        "task delete should succeed: {delete_output:?}"
    );

    assert!(
        !project_dir.join("collect-samples/.spielgantt").exists(),
        "remove from chart should delete task metadata"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join("collect-samples/notes.txt"))
            .expect("user task file should remain readable"),
        "keep user notes"
    );
    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(project_dir.join("analyze-results/.spielgantt/task.json"))
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert!(
        dependent_metadata.dependencies.is_empty(),
        "task delete should remove stale task dependencies"
    );
    assert!(
        stdout(&delete_output).contains("Removed task 'collect-samples' from chart"),
        "task delete should confirm the safe removal mode: {}",
        stdout(&delete_output)
    );
}

#[test]
fn task_delete_remove_from_chart_reconnects_dependents_to_deleted_task_blockers() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before deleting a task: {init_output:?}"
    );
    for event_id in ["START", "DONE"] {
        let event_output = run_spielgantt_in(&project_dir, &["event", "create", event_id]);
        assert!(
            event_output.status.success(),
            "event create should succeed for {event_id}: {event_output:?}"
        );
    }
    for task_id in ["make chart", "literature-review"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }

    let make_chart_start =
        run_spielgantt_in(&project_dir, &["task", "depend", "make chart", "START"]);
    assert!(
        make_chart_start.status.success(),
        "make chart should depend on START before deletion: {make_chart_start:?}"
    );
    let literature_make_chart = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "literature-review", "make chart"],
    );
    assert!(
        literature_make_chart.status.success(),
        "literature-review should depend on make chart before deletion: {literature_make_chart:?}"
    );
    let literature_done = run_spielgantt_in(
        &project_dir,
        &["task", "ends-at", "literature-review", "DONE"],
    );
    assert!(
        literature_done.status.success(),
        "literature-review should end at DONE before deletion: {literature_done:?}"
    );

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "make chart", "--remove-from-chart"],
    );
    assert!(
        delete_output.status.success(),
        "task delete should succeed: {delete_output:?}"
    );

    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(project_dir.join("literature-review/.spielgantt/task.json"))
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert_eq!(
        dependent_metadata.dependencies,
        vec!["START".to_string()],
        "dependent task should adopt the deleted task's blockers"
    );
    assert_eq!(
        dependent_metadata.ends_at.as_deref(),
        Some("DONE"),
        "dependent task should keep its own end event"
    );
}

#[test]
fn task_delete_directory_removes_task_bucket_by_cli() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before deleting a task directory: {init_output:?}"
    );

    for task_id in ["collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before directory deletion: {depend_output:?}"
    );

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "collect-samples", "--delete-directory"],
    );
    assert!(
        delete_output.status.success(),
        "task directory delete should succeed: {delete_output:?}"
    );

    assert!(
        !project_dir.join("collect-samples").exists(),
        "delete directory should remove the task bucket"
    );
    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(project_dir.join("analyze-results/.spielgantt/task.json"))
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert!(
        dependent_metadata.dependencies.is_empty(),
        "directory deletion should remove stale task dependencies"
    );
    assert!(
        stdout(&delete_output).contains("Deleted task directory for 'collect-samples'"),
        "task delete should confirm directory deletion: {}",
        stdout(&delete_output)
    );
}

#[test]
fn staged_task_directory_is_not_loaded_as_active_task_by_cli() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before staged path scan checks: {init_output:?}"
    );

    for task_id in ["collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let depend_output = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before staged path scan checks: {depend_output:?}"
    );

    support::write_task_metadata(
        &project_dir.join(".spielgantt-delete-staged-task-manual"),
        "collect-samples",
    );

    let task_list_json = support::run_spielgantt_json_in(&project_dir, &["task", "list", "--json"]);
    assert_eq!(
        task_list_json["tasks"],
        serde_json::json!([
            {"id": "analyze-results", "status": null},
            {"id": "collect-samples", "status": null}
        ]),
        "task list should ignore staged task buckets"
    );

    let validate_json = support::run_spielgantt_json_in(&project_dir, &["validate", "--json"]);
    assert_eq!(
        validate_json["valid"], true,
        "validate should ignore staged task buckets"
    );

    let snapshot_json =
        support::run_spielgantt_json_in(&project_dir, &["agent", "snapshot", "--json"]);
    assert_eq!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .len(),
        2,
        "agent snapshot should ignore staged task buckets"
    );
    assert!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .iter()
            .all(|task| !task["project_relative_path"]
                .as_str()
                .expect("snapshot task should expose a project-relative path")
                .contains(".spielgantt-delete-staged-")),
        "agent snapshot should not expose staged task bucket paths"
    );

    let workflow_json =
        support::run_spielgantt_json_in(&project_dir, &["task", "workflow", "--json"]);
    assert_eq!(
        workflow_json["tasks"]
            .as_array()
            .expect("workflow tasks should be an array")
            .len(),
        2,
        "workflow should ignore staged task buckets"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_delete_remove_from_chart_restores_partially_removed_metadata_by_cli() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before failed chart removal: {init_output:?}"
    );

    for task_id in ["prepare-samples", "collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    fs::write(
        project_dir.join("collect-samples").join("notes.txt"),
        "keep user notes",
    )
    .expect("user task file should be written");
    let deleted_depends_on_prepare = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "collect-samples", "prepare-samples"],
    );
    assert!(
        deleted_depends_on_prepare.status.success(),
        "deleted task should depend on prepare-samples before chart removal failure: {deleted_depends_on_prepare:?}"
    );
    let dependent_depends_on_deleted = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        dependent_depends_on_deleted.status.success(),
        "dependent task should depend on deleted task before chart removal failure: {dependent_depends_on_deleted:?}"
    );

    let deleted_metadata_path = project_dir.join("collect-samples/.spielgantt/task.json");
    let deleted_metadata_before = fs::read_to_string(&deleted_metadata_path)
        .expect("deleted task metadata should be readable");
    let dependent_metadata_path = project_dir.join("analyze-results/.spielgantt/task.json");
    let dependent_metadata_before = fs::read_to_string(&dependent_metadata_path)
        .expect("dependent metadata should be readable");

    let deleted_task_dir = project_dir.join("collect-samples");
    let original_task_mode = fs::metadata(&deleted_task_dir)
        .expect("deleted task directory metadata should be readable")
        .permissions()
        .mode();
    fs::set_permissions(
        &deleted_task_dir,
        fs::Permissions::from_mode(original_task_mode & !0o222),
    )
    .expect("deleted task directory should be made read-only");

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "collect-samples", "--remove-from-chart"],
    );

    fs::set_permissions(
        &deleted_task_dir,
        fs::Permissions::from_mode(original_task_mode),
    )
    .expect("deleted task directory permissions should be restored");

    assert!(
        !delete_output.status.success(),
        "task chart removal should fail when staged metadata cannot leave the task folder: {delete_output:?}"
    );
    assert_eq!(
        fs::read_to_string(&deleted_metadata_path)
            .expect("deleted task metadata should be restored after failed chart removal"),
        deleted_metadata_before,
        "failed chart removal should restore selected task metadata"
    );
    assert_eq!(
        fs::read_to_string(&dependent_metadata_path)
            .expect("dependent metadata should remain readable after failed chart removal"),
        dependent_metadata_before,
        "failed chart removal should restore dependent metadata"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join("collect-samples/notes.txt"))
            .expect("user task file should remain readable"),
        "keep user notes"
    );

    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(&dependent_metadata_path)
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert_eq!(
        dependent_metadata.dependencies,
        vec!["collect-samples".to_string()],
        "dependent task should still point at the restored task"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_delete_remove_from_chart_cleanup_leaves_active_package_valid_by_cli() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before failed chart removal: {init_output:?}"
    );

    for task_id in ["prepare-samples", "collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let deleted_depends_on_prepare = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "collect-samples", "prepare-samples"],
    );
    assert!(
        deleted_depends_on_prepare.status.success(),
        "deleted task should depend on prepare-samples before chart removal failure: {deleted_depends_on_prepare:?}"
    );
    let dependent_depends_on_deleted = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        dependent_depends_on_deleted.status.success(),
        "dependent task should depend on deleted task before chart removal failure: {dependent_depends_on_deleted:?}"
    );

    let deleted_metadata_dir = project_dir.join("collect-samples/.spielgantt");
    let original_metadata_mode = fs::metadata(&deleted_metadata_dir)
        .expect("deleted task metadata directory metadata should be readable")
        .permissions()
        .mode();
    fs::set_permissions(
        &deleted_metadata_dir,
        fs::Permissions::from_mode(original_metadata_mode & !0o222),
    )
    .expect("deleted task metadata directory should be made read-only");

    let delete_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "delete",
            "collect-samples",
            "--remove-from-chart",
            "--json",
        ],
    );

    let staged_metadata_dir = fs::read_dir(project_dir.join("collect-samples"))
        .expect("deleted task directory should remain readable")
        .map(|entry| {
            entry
                .expect("deleted task directory entry should be readable")
                .path()
        })
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".spielgantt-delete-staged-"))
        })
        .expect("failed staged cleanup should leave staged metadata for manual cleanup");
    fs::set_permissions(
        &staged_metadata_dir,
        fs::Permissions::from_mode(original_metadata_mode),
    )
    .expect("staged task metadata directory permissions should be restored");

    assert!(
        delete_output.status.success(),
        "task chart removal should report that the package mutation committed despite staged cleanup failure: {delete_output:?}"
    );
    let delete_stdout = stdout(&delete_output);
    assert!(
        !delete_stdout.trim().is_empty(),
        "task chart removal should emit JSON stdout; stderr was: {}",
        stderr(&delete_output)
    );
    let delete_json: serde_json::Value =
        serde_json::from_str(&delete_stdout).unwrap_or_else(|error| {
            panic!("task chart removal stdout should be JSON, got {delete_stdout:?}: {error}")
        });
    assert_eq!(delete_json["schema_version"], 1);
    assert_eq!(delete_json["task_id"], "collect-samples");
    assert_eq!(delete_json["mode"], "remove-from-chart");
    assert_eq!(delete_json["committed"], true);
    assert_eq!(delete_json["cleanup"]["status"], "failed");
    assert!(
        delete_json["cleanup"]["path"]
            .as_str()
            .expect("cleanup failure should expose staged metadata path")
            .contains(".spielgantt-delete-staged-"),
        "task chart removal JSON should report staged metadata path: {delete_json}"
    );
    assert!(
        delete_json["cleanup"]["error"]
            .as_str()
            .expect("cleanup failure should expose an actionable error")
            .contains("failed to remove task metadata"),
        "task chart removal JSON should explain staged cleanup failure: {delete_json}"
    );
    assert!(
        stderr(&delete_output).contains("failed to remove task metadata"),
        "task chart removal should explain staged cleanup failure: {}",
        stderr(&delete_output)
    );
    assert!(
        stderr(&delete_output).contains(".spielgantt-delete-staged-"),
        "task chart removal should report the staged metadata path: {}",
        stderr(&delete_output)
    );
    assert!(
        !deleted_metadata_dir.exists(),
        "committed chart removal should leave no active selected task metadata"
    );

    let dependent_metadata_path = project_dir.join("analyze-results/.spielgantt/task.json");
    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(&dependent_metadata_path)
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert_eq!(
        dependent_metadata.dependencies,
        vec!["prepare-samples".to_string()],
        "dependent task should adopt the deleted task's blockers after the package mutation commits"
    );

    let validate_output = run_spielgantt_in(&project_dir, &["validate", "--json"]);
    assert!(
        validate_output.status.success(),
        "active package should remain valid after staged cleanup failure: {validate_output:?}"
    );
    assert_eq!(
        stdout_json(&validate_output)["valid"],
        true,
        "validate --json should report a valid active package after staged cleanup failure"
    );

    let snapshot_json =
        support::run_spielgantt_json_in(&project_dir, &["agent", "snapshot", "--json"]);
    assert_eq!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .len(),
        2,
        "agent snapshot should report only active tasks after staged cleanup failure"
    );
    assert!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .iter()
            .all(|task| !task["project_relative_path"]
                .as_str()
                .expect("snapshot task should expose a project-relative path")
                .contains(".spielgantt-delete-staged-")),
        "agent snapshot should not expose staged cleanup artifacts"
    );

    let task_list_json = support::run_spielgantt_json_in(&project_dir, &["task", "list", "--json"]);
    assert_eq!(
        task_list_json["tasks"]
            .as_array()
            .expect("task list should expose tasks")
            .len(),
        2,
        "task list should report only active tasks after staged cleanup failure"
    );

    let dependent_json = support::run_spielgantt_json_in(
        &project_dir,
        &["task", "show", "--json", "analyze-results"],
    );
    assert_eq!(
        dependent_json["dependencies"],
        serde_json::json!(["prepare-samples"]),
        "task show should report rewired active dependencies after staged cleanup failure"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_delete_directory_stages_before_dependent_rewrites_by_cli() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before failed directory deletion: {init_output:?}"
    );

    for task_id in ["prepare-samples", "collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let deleted_depends_on_prepare = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "collect-samples", "prepare-samples"],
    );
    assert!(
        deleted_depends_on_prepare.status.success(),
        "deleted task should depend on prepare-samples before delete failure: {deleted_depends_on_prepare:?}"
    );
    let dependent_depends_on_deleted = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        dependent_depends_on_deleted.status.success(),
        "dependent task should depend on deleted task before delete failure: {dependent_depends_on_deleted:?}"
    );

    let selected_metadata_path = project_dir.join("collect-samples/.spielgantt/task.json");
    let selected_metadata_before = fs::read_to_string(&selected_metadata_path)
        .expect("selected metadata should be readable before delete failure");
    let dependent_metadata_path = project_dir.join("analyze-results/.spielgantt/task.json");
    let dependent_metadata_before = fs::read_to_string(&dependent_metadata_path)
        .expect("dependent metadata should be readable before delete failure");

    let original_project_mode = fs::metadata(&project_dir)
        .expect("project directory metadata should be readable")
        .permissions()
        .mode();
    fs::set_permissions(
        &project_dir,
        fs::Permissions::from_mode(original_project_mode & !0o222),
    )
    .expect("project directory should be made read-only");

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "collect-samples", "--delete-directory"],
    );

    fs::set_permissions(
        &project_dir,
        fs::Permissions::from_mode(original_project_mode),
    )
    .expect("project directory permissions should be restored");

    assert!(
        !delete_output.status.success(),
        "task directory delete should fail when the task bucket cannot be staged: {delete_output:?}"
    );
    assert!(
        stderr(&delete_output).contains("failed to stage task directory"),
        "task directory delete should fail before dependent rewrites when staging fails: {}",
        stderr(&delete_output)
    );
    assert_eq!(
        fs::read_to_string(&selected_metadata_path)
            .expect("selected metadata should remain readable after failed staging"),
        selected_metadata_before,
        "failed task directory deletion should leave selected task metadata active"
    );
    assert_eq!(
        fs::read_to_string(&dependent_metadata_path)
            .expect("dependent metadata should remain readable after failed staging"),
        dependent_metadata_before,
        "failed task directory deletion should not rewrite dependents when staging fails"
    );

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "project should remain valid after failed task directory staging: {validate_output:?}"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_delete_directory_dependent_rewrite_restores_canonical_bucket_by_cli() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before failed directory deletion: {init_output:?}"
    );

    for task_id in ["prepare-samples", "collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let deleted_depends_on_prepare = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "collect-samples", "prepare-samples"],
    );
    assert!(
        deleted_depends_on_prepare.status.success(),
        "deleted task should depend on prepare-samples before delete failure: {deleted_depends_on_prepare:?}"
    );
    let dependent_depends_on_deleted = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        dependent_depends_on_deleted.status.success(),
        "dependent task should depend on deleted task before delete failure: {dependent_depends_on_deleted:?}"
    );

    let selected_task_dir = project_dir.join("collect-samples");
    let selected_metadata_path = selected_task_dir.join(".spielgantt/task.json");
    let selected_metadata_before = fs::read_to_string(&selected_metadata_path)
        .expect("selected metadata should be readable before delete failure");
    let dependent_metadata_dir = project_dir.join("analyze-results/.spielgantt");
    let dependent_metadata_path = dependent_metadata_dir.join("task.json");
    let dependent_metadata_before = fs::read_to_string(&dependent_metadata_path)
        .expect("dependent metadata should be readable before delete failure");

    let original_dependent_metadata_mode = fs::metadata(&dependent_metadata_dir)
        .expect("dependent metadata directory metadata should be readable")
        .permissions()
        .mode();
    fs::set_permissions(
        &dependent_metadata_dir,
        fs::Permissions::from_mode(original_dependent_metadata_mode & !0o222),
    )
    .expect("dependent metadata directory should be made read-only");

    let delete_output = run_spielgantt_in(
        &project_dir,
        &["task", "delete", "collect-samples", "--delete-directory"],
    );

    fs::set_permissions(
        &dependent_metadata_dir,
        fs::Permissions::from_mode(original_dependent_metadata_mode),
    )
    .expect("dependent metadata directory permissions should be restored");

    assert!(
        !delete_output.status.success(),
        "task directory delete should fail when dependent metadata cannot be rewritten: {delete_output:?}"
    );
    assert!(
        selected_task_dir.exists(),
        "failed dependent rewrite should restore the selected task bucket to its canonical path"
    );
    assert_eq!(
        fs::read_to_string(&selected_metadata_path)
            .expect("selected metadata should remain readable after rewrite failure"),
        selected_metadata_before,
        "failed dependent rewrite should restore selected task metadata"
    );
    assert_eq!(
        fs::read_to_string(&dependent_metadata_path)
            .expect("dependent metadata should remain readable after rewrite failure"),
        dependent_metadata_before,
        "failed dependent rewrite should leave dependent metadata unchanged"
    );
    assert!(
        fs::read_dir(&project_dir)
            .expect("project directory should remain readable")
            .all(|entry| !entry
                .expect("project directory entry should be readable")
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(".spielgantt-delete-staged-task-"))),
        "failed dependent rewrite should not leave the staged task bucket behind"
    );

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "project should remain valid after dependent rewrite failure: {validate_output:?}"
    );
}

#[cfg(unix)]
#[test]
fn failed_task_delete_directory_cleanup_leaves_active_package_valid_by_cli() {
    use std::os::unix::fs::PermissionsExt;

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before failed directory deletion: {init_output:?}"
    );

    for task_id in ["prepare-samples", "collect-samples", "analyze-results"] {
        let create_output = run_spielgantt_in(&project_dir, &["task", "create", task_id]);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }
    let deleted_depends_on_prepare = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "collect-samples", "prepare-samples"],
    );
    assert!(
        deleted_depends_on_prepare.status.success(),
        "deleted task should depend on prepare-samples before delete failure: {deleted_depends_on_prepare:?}"
    );
    let dependent_depends_on_deleted = run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "collect-samples"],
    );
    assert!(
        dependent_depends_on_deleted.status.success(),
        "dependent task should depend on deleted task before delete failure: {dependent_depends_on_deleted:?}"
    );

    let deleted_task_dir = project_dir.join("collect-samples");
    let deleted_metadata_dir = deleted_task_dir.join(".spielgantt");
    let original_metadata_mode = fs::metadata(&deleted_metadata_dir)
        .expect("deleted task metadata directory metadata should be readable")
        .permissions()
        .mode();
    fs::set_permissions(
        &deleted_metadata_dir,
        fs::Permissions::from_mode(original_metadata_mode & !0o222),
    )
    .expect("deleted task metadata directory should be made read-only");

    let delete_output = run_spielgantt_in(
        &project_dir,
        &[
            "task",
            "delete",
            "collect-samples",
            "--delete-directory",
            "--json",
        ],
    );

    let staged_task_dir = fs::read_dir(&project_dir)
        .expect("project directory should remain readable")
        .map(|entry| {
            entry
                .expect("project directory entry should be readable")
                .path()
        })
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(".spielgantt-delete-staged-task-"))
        })
        .expect("failed cleanup should leave staged task bucket for manual cleanup");
    fs::set_permissions(
        staged_task_dir.join(".spielgantt"),
        fs::Permissions::from_mode(original_metadata_mode),
    )
    .expect("staged task metadata directory permissions should be restored");

    assert!(
        delete_output.status.success(),
        "task directory delete should report that the package mutation committed despite staged cleanup failure: {delete_output:?}"
    );
    let delete_stdout = stdout(&delete_output);
    assert!(
        !delete_stdout.trim().is_empty(),
        "task directory delete should emit JSON stdout; stderr was: {}",
        stderr(&delete_output)
    );
    let delete_json: serde_json::Value =
        serde_json::from_str(&delete_stdout).unwrap_or_else(|error| {
            panic!("task directory delete stdout should be JSON, got {delete_stdout:?}: {error}")
        });
    assert_eq!(delete_json["schema_version"], 1);
    assert_eq!(delete_json["task_id"], "collect-samples");
    assert_eq!(delete_json["mode"], "delete-directory");
    assert_eq!(delete_json["committed"], true);
    assert_eq!(delete_json["cleanup"]["status"], "failed");
    assert!(
        delete_json["cleanup"]["path"]
            .as_str()
            .expect("cleanup failure should expose staged task bucket path")
            .contains(".spielgantt-delete-staged-task-"),
        "task directory delete JSON should report staged task bucket path: {delete_json}"
    );
    assert!(
        delete_json["cleanup"]["error"]
            .as_str()
            .expect("cleanup failure should expose an actionable error")
            .contains("failed to delete task directory"),
        "task directory delete JSON should explain staged cleanup failure: {delete_json}"
    );
    assert!(
        stderr(&delete_output).contains("failed to delete task directory"),
        "task directory delete should explain staged cleanup failure: {}",
        stderr(&delete_output)
    );
    assert!(
        stderr(&delete_output).contains(".spielgantt-delete-staged-task-"),
        "task directory delete should report the staged task bucket path: {}",
        stderr(&delete_output)
    );
    assert!(
        !deleted_task_dir.exists(),
        "committed directory deletion should leave no active selected task bucket"
    );

    let dependent_metadata_path = project_dir.join("analyze-results/.spielgantt/task.json");
    let dependent_metadata = spielgantt_lib::metadata::TaskMetadata::from_json(
        &fs::read_to_string(&dependent_metadata_path)
            .expect("dependent task metadata should remain readable"),
    )
    .expect("dependent task metadata should remain valid JSON");
    assert_eq!(
        dependent_metadata.dependencies,
        vec!["prepare-samples".to_string()],
        "dependent task should adopt the deleted task's blockers after the package mutation commits"
    );

    let validate_output = run_spielgantt_in(&project_dir, &["validate", "--json"]);
    assert!(
        validate_output.status.success(),
        "active package should remain valid after staged directory cleanup failure: {validate_output:?}"
    );
    assert_eq!(
        stdout_json(&validate_output)["valid"],
        true,
        "validate --json should report a valid active package after staged directory cleanup failure"
    );

    let snapshot_json =
        support::run_spielgantt_json_in(&project_dir, &["agent", "snapshot", "--json"]);
    assert_eq!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .len(),
        2,
        "agent snapshot should report only active tasks after staged directory cleanup failure"
    );
    assert!(
        snapshot_json["tasks"]
            .as_array()
            .expect("snapshot tasks should be an array")
            .iter()
            .all(|task| !task["project_relative_path"]
                .as_str()
                .expect("snapshot task should expose a project-relative path")
                .contains(".spielgantt-delete-staged-")),
        "agent snapshot should not expose staged cleanup artifacts"
    );

    let task_list_json = support::run_spielgantt_json_in(&project_dir, &["task", "list", "--json"]);
    assert_eq!(
        task_list_json["tasks"]
            .as_array()
            .expect("task list should expose tasks")
            .len(),
        2,
        "task list should report only active tasks after staged directory cleanup failure"
    );

    let dependent_json = support::run_spielgantt_json_in(
        &project_dir,
        &["task", "show", "--json", "analyze-results"],
    );
    assert_eq!(
        dependent_json["dependencies"],
        serde_json::json!(["prepare-samples"]),
        "task show should report rewired active dependencies after staged directory cleanup failure"
    );
}
