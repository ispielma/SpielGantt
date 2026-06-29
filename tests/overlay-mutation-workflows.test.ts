import assert from "node:assert/strict";
import test from "node:test";

import { createTaskRenameWorkflow } from "../src/task-overlay-workflows.ts";
import { type OpenProjectResult } from "../src/shell-types.ts";
import { projectFixture, taskFixture } from "./support/frontend-shell.ts";

function workflowProject(taskIds: string[] = ["sample-prep"]): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    tasks: taskIds.map((taskId) =>
      taskFixture(taskId, { path: `/tmp/fixture-spielgantt/workflow/${taskId}` }),
    ),
  });
}

test("task rename workflow refreshes remembered project contents when the project is not active", async () => {
  let state = {
    open: true,
    projectPath: "/tmp/fixture-spielgantt/remembered",
    taskId: "sample-prep",
    taskName: "sample-prep",
    errorMessage: null,
    submitting: false,
  };
  const rememberedRefreshes: string[] = [];
  const activeRefreshes: OpenProjectResult[] = [];

  const workflow = createTaskRenameWorkflow({
    getState: () => state,
    setState: (nextState) => {
      state = nextState;
    },
    close: () => {
      state = {
        open: false,
        projectPath: "",
        taskId: "",
        taskName: "",
        errorMessage: null,
        submitting: false,
      };
    },
    closeTaskAndEventMenus: () => {},
    currentProjectRoot: () => "/tmp/fixture-spielgantt/active",
    renameTaskAction: async () => ({ project: workflowProject(["Renamed sample"]) }),
    refreshProjectState: (project) => {
      activeRefreshes.push(project);
    },
    refreshRememberedProjectTasks: async (projectPath) => {
      rememberedRefreshes.push(projectPath);
    },
    setActionError: () => {
      throw new Error("unexpected action error");
    },
    setOperationIdle: () => {},
    render: () => {},
  });

  await workflow.submit("Renamed sample");

  assert.equal(state.open, false);
  assert.deepEqual(activeRefreshes, []);
  assert.deepEqual(rememberedRefreshes, ["/tmp/fixture-spielgantt/remembered"]);
});
