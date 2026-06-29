#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;

pub fn run_spielgantt_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spielgantt"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run spielgantt")
}

pub fn run_spielgantt(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_spielgantt"))
        .args(args)
        .output()
        .expect("failed to run spielgantt")
}

pub fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid utf-8")
}

pub fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be valid utf-8")
}

pub fn stdout_json(output: &Output) -> Value {
    serde_json::from_str(&stdout_text(output)).expect("stdout should be valid JSON")
}

pub fn run_spielgantt_json_in(dir: &Path, args: &[&str]) -> Value {
    let output = run_spielgantt_in(dir, args);
    assert!(
        output.status.success(),
        "JSON command should succeed for args {args:?}: {output:?}"
    );
    stdout_json(&output)
}

pub fn json_array<'a>(json: &'a Value, field: &str) -> &'a Vec<Value> {
    json[field]
        .as_array()
        .unwrap_or_else(|| panic!("JSON field '{field}' should be an array: {json}"))
}

pub fn find_json_object_by_str<'a>(items: &'a [Value], field: &str, expected: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item[field] == expected)
        .unwrap_or_else(|| panic!("expected JSON object with {field}={expected:?}: {items:?}"))
}

pub fn init_project(workspace_dir: &Path, project_name: &str) -> Output {
    let project_dir = workspace_dir.join(project_name);
    fs::create_dir(&project_dir).expect("project directory should be created");
    run_spielgantt_in(workspace_dir, &["init", project_name])
}

pub fn create_unprepared_project(workspace_dir: &Path, project_name: &str) -> PathBuf {
    let project_dir = workspace_dir.join(project_name);
    fs::create_dir(&project_dir).expect("project directory should be created");
    write_project_metadata(
        &project_dir,
        "\
{
  \"schema_version\": 1,
  \"folder_naming\": \"task_id\"
}
",
    );
    project_dir
}

pub fn assert_agent_ready_project(project_dir: &Path) {
    assert!(
        project_dir.join("AGENTS.md").is_file(),
        "project should contain AGENTS.md agent guidance"
    );
    for skill_name in [
        "review-spielgantt",
        "setup-spielgantt",
        "update-spielgantt",
        "use-spielgantt",
    ] {
        assert!(
            project_dir
                .join(".agents/skills")
                .join(skill_name)
                .join("SKILL.md")
                .is_file(),
            "project should contain the {skill_name} project-local skill"
        );
    }
    assert!(
        project_dir.join(".spielgantt/agent.json").is_file(),
        "project should contain local agent metadata"
    );
}

pub fn write_project_metadata(project_dir: &Path, contents: &str) {
    let metadata_dir = project_dir.join(".spielgantt");
    fs::create_dir_all(&metadata_dir).expect("project metadata directory should be created");
    fs::write(metadata_dir.join("project.json"), contents)
        .expect("project metadata should be written");
}

pub fn create_task(project_dir: &Path, id: &str) -> Output {
    run_spielgantt_in(project_dir, &["task", "create", id])
}

pub fn write_task_metadata(task_dir: &Path, id: &str) {
    fs::create_dir(task_dir).expect("task directory should be created");
    fs::create_dir(task_dir.join(".spielgantt"))
        .expect("task metadata directory should be created");
    fs::write(
        task_dir.join(".spielgantt/task.json"),
        format!("{{\n  \"schema_version\": 1,\n  \"id\": \"{id}\"\n}}\n"),
    )
    .expect("task metadata should be written");
}

pub fn task_dir(project_dir: &Path, id: &str) -> PathBuf {
    project_dir.join(id)
}
