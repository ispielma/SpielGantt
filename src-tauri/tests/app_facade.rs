use tempfile::tempdir;

#[test]
fn app_facade_opens_project_with_gui_refresh_payload_contract() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");
    spielgantt_lib::task::create(&project_dir, "calibrate laser")
        .expect("fixture task creation should succeed");

    let opened = spielgantt_lib::app_facade::open_project(&project_dir)
        .expect("app facade should open a project through the GUI payload contract");

    assert_eq!(opened.selected_path(), project_dir.as_path());
    assert!(opened.is_valid());
    assert_eq!(
        opened
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        vec!["calibrate laser"],
        "app facade open should assemble the task payload expected by the GUI"
    );
    assert!(
        opened.workflow().is_some(),
        "app facade open should include the Rust-owned workflow projection"
    );
}

#[test]
fn app_facade_task_mutation_returns_post_mutation_refresh_payload() {
    let workspace = tempdir().expect("workspace should be created");
    let project_dir = workspace.path().join("experiment-plan");
    std::fs::create_dir(&project_dir).expect("project directory should be created");
    spielgantt_lib::project::init(&project_dir).expect("project init should succeed");

    let result = spielgantt_lib::app_facade::create_task(&project_dir, "calibrate laser")
        .expect("app facade task creation should succeed");

    assert_eq!(
        result
            .project()
            .tasks()
            .iter()
            .map(|task| task.id())
            .collect::<Vec<_>>(),
        vec!["calibrate laser"],
        "app facade mutation should return a refreshed GUI project payload"
    );
    assert_eq!(
        std::fs::read_to_string(project_dir.join("calibrate laser/.spielgantt/task.json"))
            .expect("task metadata should be readable"),
        "{\n  \"schema_version\": 1,\n  \"id\": \"calibrate laser\"\n}\n",
        "task creation should still persist through the core package mutation path"
    );
}
