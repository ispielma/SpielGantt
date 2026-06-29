# SpielGantt Release Candidate Notes

These notes capture the current release-candidate status. They are
checked by `npm run release:verify` so known limitations stay explicit instead
of living only in agent handoff text.

## Verification Checklist

- Fresh checkout setup is documented in `README.md`: install npm dependencies,
  run frontend and release verifier tests, run Rust tests, build frontend
  assets, build the release CLI, and build the Tauri app bundle.
- CLI and core behavior are covered by `cargo test` from `src-tauri/`.
- GUI shell behavior, accessible control names, and event-axis rendering are
  covered by `npm run test:frontend`.
- The checked-in example project is validated and opened through the shared GUI
  read model by `src-tauri/tests/documentation_examples.rs`.
- The macOS `.app` bundle is produced by `npm run tauri -- build --bundles app`,
  packaged into a DMG by `npm run release:dmg`, and checked by
  `npm run release:verify`.
- Agent readiness is verified from the packaged macOS `.app` itself:
  `npm run release:verify` runs `Contents/MacOS/spielgantt agent runtime --json`,
  confirms the package context resolves to that `.app` path, initializes a
  temporary project, runs `agent prepare --json`, and checks the generated
  `AGENTS.md`, project-local skills, and `.spielgantt/agent.json` metadata.
  This check is performed from the build output path, not `/Applications`.

## GitHub Release Automation

- The repository declares a split source license: files under `src-tauri/` are
  BSD-3-Clause, while the frontend, examples, documentation, and other
  repository contents outside `src-tauri/` remain
  PolyForm-Noncommercial-1.0.0 unless a file states otherwise.
- CI runs the MVP release gate on macOS from a clean checkout for pull requests
  and pushes to `main`.
- Tagged pushes matching `v*` build and verify `SpielGantt.app`, package it as
  `SpielGantt_*.dmg`, upload the DMG as a workflow artifact, and attach it to
  the GitHub release.
- The release workflow uses the built-in `GITHUB_TOKEN`; no custom secrets are
  required for the MVP. Repository workflow permissions must allow
  `contents: write` so the release asset can be uploaded.

## Release Smoke Coverage

Slice 22 release-candidate smoke coverage combines release-binary checks with
repeatable GUI shell tests:

- Create a project and create tasks: verified with the release `spielgantt`
  binary in a temporary project directory.
- Edit metadata and README: GUI shell tests cover `Save Task` through the
  shared edit action; the release-binary smoke also updated metadata and a task
  `README.md`.
- Add a dependency: verified by release-binary smoke and GUI/core tests through
  shared behavior.
- Normalize folders: verified by release-binary smoke and GUI shell tests for
  preview and apply flows.
- Inspect the event-axis timeline: frontend shell tests cover event rails,
  connector cues, and task selection from timeline bars.
- Open the example in the GUI: documentation example tests open
  `examples/fluorescence-timecourse` through the shared GUI read model.

## Known Limitations

- The macOS DMG is the only production GUI installer target verified in this
  release candidate. Windows and Linux packaging targets are documented but not
  yet configured or smoke-tested.
- Native context menus are not yet part of the packaged release surface; the
  project and task actions stay in the sidebar webview for now.
- The macOS bundle is not configured for Apple Developer ID signing or
  notarization, so first launch may require local Gatekeeper approval.
- Windows and Linux package agent-readiness behavior is documented in the
  runtime package context contract and covered by path-resolution unit tests,
  but this release verifier directly executes only the macOS `.app` bundle.
- Timeline event selection is covered by frontend shell tests, not by an
  end-to-end native desktop automation run against the packaged app.
- The cache remains an implementation detail for validation and repair flows;
  SQLite-backed cache behavior is not a durable project artifact.
