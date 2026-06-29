mod support;

use support::{run_spielgantt, stderr_text as stderr, stdout_text as stdout};

#[test]
fn help_and_version_flags_work() {
    let help = run_spielgantt(&["--help"]);
    assert!(
        help.status.success(),
        "help exited unsuccessfully: {help:?}"
    );

    let help_stdout = stdout(&help);
    assert!(
        help_stdout.contains("local-first scientific workflow planner"),
        "help output did not describe the app: {help_stdout}"
    );
    assert!(
        help_stdout.contains("Usage: spielgantt <COMMAND>"),
        "help output did not include clap's generated top-level usage: {help_stdout}"
    );
    assert!(
        help_stdout.contains("Commands:"),
        "help output did not include clap's generated command list: {help_stdout}"
    );
    assert!(
        help_stdout.contains("init"),
        "help output did not include the init command: {help_stdout}"
    );
    assert!(
        help_stdout.contains("task"),
        "help output did not include the task command: {help_stdout}"
    );
    assert!(
        help_stdout.contains("event"),
        "help output did not include the event command: {help_stdout}"
    );
    assert!(
        !help_stdout.contains("spielgantt task create <id>"),
        "top-level help should rely on clap's generated command list, not the old manual command summary: {help_stdout}"
    );

    let version = run_spielgantt(&["--version"]);
    assert!(
        version.status.success(),
        "version exited unsuccessfully: {version:?}"
    );

    let version_stdout = stdout(&version);
    assert!(
        version_stdout.contains(env!("CARGO_PKG_VERSION")),
        "version output did not contain the package version: {version_stdout}"
    );
}

#[test]
fn task_link_is_not_a_spielgantt_cli_contract() {
    let task_help = run_spielgantt(&["task", "--help"]);
    assert!(
        task_help.status.success(),
        "task help should succeed: {task_help:?}"
    );
    let task_help_stdout = stdout(&task_help);
    assert!(
        !task_help_stdout.contains("link"),
        "task help should not advertise SpielGantt-owned task links: {task_help_stdout}"
    );

    let output = run_spielgantt(&["task", "link", "add", "analyze-results"]);
    assert!(
        !output.status.success(),
        "removed task link command should fail: {output:?}"
    );

    let stderr = stderr(&output);
    assert!(
        stderr.contains("unrecognized subcommand"),
        "removed task link command should be rejected by the parser: {stderr}"
    );
    assert!(
        stderr.contains("Usage:"),
        "missing argument output should include usage guidance: {stderr}"
    );
}

#[test]
fn task_create_and_adopt_help_use_human_task_name_labels() {
    let create_help = run_spielgantt(&["task", "create", "--help"]);
    assert!(
        create_help.status.success(),
        "task create help should succeed: {create_help:?}"
    );
    let create_help_stdout = stdout(&create_help);
    assert!(
        create_help_stdout.contains("Usage: spielgantt task create <TASK_NAME>"),
        "task create help should describe the task name argument: {create_help_stdout}"
    );
    assert!(
        !create_help_stdout.contains("<ID>"),
        "task create help should not imply slug-only ids: {create_help_stdout}"
    );

    let adopt_help = run_spielgantt(&["task", "adopt", "--help"]);
    assert!(
        adopt_help.status.success(),
        "task adopt help should succeed: {adopt_help:?}"
    );
    let adopt_help_stdout = stdout(&adopt_help);
    assert!(
        adopt_help_stdout.contains("--id <TASK_NAME>"),
        "task adopt help should label the task name flag clearly: {adopt_help_stdout}"
    );
    assert!(
        !adopt_help_stdout.contains("--id <ID>"),
        "task adopt help should not imply slug-only ids: {adopt_help_stdout}"
    );

    let event_help = run_spielgantt(&["event", "--help"]);
    assert!(
        event_help.status.success(),
        "event help should succeed: {event_help:?}"
    );
    let event_help_stdout = stdout(&event_help);
    assert!(
        event_help_stdout.contains("Usage: spielgantt event <COMMAND>"),
        "event help should describe the event subcommands: {event_help_stdout}"
    );
    assert!(
        event_help_stdout.contains("list"),
        "event help should include the list command: {event_help_stdout}"
    );
    assert!(
        event_help_stdout.contains("create"),
        "event help should include the create command: {event_help_stdout}"
    );
    assert!(
        event_help_stdout.contains("rename"),
        "event help should include the rename command: {event_help_stdout}"
    );
}
