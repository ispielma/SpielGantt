use std::{fs, path::Path};

use tempfile::tempdir;

use spielgantt_lib::{
    app_facade,
    metadata::{ProjectMetadata, TaskMetadata},
    mutation_plan, project, task,
};

fn filesystem_is_case_insensitive(root: &Path) -> bool {
    let probe_dir = root.join("CaseProbe");
    fs::create_dir(&probe_dir).expect("probe directory should be created");
    let is_case_insensitive = root.join("caseprobe").exists();
    fs::remove_dir(&probe_dir).expect("probe directory should be removed");
    is_case_insensitive
}

fn direct_child_names(root: &Path) -> Vec<String> {
    let mut names = fs::read_dir(root)
        .expect("directory entries should be readable")
        .map(|entry| {
            entry
                .expect("directory entry should load")
                .file_name()
                .into_string()
                .expect("directory entry name should be valid utf-8")
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

#[test]
fn normalization_plan_reports_folder_collision_before_apply_without_mutating_files() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");
    let colliding_dir = project_dir.join("calibrate-laser");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");
    fs::create_dir(&colliding_dir).expect("colliding folder should be created");
    fs::write(colliding_dir.join("notes.txt"), "do not touch")
        .expect("colliding user file should be written");

    let plan = mutation_plan::plan_task_folder_normalization(&project_dir)
        .expect("normalization plan should load");

    assert!(
        !plan.is_safe(),
        "normalization plan should expose the preflight collision before apply"
    );
    let canonical_colliding_dir =
        fs::canonicalize(&colliding_dir).expect("colliding directory should canonicalize");
    assert!(
        plan.preflight_issues().iter().any(|issue| issue
            .message()
            .contains(&canonical_colliding_dir.display().to_string())),
        "preflight issues should explain the collision: {:?}",
        plan.preflight_issues()
    );

    assert!(
        task_dir.is_dir(),
        "planning should leave the original task folder in place"
    );
    assert_eq!(
        fs::read_to_string(colliding_dir.join("notes.txt"))
            .expect("colliding user file should remain readable"),
        "do not touch",
        "planning should not mutate colliding user files"
    );
}

#[test]
fn alignment_plan_reports_folder_renames_and_collisions_without_mutating_files() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");
    let colliding_dir = project_dir.join("calibrate-laser");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");
    fs::create_dir(&colliding_dir).expect("colliding folder should be created");
    fs::write(colliding_dir.join("notes.txt"), "do not touch")
        .expect("colliding user file should be written");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");

    assert!(
        !plan.is_safe(),
        "alignment plan should expose the preflight collision before any write path is used"
    );
    let canonical_colliding_dir =
        fs::canonicalize(&colliding_dir).expect("colliding directory should canonicalize");
    assert!(
        plan.preflight_issues().iter().any(|issue| issue
            .message()
            .contains(&canonical_colliding_dir.display().to_string())),
        "preflight issues should explain the collision: {:?}",
        plan.preflight_issues()
    );

    assert!(
        task_dir.is_dir(),
        "planning should leave the original task folder in place"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("notes.txt"))
            .expect("task user file should remain readable"),
        "keep this file"
    );
    assert_eq!(
        fs::read_to_string(colliding_dir.join("notes.txt"))
            .expect("colliding user file should remain readable"),
        "do not touch",
        "planning should not mutate colliding user files"
    );
}

#[test]
fn alignment_plan_applies_folder_renames_and_refreshes_project_paths() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");

    let applied = app_facade::apply_task_folder_alignment(&project_dir, &plan)
        .expect("alignment plan should apply safely");

    assert!(
        applied.applied(),
        "alignment apply result should report that the plan was applied"
    );
    assert!(
        project_dir.join("calibrate-laser").is_dir(),
        "alignment apply should rename the task folder to match the task id"
    );
    assert!(
        !task_dir.exists(),
        "alignment apply should remove the old task folder path"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join("calibrate-laser/notes.txt"))
            .expect("user file should remain readable after the rename"),
        "keep this file",
        "alignment apply should preserve user files inside the renamed folder"
    );
    assert!(
        applied.project().tasks().iter().any(|task| {
            task.id() == "calibrate-laser" && task.project_relative_path() == "calibrate-laser"
        }),
        "alignment apply should refresh the project scan with aligned paths: {applied:?}"
    );
}

#[test]
fn alignment_plan_allows_case_only_targets_for_the_same_task_folder() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    if !filesystem_is_case_insensitive(workspace_dir.path()) {
        return;
    }

    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "Analysis Notes").expect("task adopt should succeed");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");

    assert!(
        plan.preflight_issues().is_empty(),
        "case-only target paths should not be reported as folder collisions for the same task"
    );

    let applied = app_facade::apply_task_folder_alignment(&project_dir, &plan)
        .expect("alignment apply should allow case-only renames for the same task folder");

    assert!(
        applied.applied(),
        "alignment apply should still report success"
    );
    assert_eq!(
        direct_child_names(&project_dir),
        vec![".spielgantt".to_string(), "Analysis Notes".to_string()],
        "case-only alignment should update the direct child folder basename"
    );
    assert_eq!(
        fs::read_to_string(project_dir.join("Analysis Notes/notes.txt"))
            .expect("renamed task user file should remain readable"),
        "keep this file"
    );
}

#[test]
fn alignment_plan_apply_rejects_new_collisions_before_any_folder_is_renamed() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");
    let colliding_dir = project_dir.join("calibrate-laser");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");

    fs::create_dir(&colliding_dir).expect("colliding folder should be created after preview");
    fs::write(colliding_dir.join("notes.txt"), "do not touch")
        .expect("colliding user file should be written");

    let error = app_facade::apply_task_folder_alignment(&project_dir, &plan)
        .expect_err("alignment apply should reject a new collision before writing");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("target folder")
            && rendered_error.contains(&colliding_dir.display().to_string()),
        "alignment apply should explain the target collision: {rendered_error}"
    );

    assert!(
        task_dir.is_dir(),
        "collision rejection should leave the original task folder in place"
    );
    assert_eq!(
        fs::read_to_string(task_dir.join("notes.txt"))
            .expect("task user file should remain readable"),
        "keep this file",
        "collision rejection should preserve user files in the original folder"
    );
    assert_eq!(
        fs::read_to_string(colliding_dir.join("notes.txt"))
            .expect("colliding user file should remain readable"),
        "do not touch",
        "collision rejection should not mutate the colliding folder"
    );
}

#[test]
fn alignment_apply_rejects_extra_submitted_operations_before_any_folder_is_renamed() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");
    let extra_from_dir = project_dir.join("unmanaged notes");
    let extra_to_dir = project_dir.join("Unmanaged Task");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");
    fs::create_dir(&extra_from_dir).expect("extra folder should be created");
    fs::write(extra_from_dir.join("notes.txt"), "not a task bucket")
        .expect("extra user file should be written");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");
    let mut tampered_plan_json =
        serde_json::to_value(&plan).expect("alignment plan should serialize");
    tampered_plan_json["operations"]
        .as_array_mut()
        .expect("serialized alignment plan should contain operations")
        .push(serde_json::json!({
            "renameTaskFolder": {
                "task_id": "Unmanaged Task",
                "from": extra_from_dir,
                "to": extra_to_dir,
            }
        }));
    let tampered_plan = serde_json::from_value(tampered_plan_json)
        .expect("tampered alignment payload should deserialize like a Tauri request");

    let error = app_facade::apply_task_folder_alignment(&project_dir, &tampered_plan)
        .expect_err("alignment apply should reject operations outside the canonical plan");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("canonical folder alignment plan"),
        "alignment apply should explain that the submitted plan is not canonical: {rendered_error}"
    );

    assert!(
        task_dir.is_dir(),
        "tampered plan rejection should leave the canonical task folder unmoved"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "tampered plan rejection should happen before any canonical rename"
    );
    assert!(
        extra_from_dir.is_dir(),
        "tampered plan rejection should not rename extra folders"
    );
    assert!(
        !extra_to_dir.exists(),
        "tampered plan rejection should not create the extra target folder"
    );
}

#[test]
fn alignment_apply_rejects_stale_submitted_operations_before_any_folder_is_renamed() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");

    let stale_plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");
    fs::write(
        task_dir.join(".spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"align-optics\"\n}\n",
    )
    .expect("external metadata edit should be written after preview");

    let error = app_facade::apply_task_folder_alignment(&project_dir, &stale_plan)
        .expect_err("alignment apply should reject a stale submitted plan");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("canonical folder alignment plan"),
        "alignment apply should explain that the submitted plan is stale: {rendered_error}"
    );

    assert!(
        task_dir.is_dir(),
        "stale plan rejection should leave the task folder unmoved"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "stale plan rejection should not apply the old target rename"
    );
    assert!(
        !project_dir.join("align-optics").exists(),
        "stale plan rejection should not apply the new canonical rename without confirmation"
    );
}

#[test]
fn alignment_apply_rejects_path_tampered_submitted_operations_before_any_folder_is_renamed() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let task_dir = project_dir.join("analysis notes");
    let outside_target = workspace_dir.path().join("outside-project-target");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&task_dir).expect("task directory should be created");
    fs::write(task_dir.join("notes.txt"), "keep this file").expect("user file should be written");
    task::adopt(&task_dir, "calibrate-laser").expect("task adopt should succeed");

    let plan = app_facade::preview_task_folder_alignment(&project_dir)
        .expect("alignment plan should load");
    let mut tampered_plan_json =
        serde_json::to_value(&plan).expect("alignment plan should serialize");
    tampered_plan_json["operations"][0]["renameTaskFolder"]["to"] =
        serde_json::json!(outside_target);
    let tampered_plan = serde_json::from_value(tampered_plan_json)
        .expect("path-tampered alignment payload should deserialize like a Tauri request");

    let error = app_facade::apply_task_folder_alignment(&project_dir, &tampered_plan)
        .expect_err("alignment apply should reject path-tampered submitted operations");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("canonical folder alignment plan"),
        "alignment apply should explain that the submitted path is not canonical: {rendered_error}"
    );

    assert!(
        task_dir.is_dir(),
        "path-tampered plan rejection should leave the task folder unmoved"
    );
    assert!(
        !project_dir.join("calibrate-laser").exists(),
        "path-tampered plan rejection should not apply the canonical rename either"
    );
    assert!(
        !outside_target.exists(),
        "path-tampered plan rejection should not create the outside target"
    );
}

#[test]
fn alignment_plan_rejects_nested_task_buckets_without_mutating_files() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let outer_task_dir = project_dir.join("outer task");
    let nested_task_dir = outer_task_dir.join("inner task");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    task::create(&project_dir, "outer task").expect("outer task should be created");
    fs::create_dir(&nested_task_dir).expect("nested task directory should be created");
    task::adopt(&nested_task_dir, "inner task").expect("nested task should be adopted");
    fs::write(nested_task_dir.join("notes.txt"), "keep this file")
        .expect("nested task file should be written");

    let error = app_facade::preview_task_folder_alignment(&project_dir)
        .expect_err("nested task buckets should be rejected by the strict project shape");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("nested task bucket")
            && rendered_error.contains(&nested_task_dir.display().to_string())
            && rendered_error.contains(&outer_task_dir.display().to_string()),
        "nested task bucket rejection should identify the nested and ancestor buckets: {rendered_error}"
    );

    assert!(
        nested_task_dir.is_dir(),
        "planning should not remove the nested task bucket"
    );
    assert_eq!(
        fs::read_to_string(nested_task_dir.join("notes.txt"))
            .expect("nested task user file should remain readable"),
        "keep this file"
    );
}

#[test]
fn alignment_plan_rejects_non_direct_task_buckets_without_mutating_files() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    let group_dir = project_dir.join("group");
    let nested_task_dir = group_dir.join("analysis notes");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::create_dir(&group_dir).expect("group directory should be created");
    fs::create_dir(&nested_task_dir).expect("nested task directory should be created");
    task::adopt(&nested_task_dir, "analysis").expect("nested task should be adopted");
    fs::write(nested_task_dir.join("notes.txt"), "keep this file")
        .expect("nested task file should be written");

    let error = app_facade::preview_task_folder_alignment(&project_dir)
        .expect_err("non-direct task buckets should be rejected by the strict project shape");
    let rendered_error = error.to_string();
    assert!(
        rendered_error.contains("direct children"),
        "non-direct task bucket rejection should mention the strict direct-child project model: {rendered_error}"
    );

    assert!(
        nested_task_dir.is_dir(),
        "alignment planning should leave non-direct task folders in place"
    );
    assert_eq!(
        fs::read_to_string(nested_task_dir.join("notes.txt"))
            .expect("nested task user file should remain readable"),
        "keep this file"
    );
    assert!(
        !group_dir.join("analysis").exists(),
        "alignment planning should not rename a task bucket under a user-owned parent folder"
    );
}

#[test]
fn project_metadata_rewrite_plan_updates_project_and_task_metadata_without_writing() {
    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");

    fs::create_dir(&project_dir).expect("project directory should be created");
    project::init(&project_dir).expect("project init should succeed");
    fs::write(
        project_dir.join(".spielgantt/project.json"),
        "{\n  \"schema_version\": 1,\n  \"folder_naming\": \"task_id\",\n  \"events\": [\n    \"START\",\n    \"MOT\"\n  ]\n}\n",
    )
    .expect("project metadata should be written with events");
    task::create(&project_dir, "prepare sample").expect("ending task should be created");
    fs::write(
        project_dir.join("prepare sample/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"prepare sample\",\n  \"ends_at\": \"START\"\n}\n",
    )
    .expect("ending task metadata should be written");
    task::create(&project_dir, "analyze results").expect("dependent task should be created");
    fs::write(
        project_dir.join("analyze results/.spielgantt/task.json"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"analyze results\",\n  \"dependencies\": [\n    \"START\",\n    \"prepare sample\"\n  ]\n}\n",
    )
    .expect("dependent task metadata should be written");

    let plan = mutation_plan::plan_project_metadata_rewrites(
        &project_dir,
        |project_metadata| {
            for event in project_metadata.events.get_or_insert_with(Vec::new) {
                if event == "START" {
                    *event = "BEGIN".to_string();
                }
            }
        },
        |_, task_metadata| {
            for dependency in &mut task_metadata.dependencies {
                if dependency == "START" {
                    *dependency = "BEGIN".to_string();
                }
            }
            if task_metadata.ends_at.as_deref() == Some("START") {
                task_metadata.ends_at = Some("BEGIN".to_string());
            }
        },
    )
    .expect("metadata rewrite plan should be built");

    let expected_events = vec!["BEGIN".to_string(), "MOT".to_string()];
    let updated_project = ProjectMetadata::from_json(plan.project_rewrite().updated_contents())
        .expect("updated project rewrite should remain valid metadata");
    assert_eq!(
        updated_project.events.as_deref(),
        Some(expected_events.as_slice()),
        "project rewrite should expose updated project metadata"
    );
    let original_project = ProjectMetadata::from_json(plan.project_rewrite().original_contents())
        .expect("original project rewrite should remain valid metadata");
    assert!(
        original_project
            .events
            .as_deref()
            .is_some_and(|events| events.contains(&"START".to_string())),
        "project rewrite should retain original project metadata for review"
    );

    let updated_tasks = plan
        .task_rewrites()
        .iter()
        .map(|rewrite| {
            TaskMetadata::from_json(rewrite.updated_contents())
                .expect("updated task rewrite should remain valid metadata")
        })
        .collect::<Vec<_>>();
    assert!(
        updated_tasks
            .iter()
            .any(|metadata| metadata.ends_at.as_deref() == Some("BEGIN")),
        "planner should update task ends_at references"
    );
    assert!(
        updated_tasks
            .iter()
            .any(|metadata| metadata.dependencies.contains(&"BEGIN".to_string())),
        "planner should update task dependency references"
    );

    let persisted_project = ProjectMetadata::from_json(
        &fs::read_to_string(project_dir.join(".spielgantt/project.json"))
            .expect("project metadata should remain readable"),
    )
    .expect("persisted project metadata should remain valid JSON");
    assert!(
        persisted_project
            .events
            .as_deref()
            .is_some_and(|events| events.contains(&"START".to_string())),
        "planning should not write project metadata"
    );
    let persisted_task = TaskMetadata::from_json(
        &fs::read_to_string(project_dir.join("prepare sample/.spielgantt/task.json"))
            .expect("task metadata should remain readable"),
    )
    .expect("persisted task metadata should remain valid JSON");
    assert_eq!(
        persisted_task.ends_at.as_deref(),
        Some("START"),
        "planning should not write task metadata"
    );
}
