use std::fs;

use tempfile::tempdir;

use spielgantt_lib::metadata::{ProjectMetadata, TaskMetadata};

mod support;

use support::{run_spielgantt_in, stderr_text as stderr, stdout_text as stdout};

#[test]
fn event_create_preserves_finish_boundary_and_rejects_namespace_collisions_before_writes() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before creating events: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    let task_output = support::create_task(&project_dir, "calibrate-laser");
    assert!(
        task_output.status.success(),
        "task create should succeed before event create collision checks: {task_output:?}"
    );

    let create_output = run_spielgantt_in(&project_dir, &["event", "create", "BEC"]);
    assert!(
        create_output.status.success(),
        "event create should succeed for an unused project-wide id: {create_output:?}"
    );
    assert!(
        stdout(&create_output).contains("Created event 'BEC'"),
        "event create should report the created event id: {}",
        stdout(&create_output)
    );

    let created_project = ProjectMetadata::from_json(
        &fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable after event create"),
    )
    .expect("project metadata should remain valid after event create");
    let expected_events = vec!["START".to_string(), "BEC".to_string(), "MOT".to_string()];
    assert_eq!(
        created_project.events.as_deref(),
        Some(expected_events.as_slice()),
        "event create should preserve the inferred finish boundary as the final event"
    );
    assert_eq!(
        created_project
            .boundary_events
            .as_ref()
            .map(|boundary_events| (
                boundary_events.start.as_str(),
                boundary_events.finish.as_str()
            )),
        Some(("START", "MOT")),
        "event create should persist inferred boundary semantics for legacy string-list projects"
    );

    let project_metadata_after_success =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable after event create");

    let task_collision_output =
        run_spielgantt_in(&project_dir, &["event", "create", "calibrate-laser"]);
    assert!(
        !task_collision_output.status.success(),
        "event create should reject ids that collide with tasks: {task_collision_output:?}"
    );
    assert!(
        stderr(&task_collision_output)
            .contains("event id 'calibrate-laser' collides with project task id 'calibrate-laser'"),
        "event create should explain task collisions: {}",
        stderr(&task_collision_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after task collision"),
        project_metadata_after_success,
        "task-collision rejection should not rewrite project metadata"
    );

    let event_collision_output = run_spielgantt_in(&project_dir, &["event", "create", "START"]);
    assert!(
        !event_collision_output.status.success(),
        "event create should reject ids that collide with existing events: {event_collision_output:?}"
    );
    assert!(
        stderr(&event_collision_output)
            .contains("event id 'START' collides with project event id 'START'"),
        "event create should explain event collisions: {}",
        stderr(&event_collision_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after event collision"),
        project_metadata_after_success,
        "event-collision rejection should not rewrite project metadata"
    );
}

#[test]
fn event_rename_updates_project_metadata_and_task_references() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before renaming events: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ],\n  \"boundary_events\": {\n    \"start\": \"START\",\n    \"finish\": \"BEC\"\n  }\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("task metadata should be written with ends_at");
    let user_notes_path = project_dir.join("prepare-sample/notes.txt");
    fs::write(&user_notes_path, "ordinary user notes")
        .expect("ordinary task user file should be written");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-sample\"\n  ]\n}\n",
    )
    .expect("task metadata should be written with dependencies");

    let rename_output = run_spielgantt_in(&project_dir, &["event", "rename", "START", "BEGIN"]);
    assert!(
        rename_output.status.success(),
        "event rename should succeed: {rename_output:?}"
    );
    assert!(
        stdout(&rename_output).contains("Renamed event 'START' to 'BEGIN'"),
        "event rename should report the event rename: {}",
        stdout(&rename_output)
    );

    let project_metadata = ProjectMetadata::from_json(
        &fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable after event rename"),
    )
    .expect("project metadata should remain valid after event rename");
    let renamed_events = vec!["BEGIN".to_string(), "MOT".to_string(), "BEC".to_string()];
    assert_eq!(
        project_metadata.events.as_deref(),
        Some(renamed_events.as_slice()),
        "event rename should update the ordered project event list"
    );
    assert_eq!(
        project_metadata
            .boundary_events
            .as_ref()
            .map(|boundary_events| (
                boundary_events.start.as_str(),
                boundary_events.finish.as_str()
            )),
        Some(("BEGIN", "BEC")),
        "event rename should update project boundary event references"
    );

    let prepared_task = TaskMetadata::from_json(
        &fs::read_to_string(
            project_dir
                .join("prepare-sample")
                .join(".spielgantt/task.json"),
        )
        .expect("prepare-sample metadata should be readable after event rename"),
    )
    .expect("prepare-sample metadata should remain valid after event rename");
    assert_eq!(
        prepared_task.ends_at.as_deref(),
        Some("BEGIN"),
        "event rename should update task ends_at references"
    );
    assert_eq!(
        fs::read_to_string(&user_notes_path)
            .expect("ordinary user file should remain readable after event rename"),
        "ordinary user notes",
        "event rename should only rewrite SpielGantt metadata under .spielgantt"
    );

    let analyzed_task = TaskMetadata::from_json(
        &fs::read_to_string(
            project_dir
                .join("analyze-results")
                .join(".spielgantt/task.json"),
        )
        .expect("analyze-results metadata should be readable after event rename"),
    )
    .expect("analyze-results metadata should remain valid after event rename");
    assert_eq!(
        analyzed_task.dependencies,
        vec!["BEGIN".to_string(), "prepare-sample".to_string()],
        "event rename should update task dependency references"
    );
}

#[test]
fn event_rename_rolls_back_project_and_task_metadata_when_a_later_write_fails() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before rename rollback checks: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\"\n  ]\n}\n",
    )
    .expect("downstream task metadata should be written with the source event");

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("ending task metadata should be written with the source event");

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rename");
    let original_analyze_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("downstream task metadata should be readable before rename");
    let original_prepare_metadata = fs::read_to_string(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
    )
    .expect("ending task metadata should be readable before rename");

    fs::create_dir(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json.tmp"),
    )
    .expect("temporary metadata collision directory should be created");

    let rename_output = run_spielgantt_in(&project_dir, &["event", "rename", "START", "BEGIN"]);
    assert!(
        !rename_output.status.success(),
        "event rename should fail when a later task metadata write fails: {rename_output:?}"
    );
    assert!(
        stderr(&rename_output).contains("failed to write temporary metadata"),
        "event rename failure should surface the failed metadata write: {}",
        stderr(&rename_output)
    );

    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after failed rename"),
        original_project_metadata,
        "failed event rename should restore project metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("analyze-results")
                .join(".spielgantt/task.json"),
        )
        .expect("downstream task metadata should remain readable after failed rename"),
        original_analyze_metadata,
        "failed event rename should restore already-written task metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("prepare-sample")
                .join(".spielgantt/task.json"),
        )
        .expect("ending task metadata should remain readable after failed rename"),
        original_prepare_metadata,
        "failed event rename should preserve later task metadata"
    );
}

#[test]
fn event_rename_rejects_invalid_or_colliding_new_names_before_writes() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before rename rejection checks: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("calibrate-laser"), "calibrate-laser");
    let original_task_metadata = fs::read_to_string(
        project_dir
            .join("calibrate-laser")
            .join(".spielgantt/task.json"),
    )
    .expect("task metadata should be readable before rename");
    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rename");

    let invalid_output =
        run_spielgantt_in(&project_dir, &["event", "rename", "START", "Align/Optics"]);
    assert!(
        !invalid_output.status.success(),
        "event rename should reject invalid new names: {invalid_output:?}"
    );
    assert!(
        stderr(&invalid_output).contains("filesystem separator"),
        "invalid-name rejection should explain the name rule: {}",
        stderr(&invalid_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after invalid rename"),
        original_project_metadata,
        "invalid-name rejection should not rewrite project metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("calibrate-laser")
                .join(".spielgantt/task.json")
        )
        .expect("task metadata should remain readable after invalid rename"),
        original_task_metadata,
        "invalid-name rejection should not rewrite task metadata"
    );

    let collision_output = run_spielgantt_in(
        &project_dir,
        &["event", "rename", "START", "calibrate-laser"],
    );
    assert!(
        !collision_output.status.success(),
        "event rename should reject collisions with tasks: {collision_output:?}"
    );
    assert!(
        stderr(&collision_output)
            .contains("event id 'calibrate-laser' collides with project task id 'calibrate-laser'"),
        "collision rejection should explain the shared namespace rule: {}",
        stderr(&collision_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after collision rename"),
        original_project_metadata,
        "collision rejection should not rewrite project metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("calibrate-laser")
                .join(".spielgantt/task.json")
        )
        .expect("task metadata should remain readable after collision rename"),
        original_task_metadata,
        "collision rejection should not rewrite task metadata"
    );

    let event_collision_output =
        run_spielgantt_in(&project_dir, &["event", "rename", "START", "MOT"]);
    assert!(
        !event_collision_output.status.success(),
        "event rename should reject collisions with other events: {event_collision_output:?}"
    );
    assert!(
        stderr(&event_collision_output)
            .contains("event id 'MOT' collides with project event id 'MOT'"),
        "event collision rejection should explain the shared namespace rule: {}",
        stderr(&event_collision_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after event collision rename"),
        original_project_metadata,
        "event-collision rejection should not rewrite project metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("calibrate-laser")
                .join(".spielgantt/task.json")
        )
        .expect("task metadata should remain readable after event collision rename"),
        original_task_metadata,
        "event-collision rejection should not rewrite task metadata"
    );
}

#[test]
fn event_rename_reports_missing_source_before_destination_collisions() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before missing-source rename checks: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\"\n  ]\n}\n",
    );
    support::write_task_metadata(&project_dir.join("task-a"), "task-a");

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rename");
    let original_task_metadata =
        fs::read_to_string(project_dir.join("task-a").join(".spielgantt/task.json"))
            .expect("task metadata should be readable before rename");

    let rename_output = run_spielgantt_in(&project_dir, &["event", "rename", "MISSING", "task-a"]);
    assert!(
        !rename_output.status.success(),
        "event rename should reject a missing source event before destination collisions: {rename_output:?}"
    );
    let rename_stderr = stderr(&rename_output);
    assert!(
        rename_stderr.contains("event id 'MISSING' was not found"),
        "event rename should report the missing source event first: {rename_stderr}"
    );
    assert!(
        !rename_stderr.contains("collides with project task id 'task-a'"),
        "missing-source rejection should not be masked by destination collisions: {rename_stderr}"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after rejected rename"),
        original_project_metadata,
        "missing-source rejection should not rewrite project metadata"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join("task-a").join(".spielgantt/task.json"))
            .expect("task metadata should remain readable after rejected rename"),
        original_task_metadata,
        "missing-source rejection should not rewrite task metadata"
    );
}

#[test]
fn event_delete_removes_unreferenced_events_and_preserves_order() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before deleting events: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    );

    let delete_output = run_spielgantt_in(&project_dir, &["event", "delete", "MOT"]);
    assert!(
        delete_output.status.success(),
        "event delete should succeed for an unreferenced event: {delete_output:?}"
    );
    assert!(
        stdout(&delete_output).contains("Deleted event 'MOT'"),
        "event delete should report the removed event id: {}",
        stdout(&delete_output)
    );

    let updated_project = ProjectMetadata::from_json(
        &fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable after event delete"),
    )
    .expect("project metadata should remain valid after event delete");
    let expected_events = vec!["START".to_string(), "BEC".to_string()];
    assert_eq!(
        updated_project.events.as_deref(),
        Some(expected_events.as_slice()),
        "event delete should preserve the remaining event order"
    );
}

#[test]
fn event_delete_rejects_referenced_events_and_lists_blocking_tasks() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before checking delete safety: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written with an event blocker");

    support::write_task_metadata(&project_dir.join("prepare-sample"), "prepare-sample");
    fs::write(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-sample\",\n  \"ends_at\": \"MOT\"\n}\n",
    )
    .expect("ending task metadata should be written with ends_at");

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rejected delete");
    let original_dependent_metadata = fs::read_to_string(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
    )
    .expect("dependent task metadata should be readable before rejected delete");
    let original_ending_metadata = fs::read_to_string(
        project_dir
            .join("prepare-sample")
            .join(".spielgantt/task.json"),
    )
    .expect("ending task metadata should be readable before rejected delete");

    let delete_output = run_spielgantt_in(&project_dir, &["event", "delete", "MOT"]);
    assert!(
        !delete_output.status.success(),
        "event delete should reject referenced events: {delete_output:?}"
    );
    let delete_stderr = stderr(&delete_output);
    assert!(
        delete_stderr.contains("cannot delete event 'MOT': referenced by tasks"),
        "delete rejection should explain the safety check: {delete_stderr}"
    );
    assert!(
        delete_stderr.contains("'analyze-results'") && delete_stderr.contains("'prepare-sample'"),
        "delete rejection should list every blocking task: {delete_stderr}"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after rejected delete"),
        original_project_metadata,
        "rejected delete should not rewrite project metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("analyze-results")
                .join(".spielgantt/task.json"),
        )
        .expect("dependent task metadata should remain readable after rejected delete"),
        original_dependent_metadata,
        "rejected delete should not rewrite dependent task metadata"
    );
    assert_eq!(
        fs::read_to_string(
            project_dir
                .join("prepare-sample")
                .join(".spielgantt/task.json"),
        )
        .expect("ending task metadata should remain readable after rejected delete"),
        original_ending_metadata,
        "rejected delete should not rewrite ending task metadata"
    );
}

#[test]
fn event_delete_rejects_boundary_events_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before checking boundary delete safety: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ],\n  \"boundary_events\": {\n    \"start\": \"START\",\n    \"finish\": \"BEC\"\n  }\n}\n",
    );

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rejected boundary delete");

    let delete_output = run_spielgantt_in(&project_dir, &["event", "delete", "START"]);
    assert!(
        !delete_output.status.success(),
        "event delete should reject project boundary events: {delete_output:?}"
    );
    assert!(
        stderr(&delete_output)
            .contains("cannot delete event 'START': referenced by project boundary events"),
        "boundary delete rejection should explain the package-level reference: {}",
        stderr(&delete_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after rejected boundary delete"),
        original_project_metadata,
        "rejected boundary delete should not rewrite project metadata"
    );
}

#[test]
fn event_delete_rejects_inferred_boundary_events_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before checking inferred boundary delete safety: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"LEGACY-START\",\n    \"MOT\",\n    \"LEGACY-FINISH\"\n  ]\n}\n",
    );

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before rejected inferred boundary delete");

    let delete_output = run_spielgantt_in(&project_dir, &["event", "delete", "LEGACY-FINISH"]);
    assert!(
        !delete_output.status.success(),
        "event delete should reject inferred project boundary events: {delete_output:?}"
    );
    assert!(
        stderr(&delete_output)
            .contains("cannot delete event 'LEGACY-FINISH': referenced by project boundary events"),
        "inferred boundary delete rejection should explain the package-level reference: {}",
        stderr(&delete_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json")).expect(
            "project metadata should remain readable after rejected inferred boundary delete"
        ),
        original_project_metadata,
        "rejected inferred boundary delete should not rewrite project metadata"
    );
}

#[test]
fn event_delete_reports_missing_events_before_writing_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before missing delete rejection: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    );

    let original_project_metadata =
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should be readable before missing delete");

    let delete_output = run_spielgantt_in(&project_dir, &["event", "delete", "MISSING"]);
    assert!(
        !delete_output.status.success(),
        "event delete should reject missing events: {delete_output:?}"
    );
    assert!(
        stderr(&delete_output).contains("event id 'MISSING' was not found"),
        "missing-event rejection should use the shared not-found message: {}",
        stderr(&delete_output)
    );
    assert_eq!(
        fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable after missing delete"),
        original_project_metadata,
        "missing-event rejection should not rewrite project metadata"
    );
}
