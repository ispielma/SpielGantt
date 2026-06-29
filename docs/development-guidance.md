# Development Guidance

This document holds durable project guidance that should outlive any one
implementation issue plan. `AGENTS.md` is the authoritative project-level
vision and boundary contract; this guide records the development consequences
of that contract.

## Product Goals

- Keep scientific workflow projects as ordinary folders that remain useful
  without SpielGantt.
- Store SpielGantt-owned metadata under hidden `.spielgantt/` directories.
- Treat task IDs as user-readable canonical identity. Current GUI work is
  moving IDs from slug-only strings to human task names that are valid single
  filesystem path components.
- For the GUI, keep the project shape strict: direct project child folders are
  task buckets, and task folder basenames align with task IDs/names.
- Keep user files outside `.spielgantt/` untouched.
- Follow the CLI-first boundary in `AGENTS.md`: Rust/core owns
  CLI-equivalent package behavior, while the frontend owns GUI/session and
  desktop-only workflows that are not package operations.

## CLI-Domain Boundary Contract

Apply the contract in `AGENTS.md` this way:

- Package-semantic GUI behavior must have CLI parity through shared Rust
  behavior and versioned JSON contracts when machine-readable output matters.
- Purely visual presentation, GUI session state, and desktop-only workflows do
  not require CLI or Rust/core package contracts.
- Rust must expose domain contracts, not GUI-specific view models.
- Frontend code must not derive dependency validity, legal dependency choices,
  event references, validation results, file-format behavior, or package
  mutation semantics.
- Frontend code may own visual timeline rendering, row/lane placement,
  connector drawing, remembered-project UI state, and other GUI/session
  behavior.
- As a rule of thumb, the frontend should be rewritable to use only CLI JSON
  calls plus local rendering and GUI/session code.

## Workflow

- Use vertical TDD: write one behavior test through a public interface, make it
  pass, then refactor while green.
- Prefer CLI/core behavior tests before GUI tests when behavior is not visual.
- For CLI behavior tests, use the shared helpers in
  `src-tauri/tests/support/mod.rs`.
- Favor the best local design over the smallest expedient patch. If a bug
  exposes a weak model or leaky abstraction inside the current issue boundary,
  repair the model rather than adding a special case.
- Keep design improvements scoped to the behavior being changed and avoid
  speculative architecture outside the current issue.
- Commit completed work after tests pass.

## Architecture Guard

- Run `npm run test:architecture` after cleanup work that touches the frontend
  shell, Rust command module, or frontend shell tests.
- `npm run release:verify` also runs the architecture guard before checking the
  bundled app artifact.
- The guard enforces line-count budgets for the files that were split during
  cleanup and rejects reintroduced TypeScript package-semantic algorithms.
  Visual chart layout remains a frontend concern.
- Files at or above 80% of their configured line budget are in the
  architecture yellow zone. Treat them as refactor candidates before adding
  substantial behavior, even when the hard budget still passes.
- Yellow-zone guard output should name the responsible owner area and the next
  follow-up direction so pressure does not accumulate silently.
- Do not satisfy line budgets by compressing code, hiding logic in dense
  expressions, or moving behavior into unrelated modules. Prefer a focused
  owner module with a small interface and behavior tests through the public
  contract.
- When a yellow-zone file must stay large temporarily, document the owner and
  the follow-up pressure-relief slice before committing more behavior there.

## GUI Expectations

- The active frontend stack is React + Mantine inside the standard Tauri/Vite
  package; keep that package shape intact instead of creating a parallel
  frontend application.
- Use Mantine AppShell for the main desktop shell and prefer Mantine
  components for menus, dialogs, forms, and other interactive controls when
  they fit the workflow.
- Every interactive GUI control must have a stable accessible name through
  native labels, visible text, `aria-label`, or equivalent platform semantics.
- New GUI behavior should be tested through accessible names where practical.
- GUI slices should generate a working production Tauri app bundle that can be
  tested without a development server.
- For macOS, verify the generated app bundle with `npm run release:verify`.
- When Codex's sandbox cannot launch the built macOS `.app` directly, launch
  the bundle through Finder/LaunchServices instead of executing the app binary
  from the shell:
  `osascript -e 'tell application "Finder" to open POSIX file "/absolute/path/to/SpielGantt.app"'`.
  Then attach Computer Use to the running app for direct GUI stress testing.

## Package Decisions

- Favor established packages, crates, and Tauri/frontend libraries over custom
  implementations for CLI parsing, serialization, validation, date handling,
  dependency analysis, filesystem watching, testing, UI controls, and timeline
  interactions.
- Add custom code only when no suitable package exists, integration would be
  more complex than the behavior itself, or a small domain-specific layer is
  needed around a package.
- Avoid deprecated or unmaintained packages.
- JSON is the only structured data format for SpielGantt metadata, disposable
  cache files, Tauri payloads, machine-readable CLI output, and test fixtures;
  new `.spielgantt` files must be JSON, not YAML.
- Expected metadata replacements are `project.yaml` to `project.json`,
  `task.yaml` to `task.json`, and disposable cache YAML to JSON. SpielGantt does not maintain task-link metadata; stale `links.json` files are not part of the
  current package contract.
- Human CLI output may remain plain text, but structured CLI output must be JSON.
- Do not use `serde-saphyr` for new metadata work. If a YAML reader remains
  temporarily, it must be documented as migration-only compatibility and must
  not write YAML.

## Model Guidance

- `5.4-mini`: very easy, low-risk documentation, scaffold, or smoke-test work.
- `GPT-5.4`: focused implementation work with clear acceptance criteria.
- `GPT-5.4 + review`: implementation that needs stronger follow-up review.
- Stronger model: subtle architecture, cross-platform behavior, or dense UI
  interaction.
