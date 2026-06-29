import assert from "node:assert/strict";
import test from "node:test";

import {
  projectActionEntriesForProject,
  type ProjectActionEntry,
} from "../src/project-actions.ts";

function actionSummaries(entries: ProjectActionEntry[]) {
  return entries
    .filter((entry) => entry.kind === "action")
    .map((entry) => ({
      id: entry.id,
      label: entry.label,
      disabled: entry.disabled,
      command: entry.command,
    }));
}

test("project action model owns labels, missing-project availability, disabled state, and command mapping", () => {
  assert.deepEqual(
    actionSummaries(
      projectActionEntriesForProject({
        projectPath: "/tmp/fixture-spielgantt/workflow",
        projectName: "workflow",
        missing: false,
        refreshing: false,
      }),
    ),
    [
      {
        id: "refresh-project",
        label: "Refresh Project",
        disabled: false,
        command: { kind: "refresh-project" },
      },
      {
        id: "open-project-folder",
        label: "Reveal in Finder",
        disabled: false,
        command: { kind: "reveal-project-folder" },
      },
      {
        id: "create-task",
        label: "Create Task",
        disabled: false,
        command: { kind: "open-create-task-dialog" },
      },
      {
        id: "create-task-from-folder",
        label: "Create Task from Folder...",
        disabled: false,
        command: { kind: "open-create-task-from-folder-workflow" },
      },
      {
        id: "create-event",
        label: "Create Event",
        disabled: false,
        command: { kind: "open-create-event-dialog" },
      },
      {
        id: "delete-project",
        label: "Delete Project...",
        disabled: false,
        command: { kind: "open-delete-project-dialog" },
      },
      {
        id: "remove-project",
        label: "Remove from Sidebar",
        disabled: false,
        command: { kind: "remove-remembered-project" },
      },
    ],
  );

  assert.deepEqual(
    actionSummaries(
      projectActionEntriesForProject({
        projectPath: "/tmp/fixture-spielgantt/Missing Project",
        projectName: "Missing Project",
        missing: true,
        refreshing: false,
      }),
    ),
    [
      {
        id: "find-project",
        label: "Find Project...",
        disabled: false,
        command: { kind: "find-remembered-project" },
      },
      {
        id: "remove-project",
        label: "Remove from Sidebar",
        disabled: false,
        command: { kind: "remove-remembered-project" },
      },
    ],
  );

  assert.equal(
    actionSummaries(
      projectActionEntriesForProject({
        projectPath: "/tmp/fixture-spielgantt/workflow",
        projectName: "workflow",
        missing: false,
        refreshing: true,
      }),
    ).find((entry) => entry.id === "refresh-project")?.disabled,
    true,
  );
});
