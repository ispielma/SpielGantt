# Contributing to SpielGantt

Thanks for your interest in improving SpielGantt. Contributions can include bug
reports, feature ideas, documentation fixes, tests, examples, and code changes.

SpielGantt is a local-first Gantt tool for scientific workflows. Its durable
artifact is an ordinary project folder with JSON metadata, so contributions
should preserve that command-line-first, filesystem-first design.

## Before You Start

- Search existing issues and pull requests before opening a duplicate.
- Keep each issue or pull request focused on one problem or feature.
- For larger changes, open an issue first so the approach can be discussed
  before you spend time on implementation.
- Read `README.md` for the user-facing overview and
  `docs/development-guidance.md` for current architecture notes.

## Reporting Bugs

When reporting a bug, include enough detail for someone else to reproduce it:

- The SpielGantt version or commit.
- Your operating system.
- Whether the problem appears in the CLI, GUI, or both.
- The command, project folder shape, or GUI steps that triggered the problem.
- The expected result and the actual result.
- Any relevant terminal output, validation errors, or screenshots.

Please do not include private project data. A small synthetic project or edited
metadata snippet is usually enough.

## Suggesting Features

Feature requests are most useful when they describe the workflow need, not only
the proposed UI. Please include:

- The scientific workflow problem you are trying to solve.
- Whether the feature should exist in the CLI, GUI, or both.
- What project files or metadata would need to change.
- Any compatibility concerns for existing SpielGantt projects.

## Development Setup

SpielGantt is a Tauri app with a Rust CLI/core and a React/Mantine frontend.

Prerequisites:

- Node.js and npm.
- Rust and Cargo.
- Platform dependencies required by
  [Tauri v2](https://v2.tauri.app/start/prerequisites/).

Install dependencies:

```sh
npm install
```

Run the frontend development server:

```sh
npm run dev
```

Run the CLI from source:

```sh
cargo run --manifest-path src-tauri/Cargo.toml -- --help
```

Build frontend assets:

```sh
npm run build
```

Build and verify the macOS release candidate:

```sh
npm run release:build
```

## Project Design Expectations

SpielGantt's project format should stay simple, readable, and repairable:

- SpielGantt-owned metadata is JSON under `.spielgantt/`.
- User notes, data, scripts, and other files live outside `.spielgantt/` and
  must not be treated as application-owned data.
- CLI/core behavior owns package semantics such as validation, dependency
  rules, metadata mutation, and machine-readable JSON output.
- GUI code should present and adapt shared package behavior; it should not
  become the source of truth for project semantics.
- New structured metadata and machine-readable output should be JSON, not YAML.

See `docs/development-guidance.md` for more detail on the CLI/core and GUI
boundary.

## Tests

Use whatever development workflow works for you. The project does not require a
specific private process such as test-driven development. Pull requests should,
however, leave final behavior covered by tests through public interfaces when
the change affects behavior.

Useful checks:

```sh
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
npm run test:architecture
npm run test:release
```

Run the checks that match the files you changed. Broader changes should run the
broader suite.

Guidance by area:

- Rust/core or CLI behavior: add or update tests under `src-tauri/tests/`.
- CLI command behavior: use helpers from `src-tauri/tests/support/mod.rs`
  instead of duplicating command-running or JSON parsing helpers.
- Frontend behavior: add or update tests under `tests/`, preferably through
  accessible names and user-facing workflows.
- Architecture-sensitive changes: run `npm run test:architecture`.
- Release packaging changes: run `npm run test:release` and, for macOS release
  changes, `npm run release:build`.

Documentation-only changes usually do not need automated tests.

## Pull Requests

Before opening a pull request:

- Keep the diff focused on the stated issue or feature.
- Include tests or explain why tests are not applicable.
- Update documentation when behavior, commands, metadata, or user workflows
  change.
- Preserve existing user files and project data when changing package behavior.
- Avoid unrelated formatting or refactoring churn.
- Make sure generated build outputs, local caches, and personal development
  files are not committed.

By contributing, you agree that your contribution is licensed under the license
that applies to the files you change. Contributions under `src-tauri/` are
BSD-3-Clause; frontend, documentation, examples, and other contributions outside
`src-tauri/` are PolyForm-Noncommercial-1.0.0 unless a file states otherwise.

In the pull request description, summarize:

- What changed.
- Why it changed.
- Which checks you ran.
- Any risks, follow-up work, or unsupported platforms.

## Release Notes

Release expectations and known limitations are tracked in
`docs/release-candidate.md`. The currently verified production GUI package is
the macOS `.app` bundle; Windows and Linux can be built from source but are not
yet release-verified.
