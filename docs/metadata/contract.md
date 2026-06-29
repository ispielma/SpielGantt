# SpielGantt Metadata Contract

SpielGantt projects and tasks are ordinary folders with hidden structured
metadata. User files outside `.spielgantt/` are not owned by SpielGantt and must
not be rewritten as part of metadata operations.

## Project Metadata

A project root is any directory containing `.spielgantt/project.json`.

Version 1 project metadata:

```json
{
  "schema_version": 1,
  "folder_naming": "task_id",
  "events": ["START", "MOT", "BEC"],
  "boundary_events": {
    "start": "START",
    "finish": "BEC"
  }
}
```

Fields:

- `schema_version`: required integer. Version `1` is the first durable
  metadata contract.
- `folder_naming`: required policy for normalizing task bucket folder names.
  Version 1 supports `task_id`, meaning task folders can be normalized from the
  canonical task `id`.
- `events`: optional ordered list of project event IDs. New projects are
  initialized with `["start", "finished"]` so tasks can be placed on the event
  axis immediately. Events are metadata only and do not create folders.
- `boundary_events`: optional explicit start and finish event IDs for the event
  axis. When present, both `start` and `finish` must be valid project event IDs
  from `events`, must be different, and use the same single-path-component
  naming rule as task IDs. New projects persist
  `{"start": "start", "finish": "finished"}`. Older project metadata without
  this field infers the first event as the start boundary and the last event as
  the finish boundary when at least two events exist.

## Task Metadata

A task bucket is any ordinary folder containing `.spielgantt/task.json`.

Version 1 task metadata:

```json
{
  "schema_version": 1,
  "id": "calibrate-laser",
  "ends_at": "BEC",
  "status": "unblocked",
  "dependencies": ["prepare-sample"]
}
```

Fields:

- `schema_version`: required integer. Version `1` is the first durable
  metadata contract.
- `id`: required user-readable task identity. The task ID is canonical; a
  separate `title` field is not required.
- `ends_at`: optional event ID naming the event where the task completes. The
  value must obey the same single-path-component naming rule as task IDs and
  project event IDs.
- Task IDs and project event IDs share one project-wide namespace. Validation
  rejects collisions between task IDs and event IDs.
- `status`: optional task state. Version 1 supports `blocked`, `unblocked`,
  and `done`. Older `planned` and `in_progress` values are read as
  `unblocked` for compatibility.
- `dependencies`: optional list of task IDs that block this task. Rename
  operations treat these IDs as references and update them when the referenced
  task ID changes.

Dependency IDs may resolve to tasks or project events in the same project, and
the dependency graph must be acyclic. A task dependency on another task that
already ends at an event is invalid; the dependency should target the event
instead. `ends_at` may only resolve to an existing project event. CLI
dependency operations reject cycles before writing; project validation also
reports missing dependency IDs, invalid `ends_at` references, and cycles
introduced by manual metadata edits.

Tasks may omit event, dependency, and status fields and remain valid project
tasks.

## User-Owned Files And Filesystem Links

SpielGantt does not maintain task-link metadata. Ordinary files, folders,
shortcuts, symlinks, aliases, or other filesystem-level links placed in project
or task folders are user-owned content. They remain outside the structured
package contract unless they are under a documented SpielGantt-owned metadata
path such as `.spielgantt/project.json`, `.spielgantt/task.json`, or disposable
cache files.

Stale files named `.spielgantt/links.json` may exist from early prototypes or
manual experimentation. Current SpielGantt versions do not read, validate,
rewrite, or expose those files as package semantics.

## Disposable Cache

SpielGantt may write cache files under project `.spielgantt/cache/`. Cache
files are internal implementation details, not durable project metadata. They
can be deleted at any time and rebuilt from `project.json`, task `task.json`
files, and ordinary filesystem scanning.

Version 1 of the internal task cache is stored at
`.spielgantt/cache/tasks.json`:

```json
{
  "schema_version": 1,
  "tasks": [
    {
      "id": "calibrate-laser",
      "path": "calibrate-laser"
    }
  ]
}
```

Fields:

- `schema_version`: required integer for internal cache compatibility. It is
  not part of the durable metadata contract.
- `tasks`: cached task IDs and project-relative task folder paths from the last
  cache rebuild.

`spielgantt cache rebuild` replaces this cache from canonical metadata.
`spielgantt repair` may compare this cache with a fresh metadata scan to report
externally moved, renamed, or missing task folders. Repair reporting is
non-destructive by default and does not rewrite project, task, link, Markdown,
or user files.

## Task ID Rules

Task IDs are stable until the user explicitly renames them. Version 1 IDs must
be:

- human-readable names that fit within a single filesystem path component;
- unique within a project;
- non-empty after trimming surrounding whitespace;
- different from `.` or `..`;
- free of path separators and cross-platform invalid filename characters;
- different from reserved device names such as `con`, `prn`, `aux`, `nul`,
  `com1` through `com9`, and `lpt1` through `lpt9`.

Project-wide uniqueness is enforced when tasks are scanned, adopted, or created.
Single task metadata validation can only validate the ID shape.

Project event IDs use the same single-component naming rule as task IDs. Event
lists preserve order and reject duplicate IDs within the project metadata.

## Task Scanning

Project scanning discovers tasks by walking the project tree and finding
`.spielgantt/task.json` files. The indexed task record is the canonical task
`id` plus the task folder's current path. Ordinary user files inside a task
folder do not affect scanning.

Nested task buckets are supported in version 1. If both a parent folder and a
child folder contain `.spielgantt/task.json`, SpielGantt indexes both tasks and
reports each task at its actual current folder path.

## Golden Examples

The examples in `docs/metadata/examples/project.json` and
`docs/metadata/examples/task.json` are executable golden fixtures used by the
Rust metadata contract tests.
