# Agent CLI Contract

This document audits the CLI surface that setup and update agents need in
order to work on SpielGantt projects without the GUI. It is a development
contract for this repository. Generated project-agent command instructions are
canonical only in the generated `use-spielgantt` skill; workflow skills must
delegate CLI syntax to that skill instead of duplicating command reference
material.

`AGENTS.md` is the authoritative project-level vision and boundary contract.
This document narrows that contract to the CLI and agent-facing JSON surface.

SpielGantt project mutations for agents must go through shared Rust behavior
exposed by the CLI. User files outside `.spielgantt/` remain ordinary project
content and must not be treated as owned application data.

## Package Boundary

The Rust backend is the package-domain library for the CLI. Agent-facing setup,
inspection, and mutation workflows therefore use CLI commands backed by shared
Rust package behavior. Any GUI backend call that affects package semantics must
map to the same CLI-expressible behavior and, where automation needs stable
machine output, documented JSON.

GUI/session and desktop-only workflows are outside this CLI contract. For
example, remembered-project sidebar state, visual timeline placement, file
watch refresh triggers, and whole-project deletion are not package mutations and
must not create agent CLI commands.

## Generated Command Tree

The following list is generated from Clap usage output and is locked by
`src-tauri/tests/agent_cli_contract.rs`.

<!-- BEGIN GENERATED CLI COMMAND LIST -->
```text
spielgantt <COMMAND>
spielgantt init [PATH]
spielgantt validate [OPTIONS] [PATH]
spielgantt export [PATH]
spielgantt cache <COMMAND>
spielgantt cache rebuild [PATH]
spielgantt repair [PATH]
spielgantt normalize [OPTIONS] [PATH]
spielgantt agent <COMMAND>
spielgantt agent prepare [OPTIONS] [PATH]
spielgantt agent runtime [OPTIONS]
spielgantt agent status [OPTIONS] [PATH]
spielgantt agent snapshot [OPTIONS] [PATH]
spielgantt event <COMMAND>
spielgantt event list [OPTIONS]
spielgantt event create <EVENT_NAME>
spielgantt event rename <EVENT_NAME> <EVENT_NAME>
spielgantt event delete <EVENT_NAME>
spielgantt project <COMMAND>
spielgantt project update-readme [OPTIONS] --content <CONTENT> --expected-version <README_VERSION>
spielgantt task <COMMAND>
spielgantt task adopt --id <TASK_NAME> <TASK_FOLDER>
spielgantt task adoptable-folders [OPTIONS]
spielgantt task create <TASK_NAME>
spielgantt task insert-before [OPTIONS] <SELECTED_TASK_NAME> <TASK_NAME>
spielgantt task insert-after [OPTIONS] <SELECTED_TASK_NAME> <TASK_NAME>
spielgantt task readme [OPTIONS] <TASK_NAME>
spielgantt task list [OPTIONS]
spielgantt task rename <TASK_NAME> <TASK_NAME>
spielgantt task delete [OPTIONS] <TASK_NAME>
spielgantt task ends-at [OPTIONS] <TASK_NAME> [EVENT_NAME]
spielgantt task depend <TASK_NAME> <TASK_NAME>
spielgantt task dependency <COMMAND>
spielgantt task dependency remove <TASK_NAME> <BLOCKER_NAME>
spielgantt task relationships [OPTIONS]
spielgantt task workflow [OPTIONS]
spielgantt task open [OPTIONS] <TASK_NAME>
spielgantt task update [OPTIONS] <TASK_NAME>
spielgantt task show [OPTIONS] <TASK_ID>
```
<!-- END GENERATED CLI COMMAND LIST -->

## Existing Agent Operations

| Agent operation | Existing command | Contract notes |
| --- | --- | --- |
| Initialize a project in an empty or existing folder | `spielgantt init [PATH]` | Creates `.spielgantt/project.json` and prepares project-local agent scaffolding by default. Reruns refresh generated agent files safely. |
| Validate project structure and metadata | `spielgantt validate [--json] [PATH]` | Required after setup or update mutations. JSON reports include `valid`, `project_root`, and `issues`. |
| Export a readable project snapshot | `spielgantt export [PATH]` | Human-readable Markdown for review; not the stable machine snapshot. |
| Rebuild disposable cache | `spielgantt cache rebuild [PATH]` | Recreates cache from canonical metadata; cache is not durable truth. |
| Report repair issues | `spielgantt repair [PATH]` | Non-destructive report path for moved or inconsistent metadata/cache state. |
| Preview and apply folder normalization | `spielgantt normalize [PATH]`, `spielgantt normalize --apply [PATH]` | Aligns task folder basenames with task IDs when safe. |
| Prepare or refresh project-local agent scaffolding | `spielgantt agent prepare [--json] [PATH]` | Writes managed project `AGENTS.md`, `.agents/skills`, and `.spielgantt/agent.json` metadata while preserving unrelated files. |
| Report runtime CLI path and package context | `spielgantt agent runtime --json` | Reports the executable path agents should record in local scaffolding metadata, the package context, and the SpielGantt version. |
| Report agent readiness and validation state | `spielgantt agent status [--json] [PATH]` | Reports whether project-local agent guidance, skills, and metadata are present. |
| Inspect an agent-readable project snapshot | `spielgantt agent snapshot --json [PATH]` | Read-only machine snapshot for agents to inspect before interviewing the user or mutating through other CLI commands. |
| Create a task bucket | `spielgantt task create <TASK_NAME>` | Creates the task folder and `.spielgantt/task.json` through shared Rust behavior. |
| Insert a task before or after an existing task | `spielgantt task insert-before <SELECTED_TASK_NAME> <TASK_NAME> --json`, `spielgantt task insert-after <SELECTED_TASK_NAME> <TASK_NAME> --json` | Creates the task and rewires relative blockers, downstream task references, and transferred `ends_at` metadata through one shared Rust operation. JSON output includes `schema_version`, `mode`, `selected_task_id`, `inserted_task_id`, and `task_path`. |
| Adopt an existing user folder as a task bucket | `spielgantt task adopt --id <TASK_NAME> <TASK_FOLDER>` | Preserves user files and adds task metadata to the chosen folder. |
| Rename a task identity | `spielgantt task rename <TASK_NAME> <TASK_NAME>` | Refactors task identity and updates task references through shared behavior. |
| Delete a task from the chart | `spielgantt task delete <TASK_NAME> [--remove-from-chart\|--delete-directory] [--json]` | Defaults to `--remove-from-chart`, which removes SpielGantt metadata while preserving user files. `--delete-directory` removes the whole task bucket. Both modes remove stale blocker references through shared Rust behavior. |
| Update task status metadata | `spielgantt task update <TASK_NAME> --status <STATUS>` | Supports `--status` with `blocked`, `unblocked`, or `done`. |
| List tasks | `spielgantt task list [--json]` | JSON reports include a `tasks` array with task identity and state fields. |
| Inspect one task | `spielgantt task show [--json] <TASK_ID>` | JSON reports include task path, dependencies, and state fields. |
| Create or locate a task README | `spielgantt task readme --open --dry-run <TASK_NAME>` | README files are ordinary Markdown user files; existing content is preserved. Use `--dry-run` with `--open` when previewing the opener target. |
| Add a task dependency | `spielgantt task depend <TASK_NAME> <TASK_NAME>` | The second ID blocks the first. Dependency IDs resolve in the shared task-event namespace. |
| Remove a task dependency | `spielgantt task dependency remove <TASK_NAME> <BLOCKER_NAME>` | Removes the literal blocker ID from the task metadata through shared Rust behavior. |
| Inspect dependency relationships | `spielgantt task relationships --json` | Versioned domain JSON reports direct blockers, reverse task relationships, valid dependency targets, event references, and event deletion blockers. |
| Inspect event-axis workflow semantics | `spielgantt task workflow --json` | Versioned domain JSON reports events, task/event dependency edges, `ends_at` references, valid dependency and event targets, unresolved references, invalid references, and workflow validation diagnostics without visual layout fields. |
| End a task at an event | `spielgantt task ends-at <TASK_NAME> <EVENT_NAME>` | `--clear` removes the endpoint. `ends_at` may only target an event. |
| Open or dry-run task paths | `spielgantt task open [--dry-run] <TASK_NAME>` | Agents should prefer `--dry-run` unless the user explicitly wants an OS opener. SpielGantt does not maintain task-link metadata; user-created files and filesystem links inside project folders are ordinary user content. |
| List project events | `spielgantt event list [--json]` | JSON reports include project events in metadata order. |
| Create a project event | `spielgantt event create <EVENT_NAME>` | Events are metadata only and do not have folders. |
| Rename a project event | `spielgantt event rename <EVENT_NAME> <EVENT_NAME>` | Updates event references in task metadata. |
| Delete a project event | `spielgantt event delete <EVENT_NAME>` | Rejects unsafe deletes when task metadata still references the event. |

## Shared Task-Event Namespace

Task IDs and event IDs share one project-wide namespace. Agents must avoid
creating a task and event with the same name, and validation rejects collisions.
Task dependencies may target either tasks or events. `ends_at` may only target
existing project events. Events are project metadata only; agents must not
create event folders.

No package mutation required by setup or update agents is GUI-only after the
planned agent-readiness commands below exist. Current task, event, dependency,
normalization, validation, export, repair, and cache operations already have
CLI entrypoints backed by shared Rust behavior.

## Stable Agent JSON Output

The following JSON output shapes are stable agent contracts. Human-readable
output remains the default for all commands except `agent snapshot`, whose
output is always the machine-readable snapshot shape documented below.
Every stable JSON root includes `schema_version: 1`.

`spielgantt validate --json [PATH]` returns a validation report on stdout. The
command exits successfully when `valid` is `true` and exits nonzero when
`valid` is `false`.

```json
{
  "schema_version": 1,
  "valid": true,
  "project_root": "/absolute/project/root",
  "issues": []
}
```

For invalid projects, `issues` is an array of human-readable validation
messages and `project_root` remains the resolved project root when available.

`spielgantt task list --json` returns:

```json
{
  "schema_version": 1,
  "tasks": [
    {
      "id": "analyze-results",
      "status": "unblocked"
    }
  ]
}
```

Unspecified task state fields are `null`.

`spielgantt task show --json <TASK_NAME>` returns:

```json
{
  "schema_version": 1,
  "id": "analyze-results",
  "path": "/absolute/project/root/analyze-results",
  "dependencies": ["START"],
  "status": "unblocked"
}
```

`spielgantt event list --json` returns:

```json
{
  "schema_version": 1,
  "events": ["START", "MOT", "BEC"]
}
```

`spielgantt task adoptable-folders --json` returns:

```json
{
  "schema_version": 1,
  "folders": [
    {
      "folderPath": "/absolute/project/root/analysis notes",
      "projectRelativePath": "analysis notes",
      "taskId": "analysis notes"
    }
  ]
}
```

`spielgantt task delete <TASK_NAME> --remove-from-chart --json` and
`spielgantt task delete <TASK_NAME> --delete-directory --json` return a
committed package mutation report:

```json
{
  "schema_version": 1,
  "task_id": "collect-samples",
  "mode": "remove-from-chart",
  "path": "/absolute/project/root/collect-samples",
  "committed": true,
  "cleanup": null
}
```

`mode` is `remove-from-chart` or `delete-directory`. If post-commit cleanup
fails, `cleanup` reports `{ "status": "failed", "path": "...", "error": "..." }`
and stderr explains the manual cleanup action.

`spielgantt agent status --json [PATH]` returns:

```json
{
  "schema_version": 1,
  "project_root": "/absolute/project/root",
  "agent": {
    "ready": false,
    "agents_md_present": false,
    "skills_dir_present": false,
    "metadata_present": false,
    "recorded_cli_path": null
  },
  "validation": {
    "schema_version": 1,
    "valid": true,
    "project_root": "/absolute/project/root",
    "issues": []
  }
}
```

`ready` is true only when project-local `AGENTS.md`, all bundled
`.agents/skills/*/SKILL.md` files, and parseable `.spielgantt/agent.json`
metadata with `cli_path` are present. `recorded_cli_path` is read from
`.spielgantt/agent.json` when that metadata exists and contains `cli_path`.
The JSON form exits successfully when a SpielGantt project can be inspected,
even when `validation.valid` is `false`; agents must use the validation payload
instead of the process status to decide whether repair is needed.

`spielgantt agent runtime --json` returns:

```json
{
  "schema_version": 1,
  "version": "1.0.0-rc.1",
  "executable_path": "/absolute/path/to/spielgantt",
  "package_context": {
    "platform": "macos",
    "kind": "macos_app_bundle",
    "package_path": "/Applications/SpielGantt.app"
  }
}
```

`executable_path` is resolved from the currently running process and does not
assume `/Applications`, a fixed install directory, or a global CLI install.
Agents should record `executable_path` when preparing project-local metadata.
`package_context.kind` is one of:

- `macos_app_bundle` when the executable is inside
  `<AppName>.app/Contents/MacOS/`.
- `windows_executable` when the executable path is a Windows `.exe`; the
  `package_path` is the containing install directory.
- `linux_appimage` when the AppImage runtime exposes the `APPIMAGE`
  environment variable; the `package_path` is the AppImage file.
- `standalone_executable` for unpackaged binaries such as Cargo-built test or
  development binaries; `package_path` is `null`.

`spielgantt agent prepare --json [PATH]` returns:

```json
{
  "schema_version": 1,
  "project_root": "/absolute/project/root",
  "outcome": "created",
  "files": [
    { "path": "AGENTS.md", "status": "created" },
    { "path": ".agents/skills/use-spielgantt/SKILL.md", "status": "created" },
    { "path": ".agents/skills/setup-spielgantt/SKILL.md", "status": "created" },
    { "path": ".agents/skills/update-spielgantt/SKILL.md", "status": "created" },
    { "path": ".agents/skills/review-spielgantt/SKILL.md", "status": "created" },
    { "path": ".spielgantt/agent.json", "status": "created" }
  ],
  "agent": {
    "ready": true,
    "agents_md_present": true,
    "skills_dir_present": true,
    "metadata_present": true,
    "recorded_cli_path": "/absolute/path/to/spielgantt"
  }
}
```

`outcome` and each file `status` are `created`, `refreshed`, or `unchanged`.
The command refreshes generated scaffold files that carry the SpielGantt
generated marker and does not remove unrelated project files or additional
project-local skills. It refuses to overwrite preexisting unmarked `AGENTS.md`
or same-named project-local skill files and exits nonzero without writing a
partial scaffold. Local metadata is written to `.spielgantt/agent.json` with
`schema_version`, `cli_path`, and `version`.

`spielgantt init [PATH]` uses the same scaffold preparation behavior after
project metadata is present, so a newly initialized CLI project is agent-ready
without a separate install step. GUI project creation and GUI onboarding also
prepare or refresh the same managed scaffold files.

`spielgantt agent snapshot [--json] [PATH]` returns a read-only project
snapshot:

```json
{
  "schema_version": 1,
  "project_root": "/absolute/project/root",
  "agent": {
    "ready": true,
    "agents_md_present": true,
    "skills_dir_present": true,
    "metadata_present": true,
    "recorded_cli_path": "/absolute/path/to/spielgantt"
  },
  "validation": {
    "schema_version": 1,
    "valid": true,
    "project_root": "/absolute/project/root",
    "issues": []
  },
  "paths": {
    "project_metadata": ".spielgantt/project.json",
    "agents_md": "AGENTS.md",
    "skills_dir": ".agents/skills",
    "agent_metadata": ".spielgantt/agent.json"
  },
  "tasks": [
    {
      "id": "analyze-results",
      "path": "/absolute/project/root/analyze-results",
      "project_relative_path": "analyze-results",
      "dependencies": ["START", "prepare-samples"],
      "dependency_references": [
        { "id": "START", "kind": "event" },
        { "id": "prepare-samples", "kind": "task" }
      ],
      "ends_at": null,
      "status": "unblocked"
    }
  ],
  "events": ["START", "BEC"],
  "dependencies": [
    { "task_id": "analyze-results", "id": "START", "kind": "event" },
    { "task_id": "analyze-results", "id": "prepare-samples", "kind": "task" }
  ]
}
```

Snapshot output reports current project state and relevant project paths. It
does not expose disposable cache internals. Snapshot requires a valid
SpielGantt project and exits nonzero instead of emitting a partial snapshot when
validation fails.

`spielgantt task relationships --json` returns dependency relationship domain
data on stdout. The schema is intentionally not a GUI view model; it exposes
workflow semantics that GUI and CLI automation can consume directly.

```json
{
  "schema_version": 1,
  "tasks": [
    {
      "id": "prepare-samples",
      "blockers": [{ "id": "START", "kind": "event" }],
      "blocks": [{ "id": "analyze-results", "kind": "task" }],
      "valid_dependency_targets": [{ "id": "MOT", "kind": "event" }]
    }
  ],
  "events": [
    {
      "id": "START",
      "references": [{ "task_id": "prepare-samples", "kind": "dependency" }],
      "deletion_blockers": [
        { "task_id": "prepare-samples", "kind": "dependency" }
      ]
    },
    {
      "id": "BEC",
      "references": [{ "task_id": "prepare-samples", "kind": "ends_at" }],
      "deletion_blockers": [
        { "task_id": "prepare-samples", "kind": "ends_at" }
      ]
    }
  ]
}
```

`spielgantt task workflow --json` returns event-axis workflow domain data on
stdout. The schema is intentionally not a GUI view model and must not prescribe
row placement, lane packing, CSS grid columns, connector drawing, or label
placement.

```json
{
  "schema_version": 1,
  "project_root": "/absolute/project/root",
  "events": ["START", "BEC"],
  "tasks": [
    {
      "id": "prepare-samples",
      "dependency_references": [
        { "id": "START", "kind": "event", "valid": true }
      ],
      "ends_at_reference": { "id": "BEC", "valid": true },
      "valid_dependency_targets": [{ "id": "MOT", "kind": "event" }],
      "valid_ends_at_targets": ["START", "BEC"],
      "unresolved_references": [],
      "invalid_references": [],
      "validation_diagnostics": []
    }
  ],
  "edges": [
    {
      "from": { "id": "prepare-samples", "kind": "task" },
      "to": { "id": "START", "kind": "event" },
      "kind": "dependency"
    },
    {
      "from": { "id": "BEC", "kind": "event" },
      "to": { "id": "prepare-samples", "kind": "task" },
      "kind": "ends_at"
    }
  ],
  "validation": {
    "valid": true,
    "diagnostics": []
  }
}
```

Agents should use `spielgantt agent snapshot --json [PATH]` as the first
machine-readable inspection step before setup or update interviews. Generated
workflow guidance should not present scraped human output as a stable machine
contract.
