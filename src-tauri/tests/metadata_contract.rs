use spielgantt_lib::metadata::{
    validate_project_namespace_entry, ProjectBoundaryEvents, ProjectMetadata, ProjectNamespaceKind,
    TaskMetadata, TaskStatus,
};

#[test]
fn golden_project_and_task_metadata_validate() {
    let expected_events = vec!["START".to_string(), "MOT".to_string(), "BEC".to_string()];
    let project =
        ProjectMetadata::from_json(include_str!("../../docs/metadata/examples/project.json"))
            .expect("project metadata example should validate");
    assert_eq!(project.schema_version, 1);
    assert_eq!(project.events.as_deref(), Some(expected_events.as_slice()));
    assert_eq!(
        project.boundary_events,
        Some(ProjectBoundaryEvents {
            start: "START".to_string(),
            finish: "BEC".to_string(),
        })
    );

    let task = TaskMetadata::from_json(include_str!("../../docs/metadata/examples/task.json"))
        .expect("task metadata example should validate");
    assert_eq!(task.id, "calibrate-laser");
    assert_eq!(task.ends_at.as_deref(), Some("BEC"));
    assert_eq!(task.status, Some(TaskStatus::Unblocked));
}

#[test]
fn task_metadata_reads_legacy_open_statuses_as_unblocked() {
    for legacy_status in ["planned", "in_progress"] {
        let task = TaskMetadata::from_json(&format!(
            "{{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\",\n  \"status\": \"{legacy_status}\"\n}}\n"
        ))
        .expect("legacy open statuses should remain readable");

        assert_eq!(
            task.status,
            Some(TaskStatus::Unblocked),
            "legacy status {legacy_status} should migrate to unblocked"
        );
        assert!(
            task.to_json()
                .expect("legacy status should serialize")
                .contains("\"status\": \"unblocked\""),
            "legacy status should serialize through the canonical status"
        );
    }
}

#[test]
fn project_metadata_version_1_starts_with_boundary_events() {
    let metadata = ProjectMetadata::version_1();
    let expected_events = vec!["start".to_string(), "finished".to_string()];

    assert_eq!(
        metadata.events.as_deref(),
        Some(expected_events.as_slice()),
        "new project metadata should include default timeline boundary events"
    );
}

#[test]
fn project_metadata_round_trips_ordered_events_and_rejects_duplicates() {
    let expected_events = vec!["START".to_string(), "MOT".to_string(), "BEC".to_string()];
    let project = ProjectMetadata::from_json(
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"BEC\"\n  ]\n}\n",
    )
    .expect("project metadata should accept ordered events");
    assert_eq!(project.events.as_deref(), Some(expected_events.as_slice()));

    let rendered = project
        .to_json()
        .expect("project metadata should serialize");
    let parsed =
        ProjectMetadata::from_json(&rendered).expect("serialized project should round-trip");
    assert_eq!(parsed.events.as_deref(), Some(expected_events.as_slice()));

    let duplicate_error = ProjectMetadata::from_json(
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\",\n    \"START\"\n  ]\n}\n",
    )
    .expect_err("duplicate project events should be rejected");
    assert!(
        duplicate_error.to_string().contains("duplicate event id"),
        "duplicate event rejection should mention the duplicate event id: {duplicate_error}"
    );

    let invalid_error = ProjectMetadata::from_json(
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"phase/name\"\n  ]\n}\n",
    )
    .expect_err("filesystem-unsafe event ids should be rejected");
    assert!(
        invalid_error.to_string().contains("event id"),
        "invalid event rejection should mention the event id rule: {invalid_error}"
    );
}

#[test]
fn task_metadata_accepts_human_names_and_existing_slug_ids() {
    for id in ["RbK New Experiment", "2026_04_15 RbK", "calibrate-laser"] {
        let metadata = TaskMetadata::version_1(id).expect("task metadata should accept the id");
        assert_eq!(metadata.id, id);

        let rendered = metadata.to_json().expect("task metadata should serialize");
        let parsed =
            TaskMetadata::from_json(&rendered).expect("serialized metadata should round-trip");
        assert_eq!(parsed.id, id);
    }
}

#[test]
fn task_metadata_round_trips_optional_ends_at_and_rejects_invalid_names() {
    let task = TaskMetadata::from_json(
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\",\n  \"ends_at\": \"BEC\"\n}\n",
    )
    .expect("task metadata should accept an optional ends_at event id");
    let rendered = task.to_json().expect("task metadata should serialize");
    let rendered_task = TaskMetadata::from_json(&rendered)
        .expect("serialized task metadata should remain valid JSON");
    assert_eq!(
        rendered_task.ends_at.as_deref(),
        Some("BEC"),
        "task metadata should preserve ends_at when present: {rendered}"
    );

    let omitted = TaskMetadata::version_1("calibrate-laser")
        .expect("task metadata should still create without ends_at");
    let omitted_rendered = omitted.to_json().expect("task metadata should serialize");
    let omitted_value: serde_json::Value =
        serde_json::from_str(&omitted_rendered).expect("serialized metadata should be valid JSON");
    assert!(
        omitted_value.get("ends_at").is_none(),
        "task metadata should omit ends_at when unset: {omitted_rendered}"
    );

    let invalid_error =
        TaskMetadata::from_json("{\n  \"schema_version\": 1,\n  \"id\": \"calibrate-laser\",\n  \"ends_at\": \"phase/name\"\n}\n")
            .expect_err("filesystem-unsafe ends_at ids should be rejected");
    assert!(
        invalid_error.to_string().contains("event id"),
        "invalid ends_at rejection should mention the event id rule: {invalid_error}"
    );
}

#[test]
fn task_metadata_rejects_filesystem_unsafe_ids() {
    for id in [
        "",
        " ",
        ".",
        "..",
        "phase/name",
        "phase\\name",
        "phase:name",
        "con",
    ] {
        let error = TaskMetadata::version_1(id).expect_err("unsafe task ids should not validate");

        assert!(
            error.to_string().contains("task id"),
            "error should explain the invalid task id: {error}"
        );
    }
}

#[test]
fn shared_namespace_validator_rejects_task_event_collisions_and_accepts_non_collisions() {
    let task_ids = vec!["calibrate-laser".to_string(), "prepare-sample".to_string()];
    let event_ids = vec!["START".to_string(), "MOT".to_string()];

    let collision_error = validate_project_namespace_entry(
        "MOT",
        ProjectNamespaceKind::Task,
        ProjectNamespaceKind::Event,
        &event_ids,
    )
    .expect_err("task ids should not be allowed to reuse an event id");
    assert!(
        collision_error
            .to_string()
            .contains("task id 'MOT' collides with project event id 'MOT'"),
        "collision error should describe the conflicting namespaces: {collision_error}"
    );

    validate_project_namespace_entry(
        "conclude-run",
        ProjectNamespaceKind::Event,
        ProjectNamespaceKind::Task,
        &task_ids,
    )
    .expect("an unused event id should be accepted against the task namespace");
}
