mod support;

use std::{fs, path::Path};

use tempfile::tempdir;

use support::{
    run_spielgantt, run_spielgantt_in, stderr_text as stderr, stdout_json, stdout_text as stdout,
};

#[test]
fn documented_agent_cli_command_list_matches_clap_usage_tree() {
    let documented = documented_command_list();
    let actual = collect_usage_tree(&[]);

    assert_eq!(
        documented, actual,
        "docs/agent-cli-contract.md command list must match clap-generated usage"
    );
}

#[test]
fn representative_command_families_keep_public_output_and_exit_contracts() {
    let documented = documented_command_list();
    assert!(
        documented.contains(&"spielgantt task relationships [OPTIONS]".to_string()),
        "generated command list should include task relationship inspection"
    );
    assert!(
        documented.contains(&"spielgantt event list [OPTIONS]".to_string()),
        "generated command list should include event inspection"
    );

    let workspace_dir = tempdir().expect("temporary directory should be created");
    let project_dir = workspace_dir.path().join("project");
    fs::create_dir(&project_dir).expect("project directory should be created");

    let init_output = run_spielgantt_in(workspace_dir.path(), &["init", "project"]);
    assert!(
        init_output.status.success(),
        "init should succeed before representative CLI checks: {init_output:?}"
    );
    assert!(
        stdout(&init_output).contains("Initialized SpielGantt project in"),
        "init should keep its human output contract: {}",
        stdout(&init_output)
    );

    let create_output = run_spielgantt_in(&project_dir, &["task", "create", "analyze results"]);
    assert!(
        create_output.status.success(),
        "task create should succeed before representative CLI checks: {create_output:?}"
    );
    assert!(
        stdout(&create_output).contains("Created task 'analyze results'"),
        "task create should keep its human output contract: {}",
        stdout(&create_output)
    );

    let event_json = run_spielgantt_in(&project_dir, &["event", "list", "--json"]);
    assert!(
        event_json.status.success(),
        "event list JSON should succeed: {event_json:?}"
    );
    assert_eq!(
        stdout_json(&event_json),
        serde_json::json!({
            "schema_version": 1,
            "events": ["start", "finished"]
        })
    );

    let validate_output = run_spielgantt_in(&project_dir, &["validate"]);
    assert!(
        validate_output.status.success(),
        "validate should succeed for a valid project: {validate_output:?}"
    );
    assert!(
        stdout(&validate_output).contains("SpielGantt project is valid:"),
        "validate should keep its human output contract: {}",
        stdout(&validate_output)
    );

    let invalid_status = run_spielgantt_in(
        &project_dir,
        &["task", "update", "analyze results", "--status", "paused"],
    );
    assert!(
        !invalid_status.status.success(),
        "invalid task status should keep a nonzero CLI exit: {invalid_status:?}"
    );
    assert_eq!(
        invalid_status.status.code(),
        Some(2),
        "invalid task status should keep the command-line usage exit code"
    );
    assert!(
        stderr(&invalid_status)
            .contains("invalid status 'paused': expected one of blocked, unblocked, done"),
        "invalid task status should keep the public error message: {}",
        stderr(&invalid_status)
    );
}

fn documented_command_list() -> Vec<String> {
    let contract_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri should have a repository parent")
        .join("docs/agent-cli-contract.md");
    let contract = std::fs::read_to_string(&contract_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", contract_path.display()));

    let start = "<!-- BEGIN GENERATED CLI COMMAND LIST -->";
    let end = "<!-- END GENERATED CLI COMMAND LIST -->";
    let start_index = contract
        .find(start)
        .unwrap_or_else(|| panic!("missing {start} marker in {}", contract_path.display()));
    let end_index = contract
        .find(end)
        .unwrap_or_else(|| panic!("missing {end} marker in {}", contract_path.display()));

    contract[start_index + start.len()..end_index]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && *line != "```text" && *line != "```")
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_usage_tree(args: &[String]) -> Vec<String> {
    let mut help_args = args.to_vec();
    help_args.push("--help".to_string());
    let help_arg_refs = help_args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_spielgantt(&help_arg_refs);
    assert!(
        output.status.success(),
        "help command should succeed for {:?}: {output:?}",
        help_args
    );

    let help_stdout = stdout(&output);
    let usage = help_stdout
        .lines()
        .find_map(|line| line.strip_prefix("Usage: "))
        .unwrap_or_else(|| panic!("missing Usage line in help output: {help_stdout}"));

    let mut lines = vec![usage.to_string()];
    for command in command_names(&help_stdout) {
        let mut child_args = args.to_vec();
        child_args.push(command);
        lines.extend(collect_usage_tree(&child_args));
    }

    lines
}

fn command_names(help_stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_commands = false;

    for line in help_stdout.lines() {
        if line.trim() == "Commands:" {
            in_commands = true;
            continue;
        }

        if !in_commands {
            continue;
        }

        if line.trim().is_empty() || !line.starts_with("  ") {
            break;
        }

        if let Some(name) = line.split_whitespace().next() {
            if name != "help" {
                names.push(name.to_string());
            }
        }
    }

    names
}
