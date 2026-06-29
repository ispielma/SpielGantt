# Agent Instructions

SpielGantt is a CLI-first, local-first Gantt tool for scientific workflows. The
durable artifact is an ordinary project folder that remains useful without the
GUI.

## Project Vision and Boundary Contract

SpielGantt is a command-line tool first, like `git`. The Rust backend is the
package-domain library for that CLI: it provides the capabilities needed by a
CLI user or automation agent to create, read, understand, mutate, validate, and
describe SpielGantt packages. Backend package capabilities must be expressible
through CLI commands and, where machine-readable behavior is needed, versioned
JSON contracts.

The GUI is a user-friendly interface over some or all CLI-equivalent package
capabilities. It may call the Rust library directly instead of spawning CLI
processes, but those backend calls must still correspond to CLI-expressible
package behavior. Tauri commands are adapters over those shared Rust package
capabilities, not a separate GUI-only domain model.

User-interface, session, and desktop workflows that are not package operations
belong to the frontend or desktop shell, not Rust/core or the CLI. Examples
include visual timeline layout, row/lane placement, connector drawing,
transient interaction state, remembered-project sidebar state, project file
watching, and whole-project deletion. A CLI user can delete a project folder
with ordinary filesystem tools such as `rm -rf`; therefore a GUI **Delete
Project...** workflow must stay GUI-owned and must not introduce a Rust/core or
CLI project-delete command.

The frontend must never become the source of truth for package semantics.
Dependency validity, legal dependency choices, event references, validation
results, file-format behavior, and package mutation rules belong in Rust/core
and must be exposed through CLI-equivalent contracts. As a rule of thumb, the
frontend should be rewritable to use only CLI JSON calls plus local rendering
and GUI/session code.

Core design:

- Project root: any folder containing SpielGantt project metadata under
  `.spielgantt/`.
- Task bucket: an ordinary user folder containing SpielGantt task metadata
  under `.spielgantt/`.
- User files live outside `.spielgantt/` and must not be treated as owned by SpielGantt.
- Task identity is the user-readable `id`; there is no separate required title.
- Current work is moving task IDs from slug-only strings to human names that are valid single filesystem path components.
- The GUI project model is stricter than the CLI/core model: direct project child folders are task buckets, and the GUI should align task folder basenames with task IDs/names.
- Structured metadata is canonical; OS shortcuts/symlinks may exist but are not the source of truth.

Expected stack:

- Idiomatic Tauri app scaffolded with `create-tauri-app`, preferably via `sh <(curl https://create.tauri.app/sh)`.
- Shared Rust behavior for package parsing, validation, mutation, links, dependencies, and CLI-relevant filesystem behavior.
- Rust CLI entrypoints contained inside the standard Tauri package pattern.
- Tauri GUI with the selected frontend template.
- Filesystem-first JSON/Markdown; SQLite only as disposable cache if needed.

Package preference:

- Favor established packages, crates, and Tauri/frontend libraries over custom implementations for CLI parsing, serialization, validation, date handling, dependency analysis, filesystem watching, testing, UI controls, and timeline interactions.
- Add custom code only when no suitable package exists, integration would be more complex than the behavior itself, or the slice needs a small domain-specific layer around a package.
- When adding a dependency, prefer well-maintained packages with clear docs, active releases, and idiomatic use in the relevant ecosystem.
- Avoid deprecated or unmaintained packages. If one is already present, migrate before building more behavior on top of it or document why the temporary risk is acceptable.
- SpielGantt's target structured-data format is JSON for metadata, cache files,
  and machine-readable CLI contracts. Do not add new YAML metadata behavior.
- JSON is the only structured data format for SpielGantt-owned metadata,
  disposable caches, Tauri payloads, machine-readable CLI output, and test
  fixtures; new `.spielgantt` files must be JSON, not YAML.
- Expected metadata replacements are `project.yaml` to `project.json`,
  `task.yaml` to `task.json`, and disposable cache YAML to JSON. SpielGantt does not maintain task-link metadata; stale `links.json` files are not part of
  the current package contract.
- Human CLI output may remain plain text, but structured CLI output must be JSON.
  Do not use `serde-saphyr` for new metadata work; any remaining YAML
  reader must be treated as temporary migration-only code and must not write
  YAML.

Workflow:

1. Read `docs/development-guidance.md` before working.
2. Work on one slice unless the user explicitly asks otherwise.
3. Use TDD: write one behavior test through a public interface, make it pass, then continue.
4. Prefer CLI/core behavior tests before GUI tests when behavior is not visual.
5. For CLI behavior tests, use the shared helpers in `src-tauri/tests/support/mod.rs`; do not duplicate local process-running or stdout/stderr helpers in individual test files.
6. Favor the best local design over the smallest expedient patch. If a bug reveals a weak model or leaky abstraction inside the slice boundary, repair the model rather than layering a special case on top of it.
7. Keep design improvements scoped to the behavior being changed; do not make speculative architecture or broad unrelated refactors.
8. Preserve user files and unrelated worktree changes.
9. If a current `ISSUES.md` plan exists, update its checklist when a slice is complete.
10. Commit the completed slice after tests pass and the checklist is updated.
11. End with the commit hash, what changed, tests run, and any remaining risks or follow-up slices.

GUI stress-test note:

- The built macOS `.app` may fail to launch when started from Codex's sandboxed
  shell, including LaunchServices errors such as `kLSNoExecutableErr` or an
  immediate SIGABRT in AppKit/HIServices registration. Treat this as a known
  sandbox-start limitation, not as an in-app SpielGantt behavior failure.
- For direct GUI stress tests, prefer attaching Computer Use to an app instance
  that the user has launched normally from Finder/Dock/Spotlight. If no
  instance is running, ask the user to launch the built app manually before
  continuing GUI-only testing.

Slice completion:

- A slice is not complete until its changes are committed.
- Stage only files that belong to the slice.
- Use a commit message like `Complete slice N: <slice title>`.
- If no `ISSUES.md` plan exists for the work, use a concise descriptive commit message.
- If unrelated worktree changes exist, leave them unstaged and mention them in the final report.
- If the slice cannot be committed, do not mark it complete; explain the blocker instead.

Useful skills:

- `tdd`: default for implementation slices.
- `to-issues`: update or restructure the slice plan.
- `improve-codebase-architecture`: architecture checkpoints and cleanup passes.
- `grill-me`: high-impact design decisions.
- `browser:control-in-app-browser`: GUI inspection and local frontend verification.
- `find-skills`: look for additional skills when a specialized need appears.
- `skill-creator`: create a project-specific skill if repeated SpielGantt workflows emerge.

When in doubt, keep the project format simple, readable, repairable, and useful without SpielGantt.

When choosing between an expedient local workaround and a cleaner invariant, choose the cleaner invariant. The project values durable, understandable design over short-term patch minimalism.
