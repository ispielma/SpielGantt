use std::{fs, path::PathBuf};

use spielgantt_lib::{
    metadata::TaskStatus,
    project_snapshot::{self, ProjectSnapshotDependencyKind},
};
use tempfile::tempdir;

mod support;

#[test]
fn project_snapshot_ignores_stale_task_links_metadata() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading a snapshot: {init_output:?}"
    );
    for task_id in ["prepare-sample", "analyze-results"] {
        let create_output = support::create_task(&project_dir, task_id);
        assert!(
            create_output.status.success(),
            "task create should succeed for {task_id}: {create_output:?}"
        );
    }

    let depend_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "depend", "analyze-results", "prepare-sample"],
    );
    assert!(
        depend_output.status.success(),
        "task depend should succeed before loading a snapshot: {depend_output:?}"
    );
    let update_output = support::run_spielgantt_in(
        &project_dir,
        &["task", "update", "analyze-results", "--status", "blocked"],
    );
    assert!(
        update_output.status.success(),
        "task update should succeed before loading a snapshot: {update_output:?}"
    );

    let links_metadata_path = project_dir
        .join("analyze-results")
        .join(".spielgantt/links.json");
    fs::write(
        &links_metadata_path,
        "this stale file used to be SpielGantt metadata, but is ignored now",
    )
    .expect("stale links metadata should be written");

    let snapshot = project_snapshot::load(&project_dir).expect("snapshot should load");

    let canonical_project_dir =
        fs::canonicalize(&project_dir).expect("project directory should canonicalize");
    assert_eq!(snapshot.project_root(), canonical_project_dir.as_path());
    assert_eq!(
        snapshot
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        ["analyze-results", "prepare-sample"]
    );

    let analyze_task = snapshot
        .task("analyze-results")
        .expect("snapshot should expose the analyze-results task");
    assert_eq!(
        analyze_task.path(),
        canonical_project_dir.join("analyze-results")
    );
    assert_eq!(
        analyze_task.project_relative_path(),
        PathBuf::from("analyze-results")
    );
    assert_eq!(analyze_task.dependencies(), ["prepare-sample"]);
    assert_eq!(analyze_task.status(), Some(&TaskStatus::Blocked));
    assert_eq!(
        fs::read_to_string(&links_metadata_path)
            .expect("stale links file should remain user-owned content"),
        "this stale file used to be SpielGantt metadata, but is ignored now",
        "snapshot loading should not read, validate, or rewrite stale links metadata"
    );
}

#[test]
fn project_snapshot_exposes_ordered_events_dependency_references_and_ending_tasks() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    let init_output = support::init_project(workspace_dir.path(), "project");
    assert!(
        init_output.status.success(),
        "init should succeed before loading an event-aware snapshot: {init_output:?}"
    );

    support::write_project_metadata(
        &project_dir,
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    );

    support::write_task_metadata(&project_dir.join("prepare-samples"), "prepare-samples");
    fs::write(
        project_dir
            .join("prepare-samples")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare-samples\",\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("task metadata should be written with ends_at");

    support::write_task_metadata(&project_dir.join("analyze-results"), "analyze-results");
    fs::write(
        project_dir
            .join("analyze-results")
            .join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze-results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare-samples\"\n  ]\n}\n",
    )
    .expect("task metadata should be written with mixed dependency targets");

    let snapshot = project_snapshot::load(&project_dir).expect("snapshot should load");

    assert_eq!(
        snapshot.events(),
        &["START".to_string(), "MOT".to_string(), "BEC".to_string()],
        "snapshot should preserve ordered project events"
    );

    let analyze_results = snapshot
        .task("analyze-results")
        .expect("snapshot should expose the analyze-results task");
    assert_eq!(
        analyze_results
            .dependency_references()
            .iter()
            .map(|dependency| (dependency.id(), dependency.kind()))
            .collect::<Vec<_>>(),
        vec![
            ("START", ProjectSnapshotDependencyKind::Event),
            ("prepare-samples", ProjectSnapshotDependencyKind::Task),
        ],
        "snapshot should resolve task and event dependency references distinctly"
    );

    let prepare_samples = snapshot
        .task("prepare-samples")
        .expect("snapshot should expose the prepare-samples task");
    assert_eq!(
        prepare_samples.ends_at(),
        Some("BEC"),
        "snapshot should retain the optional ends_at event reference"
    );
    assert_eq!(
        snapshot
            .tasks_ending_at("BEC")
            .into_iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        vec!["prepare-samples"],
        "snapshot should identify tasks that end at a specific event"
    );
}
