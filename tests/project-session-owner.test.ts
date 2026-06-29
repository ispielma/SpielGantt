import assert from "node:assert/strict";
import test from "node:test";

import { createProjectSessionOwner } from "../src/shell-project-session.ts";
import {
  InMemoryRememberedProjectsSettings,
  projectFixture,
  taskFixture,
} from "./support/frontend-shell.ts";

test("project session owner opens, selects, marks missing, and removes remembered projects through snapshots", async () => {
  const settings = new InMemoryRememberedProjectsSettings();
  const projectPath = "/tmp/fixture-spielgantt/session-owner";
  const owner = createProjectSessionOwner({
    initialRememberedProjects: [],
    rememberedProjectsSettings: settings,
  });

  owner.commands.activateProject(
    projectFixture({
      selectedPath: projectPath,
      projectRoot: projectPath,
      tasks: [
        taskFixture("sample-prep", {
          path: `${projectPath}/sample-prep`,
        }),
        taskFixture("analysis", {
          path: `${projectPath}/analysis`,
        }),
      ],
    }),
    "analysis",
    true,
  );
  await owner.commands.flushPersistence();

  assert.equal(owner.snapshot.activeProjectRoot, projectPath);
  assert.equal(owner.snapshot.projectState.status, "valid");
  assert.equal(owner.snapshot.selection.selectedTaskId, "analysis");
  assert.deepEqual(
    owner.snapshot.sidebarState.projectContents[projectPath],
    {
      taskNames: ["sample-prep", "analysis"],
      eventNames: [],
    },
  );
  assert.deepEqual(
    (await settings.listProjects()).map((record) => ({
      projectPath: record.projectPath,
      expanded: record.expanded,
      lastSelectedTaskName: record.lastSelectedTaskName,
    })),
    [
      {
        projectPath,
        expanded: true,
        lastSelectedTaskName: "analysis",
      },
    ],
  );

  owner.commands.selectTask("sample-prep");
  await owner.commands.flushPersistence();
  assert.equal(owner.snapshot.selection.selectedTaskId, "sample-prep");
  assert.equal(owner.snapshot.selection.selectedEventId, null);
  assert.equal(
    (await settings.listProjects())[0]?.lastSelectedTaskName,
    "sample-prep",
  );

  owner.commands.markRememberedProjectMissing(projectPath, "folder is missing");
  assert.equal(owner.snapshot.sidebarState.scanFailures[projectPath], "folder is missing");

  await owner.commands.removeRememberedProject(projectPath);
  assert.equal(owner.snapshot.projectState.status, "none");
  assert.equal(owner.snapshot.activeProjectRoot, null);
  assert.deepEqual(owner.snapshot.rememberedProjects, []);
  assert.deepEqual(owner.snapshot.sidebarState.projectContents, {});
  assert.deepEqual(owner.snapshot.sidebarState.scanFailures, {});
  assert.deepEqual(await settings.listProjects(), []);
});
