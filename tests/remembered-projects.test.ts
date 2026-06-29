import assert from "node:assert/strict";
import test from "node:test";

import { Window } from "happy-dom";

import { InMemoryRememberedProjectsSettings } from "../src/remembered-projects.ts";
import type { OpenProjectResult } from "../src/main.ts";
import { mountTestShell, openProjectFromProjectsMenu } from "./support/frontend-shell.ts";

test("mountShell remembers an opened project through an injected in-memory settings implementation", async () => {
  const window = new Window();
  const root = window.document.createElement("div");
  const project: OpenProjectResult = {
    selectedPath: "/tmp/remembered-project",
    projectRoot: "/tmp/remembered-project",
    valid: true,
    issues: [],
    projectReadmeContent: "",
    projectReadmeVersion: "project-readme-version",
    events: [],
    eventReferences: [],
    workflow: null,
    tasks: [
      {
        id: "analysis-run",
        path: "/tmp/remembered-project/analysis-run",
        projectRelativePath: "analysis-run",
        dependencies: [],
        blocks: [],
        dependencyReferences: [],
        dependencyTargets: [],
        endsAt: null,
        status: null,
        readmeContent: "# analysis-run\n",
        readmeVersion: "task-readme-version",
      },
    ],
  };
  const settings = new InMemoryRememberedProjectsSettings();

  await mountTestShell(
    root,
    {
      projectLifecycle: {
        loadHealth: async () => ({
          appName: "spielgantt",
          version: "1.0.0-rc.1",
          core: "ready",
        }),
        openSelectedProject: async () => project,
        onboardProjectAction: async () => project,
        previewAlignmentAction: async () => ({
          operations: [],
          preflightIssues: [],
        })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => settings
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/remembered-project"
      },
      projectWatch: {
        subscribeProjectSessionChanges: async () => ({
          status: { watching: true, message: "Watching project folder" },
          unsubscribe: () => {},
        })
      }
    },
  );

  await openProjectFromProjectsMenu(root);

  const [record] = await settings.listProjects();
  assert.equal(record.projectPath, "/tmp/remembered-project");
  assert.equal(record.expanded, true);
  assert.equal(record.lastSelectedTaskName, null);
  assert.match(record.lastOpenedAt, /^\d{4}-\d{2}-\d{2}T/);
});
