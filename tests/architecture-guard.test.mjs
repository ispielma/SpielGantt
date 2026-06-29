import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { test } from "node:test";

import { checkArchitecture, checkArchitectureWarnings } from "../scripts/check-architecture.mjs";

test("architecture guard rejects TypeScript dependency target algorithms", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "src"), { recursive: true });
  await writeFile(
    join(root, "src", "package-semantics.ts"),
    `
export function validDependencyTargets(tasks, selectedTask) {
  return tasks.filter((task) => task.id !== selectedTask.id);
}
`,
  );

  const failures = await checkArchitecture(root);

  assert(
    failures.some(
      (failure) =>
        failure.includes("frontend dependency/event semantic algorithms must come from Rust contracts") &&
        failure.includes("validDependencyTargets"),
    ),
    `expected architecture guard to reject dependency target algorithms:\n${failures.join("\n")}`,
  );
});

test("architecture guard yellow-zone warnings identify new owner pressure relief", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "src"), { recursive: true });
  await writeFile(join(root, "src", "workspace-selection-model.ts"), "\n".repeat(240));

  const warnings = await checkArchitectureWarnings(root);

  assert(
    warnings.some(
      (warning) =>
        warning.includes("src/workspace-selection-model.ts") &&
        warning.includes("owner: workspace selection model") &&
        warning.includes("follow-up:"),
    ),
    `expected new owner yellow-zone warning to name owner and pressure relief:\n${warnings.join("\n")}`,
  );
});

test("architecture guard rejects Rust app payload visual layout fields", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "src-tauri", "src"), { recursive: true });
  await writeFile(
    join(root, "src-tauri", "src", "project_payload.rs"),
    `
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectTask {
    id: String,
    grid_column: String,
}
`,
  );

  const failures = await checkArchitecture(root);

  assert(
    failures.some(
      (failure) =>
        failure.includes("Rust app payloads must not expose visual layout fields") &&
        failure.includes("grid_column"),
    ),
    `expected architecture guard to reject Rust app payload visual fields:\n${failures.join("\n")}`,
  );
});

test("architecture guard rejects removed frontend shell compatibility dependency shims", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "tests", "support"), { recursive: true });
  await writeFile(
    join(root, "tests", "support", "frontend-shell-deps.ts"),
    `
export type LegacyShellDeps = {
  openProject: () => Promise<void>;
};

export function mapLegacyShellDeps(deps: LegacyShellDeps) {
  return {
    projectSession: {
      openProject: deps.openProject,
    },
  };
}
`,
  );

  const failures = await checkArchitecture(root);

  assert(
    failures.some(
      (failure) =>
        failure.includes("trivial compatibility shims must stay removed") &&
        failure.includes("LegacyShellDeps"),
    ),
    `expected architecture guard to reject legacy shell dependency shims:\n${failures.join("\n")}`,
  );
});

test("architecture guard rejects removed adapter aliases and zero-state facade wrappers", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "src"), { recursive: true });
  await mkdir(join(root, "src-tauri", "src"), { recursive: true });
  await writeFile(
    join(root, "src", "project-watch.ts"),
    `
export function watchProjectChanges() {
  return undefined;
}
`,
  );
  await writeFile(
    join(root, "src", "legacy-shared-import.ts"),
    `
import { invoke } from "shared";
export { invoke };
`,
  );
  await writeFile(
    join(root, "src-tauri", "src", "lib.rs"),
    `
pub mod app_facade;
pub use app_facade as app_api;
`,
  );
  await writeFile(
    join(root, "src-tauri", "src", "app_facade.rs"),
    `
pub struct AppFacade;

impl AppFacade {
    pub fn open_project() {}
}
`,
  );

  const failures = await checkArchitecture(root);

  for (const legacyName of ["watchProjectChanges", "shared", "app_api", "AppFacade"]) {
    assert(
      failures.some(
        (failure) =>
          failure.includes("trivial compatibility shims must stay removed") &&
          failure.includes(legacyName),
      ),
      `expected architecture guard to reject ${legacyName} shim:\n${failures.join("\n")}`,
    );
  }
});

test("architecture guard allows real adapter boundaries and durable compatibility aliases", async () => {
  const root = await mkdtemp(join(tmpdir(), "spielgantt-architecture-"));
  await mkdir(join(root, "src"), { recursive: true });
  await mkdir(join(root, "src-tauri", "src"), { recursive: true });
  await writeFile(
    join(root, "src", "tauri-api.ts"),
    `
import { invoke } from "@tauri-apps/api/core";

export function openProject(path: string) {
  return invoke("spielgantt_open_project", { path });
}
`,
  );
  await writeFile(
    join(root, "src-tauri", "src", "tauri_commands.rs"),
    `
use crate::app_facade;

#[tauri::command]
fn spielgantt_open_project(path: String) -> Result<app_facade::OpenProjectResult, String> {
    app_facade::open_project(&path).map_err(|error| error.to_string())
}
`,
  );
  await writeFile(
    join(root, "src-tauri", "src", "task.rs"),
    `
#[derive(serde::Deserialize)]
struct TaskMetadata {
    #[serde(alias = "planned", alias = "in_progress")]
    status: String,
}
`,
  );
  await writeFile(
    join(root, "src-tauri", "src", "lib.rs"),
    `
pub mod app_facade;
pub mod task;
pub use task::TaskMetadata;
`,
  );

  const failures = await checkArchitecture(root);
  const shimFailures = failures.filter((failure) =>
    failure.includes("trivial compatibility shims must stay removed"),
  );

  assert.deepEqual(
    shimFailures,
    [],
    `expected architecture guard to allow real adapter boundaries:\n${failures.join("\n")}`,
  );
});
