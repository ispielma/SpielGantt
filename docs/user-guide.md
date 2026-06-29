# SpielGantt User Guide

SpielGantt keeps a scientific workflow as an ordinary project folder. The GUI
and CLI read and write the same hidden JSON metadata, so the folder remains
useful without SpielGantt.

## Project Layout

A project root is any folder containing `.spielgantt/project.json`.

```text
fluorescence-timecourse/
  .spielgantt/
    project.json
  literature-review/
    .spielgantt/
      task.json
    README.md
  collect-fluorescence/
    .spielgantt/
      task.json
    data/
      raw-observations.csv
```

A task is any ordinary folder containing `.spielgantt/task.json`. Files outside
`.spielgantt/` are user files. SpielGantt should preserve those notes, data
exports, notebooks, and protocol files rather than treating them as owned
application data.

The checked-in example project is `examples/fluorescence-timecourse`. It has
events, task endpoints, and dependencies that the GUI can show on the timeline.

## IDs And Folder Names

Task IDs are the durable identity. IDs are human-readable names that fit within
a single filesystem path component, such as `Prepare Samples`,
`2026_04_15 RbK`, or `analyze-results`. Folder names are labels, not identity.

When the project uses the `task_id` folder naming policy, folder normalization
renames task folders so their names match task IDs. A rename refactor changes a
task ID and updates dependency references that pointed at the old ID. User files
inside the task folder move with the folder.

## Events And Timeline

Projects can define metadata-only events under the `Events` sidebar heading.
Use `Create Event` to add them, and use `Rename` or `Delete` from the event
rows to manage the list. Selecting an event opens a read-only event inspector
showing the tasks that block the event and the tasks blocked by that event.

Dependency pickers label valid choices as `Task: <id>` and `Event: <id>`, so
event targets stay distinguishable from task targets. The task inspector's
`End at event` control only accepts project events.

The `Timeline` view uses project events as ordered vertical milestone lines.
The space between two adjacent event lines is the interval between those
events. A task with a valid event dependency and `ends_at` event is rendered as
a bar spanning from its latest valid event dependency line to its `ends_at`
event line. Tasks without enough event context, or with layout conflicts, are
not rendered as extra timeline text cards.

Timeline rows are ordered to read like a conventional Gantt chart: tasks that
must occur first appear higher in the chart, allowing later bars to cascade
downward as the event axis advances. Valid task blockers are kept above their
dependents. When there is no task dependency ordering between two bars, rows
are sorted by start event, then end event, then task name for stable display.

## CLI Workflow

During development, set `SPIELGANTT_REPO` to this checkout and use
`cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- ...`.
The packaged `spielgantt` binary uses the same arguments without the Cargo
prefix.

Create a project:

```sh
export SPIELGANTT_REPO=/path/to/SpielGantt
mkdir my-experiment
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- init my-experiment
```

Project initialization also prepares the project for coding agents by writing
managed `AGENTS.md`, `.agents/skills`, and `.spielgantt/agent.json` files. If
you already have an unmarked `AGENTS.md` or same-named local skill, SpielGantt
preserves it and reports the conflict instead of overwriting it.

Create a task folder from a task ID:

```sh
cd my-experiment
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task create literature-review
```

Adopt an existing folder without moving user files:

```sh
mkdir analysis-notes
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task adopt analysis-notes --id analyze-results
```

Preview and apply folder normalization:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- normalize
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- normalize --apply
```

Validate a project:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- validate .
```

Add a dependency, where the second task blocks the first:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task depend analyze-results literature-review
```

Set or clear a Task event endpoint:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task ends-at analyze-results FINAL-REVIEW
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task ends-at --clear analyze-results
```

Open a task folder when you want to inspect or manage user-owned files,
including filesystem-level links that you placed there:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task open analyze-results --dry-run
```

Rename a task ID as a refactor:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- task rename analyze-results analyze-timecourse
```

Export a readable Markdown snapshot:

```sh
cargo run --manifest-path "$SPIELGANTT_REPO/src-tauri/Cargo.toml" -- export .
```

## GUI Workflow

The GUI sidebar keeps remembered projects across launches and keeps the
project navigator as the primary entry point.

Use `New Project...` to create `~/Documents/Projects/<Project Name>/`,
initialize it, prepare agent scaffolding, remember it, and open it. Use `Open Existing Project...` to choose a folder, initialize it if needed, adopt each
direct child folder as a task bucket, safely prepare or refresh generated agent
scaffolding, remember it, and open it.

Task identity is shown as `Task name` in the GUI. Direct project child folders
are task buckets, and each task bucket basename matches the task name. The GUI
does not expose the old adopt-folder control or bulk normalize controls; those
remain CLI-only flows through `task adopt` and `normalize --apply`.

Use `Open remembered project <project-name>` to switch between remembered
projects, and `Open project folder` to show the active project in the platform
file browser. The `Timeline` view shows task placement when the project defines
events. Projects without events list tasks that need placement until events are
added.

Use `Remove from Sidebar` only to forget a remembered project shortcut; it does
not delete or move files. Use `Delete Project...` only when you want the GUI to
move the entire project folder to the operating system trash/recycle bin where
supported. The delete confirmation names the exact folder and warns that all
contents inside it will be deleted, including ordinary user files that are not
owned by SpielGantt.

Use `Create Task` with `New task name` to create a new task.

Use `Select task <id>` in the task list to open a task in the inspector. A task
bar on the event timeline can be clicked to select that task, and right-clicking
that bar opens the same task menu as right-clicking the task in the sidebar.
Use the sidebar or timeline task menu's `Open task folder <id>` action to show
the task bucket in the platform file browser. Edits to `Task status` and
`Task README` autosave through the task inspector; README text is saved when
you leave the README field.

Use `Task blocker` and `Add blocker to selected task` to add dependencies. Use
`Remove blocker <id>` to remove one. Use `Task event endpoint` to anchor the
selected task to a project event or clear its current event endpoint.
