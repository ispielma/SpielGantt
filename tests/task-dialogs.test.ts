import assert from "node:assert/strict";
import test from "node:test";

import type { Window } from "happy-dom";

import {
  clickTreeItem,
  deferred,
  getByRole,
  InMemoryRememberedProjectsSettings,
  openProjectFromProjectsMenu,
  projectFixture,
  rememberedProjectRecord,
  taskFixture,
  waitForHandlers,
  withRenderedShell,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
  type OpenProjectResult,
} from "./support/frontend-shell.ts";

function queryByRole(root: Element, role: Parameters<typeof getByRole>[1], name: RegExp) {
  try {
    return getByRole(root, role, name);
  } catch {
    return null;
  }
}

function workflowProject(taskIds: string[] = ["sample-prep"]): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    tasks: taskIds.map((taskId) =>
      taskFixture(taskId, { path: `/tmp/fixture-spielgantt/workflow/${taskId}` }),
    ),
  });
}

function workflowProjectWithEvents(
  taskIds: string[] = ["sample-prep"],
  eventIds: string[] = ["Samples ready"],
): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: eventIds,
    tasks: taskIds.map((taskId) =>
      taskFixture(taskId, { path: `/tmp/fixture-spielgantt/workflow/${taskId}` }),
    ),
  });
}

function workflowProjectWithReferencedEvent(
  eventId = "Samples ready",
  taskId = "analysis-run",
): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: [eventId, "Analysis complete"],
    eventReferences: [{ id: eventId, referencedTaskIds: [taskId] }],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: [eventId, "Analysis complete"],
      event_nodes: workflowEventNodes([eventId, "Analysis complete"]),
      tasks: [
        workflowTaskFixture(taskId, {
          dependency_references: [{ id: eventId, kind: "event", valid: true }],
          valid_dependency_targets: [eventId, "Analysis complete"],
        }),
      ],
    }),
    tasks: [
      taskFixture("sample-prep", { path: "/tmp/fixture-spielgantt/workflow/sample-prep" }),
      taskFixture(taskId, {
        path: `/tmp/fixture-spielgantt/workflow/${taskId}`,
        dependencies: [eventId],
      }),
    ],
  });
}

function workflowProjectWithTimelineTask(): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["Samples ready", "Analysis complete"],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["Samples ready", "Analysis complete"],
      event_nodes: workflowEventNodes(["Samples ready", "Analysis complete"]),
      tasks: [
        workflowTaskFixture("analysis-run", {
          dependency_references: [{ id: "Samples ready", kind: "event", valid: true }],
          ends_at_reference: { id: "Analysis complete", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Samples ready", "Analysis complete"),
          valid_dependency_targets: ["Samples ready", "Analysis complete"],
          valid_ends_at_targets: ["Samples ready", "Analysis complete"],
        }),
      ],
    }),
    tasks: [
      taskFixture("sample-prep", { path: "/tmp/fixture-spielgantt/workflow/sample-prep" }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        dependencies: ["Samples ready"],
        endsAt: "Analysis complete",
      }),
    ],
  });
}

function splitWorkflowProject(): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["Samples ready", "Analysis complete"],
    tasks: [
      taskFixture("prepare-inputs", { path: "/tmp/fixture-spielgantt/workflow/prepare-inputs" }),
      taskFixture("calibrate-laser", { path: "/tmp/fixture-spielgantt/workflow/calibrate-laser" }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        dependencies: ["prepare-inputs", "calibrate-laser"],
        blocks: [{ id: "write-report", kind: "task" }],
        endsAt: "Analysis complete",
      }),
      taskFixture("write-report", {
        path: "/tmp/fixture-spielgantt/workflow/write-report",
        dependencies: ["analysis-run"],
      }),
    ],
  });
}

async function openCreateTaskDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Create task in workflow$/i).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

async function openRenameTaskDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
    new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Rename task sample-prep$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

async function openDeleteTaskDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
    new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Delete task sample-prep$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

async function openCreateEventDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Create event in workflow$/i).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

async function openRenameEventDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "treeitem", /^Samples ready$/).dispatchEvent(
    new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Rename event Samples ready$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

async function openDeleteEventDialog(root: HTMLElement, window: Window): Promise<void> {
  getByRole(root, "treeitem", /^Samples ready$/).dispatchEvent(
    new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
  );
  await waitForHandlers();

  getByRole(root, "menuitem", /^Delete event Samples ready$/).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true }),
  );
  await waitForHandlers();
}

function submitDialog(root: HTMLElement, window: Window, buttonName: RegExp): void {
  getByRole(root, "button", buttonName).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true, cancelable: true }),
  );
}

function assertActiveElement(
  root: HTMLElement,
  expected: HTMLElement,
  message: string,
): void {
  assert.ok(root.ownerDocument.activeElement === expected, message);
}

test("creating an event shows a submitting state and selects the created event after success", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(
      "/tmp/fixture-spielgantt/workflow",
      ["sample-prep"],
      null,
      true,
      ["Samples ready"],
    ),
  ]);
  const createEventRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProjectWithEvents(["sample-prep"], ["Samples ready"]),
        onboardProjectAction: async () => workflowProjectWithEvents(["sample-prep"], ["Samples ready"])
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      eventMutation: {
        createEventAction: async () => createEventRequest.promise
      }
    },
    async ({ root, window }) => {
      await openCreateEventDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "textbox", /^Event name$/),
        "new event dialog should focus the event name input",
      );

      const eventNameInput = getByRole(root, "textbox", /^Event name$/) as HTMLInputElement;
      eventNameInput.value = "Data archived";
      eventNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Event$/);
      await waitForHandlers();

      const submitButton = getByRole(root, "button", /^Creating Event\.\.\.$/) as HTMLButtonElement;
      assert.equal(submitButton.disabled, true);

      createEventRequest.resolve({
        project: workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Data archived"]),
      });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Data archived$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("renaming an event shows a submitting state and selects the renamed event after success", async () => {
  const renameEventRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"])
      },
      eventMutation: {
        renameEventAction: async () => renameEventRequest.promise
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openRenameEventDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "textbox", /^Event name$/),
        "rename event dialog should focus the event name input",
      );

      const eventNameInput = getByRole(root, "textbox", /^Event name$/) as HTMLInputElement;
      eventNameInput.value = "Samples verified";
      eventNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Rename Event$/);
      await waitForHandlers();

      const submitButton = getByRole(root, "button", /^Renaming Event\.\.\.$/) as HTMLButtonElement;
      assert.equal(submitButton.disabled, true);

      renameEventRequest.resolve({
        project: workflowProjectWithEvents(["sample-prep"], ["Samples verified", "Analysis complete"]),
      });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Samples verified$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("deleting a referenced event shows the blocking tasks and omits confirmation", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProjectWithReferencedEvent(),
        onboardProjectAction: async () => workflowProjectWithReferencedEvent()
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openDeleteEventDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "button", /^Close$/),
        "blocked delete event dialog should focus the close button",
      );

      assert.match(root.textContent ?? "", /Cannot delete event 'Samples ready'/);
      assert.match(root.textContent ?? "", /Referencing tasks/);
      assert.match(root.textContent ?? "", /analysis-run/);
      assert.equal(queryByRole(root, "button", /^Delete Event$/), null);
      assert.ok(getByRole(root, "button", /^Close$/));
    },
  );
});

test("cancelling event delete closes the dialog without changing the selected event", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"])
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await openDeleteEventDialog(root, window);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Delete Event$/), null);
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("deleting an event shows a submitting state and clears the selected event after success", async () => {
  const deleteEventRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"])
      },
      eventMutation: {
        deleteEventAction: async () => deleteEventRequest.promise
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await openDeleteEventDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "button", /^Delete Event$/),
        "delete event dialog should focus the destructive confirmation button",
      );

      submitDialog(root, window, /^Delete Event$/);
      await waitForHandlers();

      const submitButton = getByRole(root, "button", /^Deleting Event\.\.\.$/) as HTMLButtonElement;
      assert.equal(submitButton.disabled, true);

      deleteEventRequest.resolve({
        project: workflowProjectWithEvents(["sample-prep"], ["Analysis complete"]),
      });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Delete Event$/), null);
      assert.doesNotMatch(root.textContent ?? "", /Samples ready/);
      assert.equal(
        getByRole(root, "treeitem", /^Analysis complete$/).getAttribute("aria-selected"),
        "false",
      );
    },
  );
});

test("event delete backend errors stay visible and recoverable", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"])
      },
      eventMutation: {
        deleteEventAction: async () => {
          throw new Error("Event delete permission denied.");
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await openDeleteEventDialog(root, window);

      submitDialog(root, window, /^Delete Event$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.match(
        getByRole(root, "alert", /Event delete permission denied\./).textContent ?? "",
        /Event delete permission denied\./,
      );
      assert.equal((getByRole(root, "button", /^Delete Event$/) as HTMLButtonElement).disabled, false);
      assert.ok(getByRole(root, "button", /^Cancel$/));
    },
  );
});

test("cancelling event rename closes the dialog without changing the selected event", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready", "Analysis complete"])
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await openRenameEventDialog(root, window);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Rename Event$/), null);
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("creating a task shows a submitting state and selects the created task after success", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"], null, true),
  ]);
  const createTaskRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep"]),
        onboardProjectAction: async () => workflowProject(["sample-prep"])
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      taskMutation: {
        createTaskAction: async () => createTaskRequest.promise
      }
    },
    async ({ root, window }) => {
      await openCreateTaskDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "textbox", /^Task name$/),
        "new task dialog should focus the task name input",
      );

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Data review";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();

      const submitButton = getByRole(root, "button", /^Creating Task\.\.\.$/) as HTMLButtonElement;
      assert.equal(submitButton.disabled, true);

      createTaskRequest.resolve({
        project: workflowProject(["sample-prep", "Data review"]),
      });
      await waitForHandlers();
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Data review$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("creating a task after selecting an event clears the event selection", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"], null, true),
  ]);
  const createTaskRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready"]),
        onboardProjectAction: async () =>
          workflowProjectWithEvents(["sample-prep"], ["Samples ready"])
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      taskMutation: {
        createTaskAction: async () => createTaskRequest.promise
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );

      await openCreateTaskDialog(root, window);
      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Prepare samples";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();

      createTaskRequest.resolve({
        project: workflowProjectWithEvents(
          ["sample-prep", "Prepare samples"],
          ["Samples ready"],
        ),
      });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Prepare samples$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.ok(getByRole(root, "textbox", /^Task README$/));
    },
  );
});

test("task context menus open add-before and add-after task dialogs from sidebar and timeline", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProjectWithTimelineTask(),
        onboardProjectAction: async () => workflowProjectWithTimelineTask()
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Add before sample-prep$/));
      assert.ok(getByRole(root, "menuitem", /^Add after sample-prep$/));

      getByRole(root, "menuitem", /^Add before sample-prep$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.match(root.textContent ?? "", /Add task before sample-prep/);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "button", /^Task analysis-run spans/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 120, clientY: 48 }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Add before analysis-run$/));
      assert.ok(getByRole(root, "menuitem", /^Add after analysis-run$/));

      getByRole(root, "menuitem", /^Add after analysis-run$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.match(root.textContent ?? "", /Add task after analysis-run/);
      assert.ok(getByRole(root, "textbox", /^Task name$/));
    },
  );
});

test("adding a task before submits one backend relative insert operation", async () => {
  const operations: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => splitWorkflowProject(),
        onboardProjectAction: async () => splitWorkflowProject()
      },
      taskMutation: {
        insertTaskRelativeAction: async (projectRoot, mode, selectedTaskId, insertedTaskId) => {
          operations.push(`insert:${projectRoot}:${mode}:${selectedTaskId}:${insertedTaskId}`);
          return {
            project: projectFixture({
              ...splitWorkflowProject(),
              tasks: [
                ...splitWorkflowProject().tasks,
                taskFixture(insertedTaskId, { path: `${projectRoot}/${insertedTaskId}` }),
              ],
            }),
          };
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      getByRole(root, "treeitem", /^analysis-run$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Add before analysis-run$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "quality-gate";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(operations, [
        "insert:/tmp/fixture-spielgantt/workflow:before:analysis-run:quality-gate",
      ]);
      assert.equal(
        getByRole(root, "treeitem", /^quality-gate$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("adding a task after submits one backend relative insert operation", async () => {
  const operations: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => splitWorkflowProject(),
        onboardProjectAction: async () => splitWorkflowProject()
      },
      taskMutation: {
        insertTaskRelativeAction: async (projectRoot, mode, selectedTaskId, insertedTaskId) => {
          operations.push(`insert:${projectRoot}:${mode}:${selectedTaskId}:${insertedTaskId}`);
          return {
            project: projectFixture({
              ...splitWorkflowProject(),
              tasks: [
                ...splitWorkflowProject().tasks,
                taskFixture(insertedTaskId, { path: `${projectRoot}/${insertedTaskId}` }),
              ],
            }),
          };
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      getByRole(root, "treeitem", /^analysis-run$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Add after analysis-run$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "archive-results";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(operations, [
        "insert:/tmp/fixture-spielgantt/workflow:after:analysis-run:archive-results",
      ]);
      assert.equal(
        getByRole(root, "treeitem", /^archive-results$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("cancelling task creation closes the dialog without mutating project state", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"], null, true),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep"]),
        onboardProjectAction: async () => workflowProject(["sample-prep"])
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      await openCreateTaskDialog(root, window);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Create Task$/), null);
      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "false");
    },
  );
});

test("task creation errors stay visible and accessible after a rejected submit", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"], null, true),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep"]),
        onboardProjectAction: async () => workflowProject(["sample-prep"])
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      taskMutation: {
        createTaskAction: async () => {
          throw new Error("Task 'Data review' already exists.");
        }
      }
    },
    async ({ root, window }) => {
      await openCreateTaskDialog(root, window);

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Data review";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();
      await waitForHandlers();

      const taskError = getByRole(root, "alert", /Task 'Data review' already exists\./);
      assert.match(taskError.textContent ?? "", /Task 'Data review' already exists\./);
      const taskNameField = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      assert.equal(taskNameField.form?.getAttribute("aria-describedby"), taskError.id);
      assert.equal((getByRole(root, "button", /^Create Task$/) as HTMLButtonElement).disabled, false);
    },
  );
});

test("renaming a task shows a submitting state and selects the renamed task after success", async () => {
  const renameTaskRequest = deferred<{ project: OpenProjectResult }>();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      taskMutation: {
        renameTaskAction: async () => renameTaskRequest.promise
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openRenameTaskDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "textbox", /^Task name$/),
        "rename task dialog should focus the task name input",
      );

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Sample review";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Rename Task$/);
      await waitForHandlers();

      const submitButton = getByRole(root, "button", /^Renaming Task\.\.\.$/) as HTMLButtonElement;
      assert.equal(submitButton.disabled, true);

      renameTaskRequest.resolve({
        project: workflowProject(["Sample review", "analysis-run"]),
      });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Sample review$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("deleting a task asks whether to remove chart metadata or delete the directory", async () => {
  const deleteTaskRequest = deferred<{ project: OpenProjectResult }>();
  const deleteRequests: Array<{ projectRoot: string; taskId: string; mode: string }> = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      taskMutation: {
        deleteTaskAction: async (projectRoot, taskId, mode) => {
          deleteRequests.push({ projectRoot, taskId, mode });
          return deleteTaskRequest.promise;
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openDeleteTaskDialog(root, window);
      assertActiveElement(
        root,
        getByRole(root, "button", /^Cancel$/),
        "delete task dialog should focus the cancel button",
      );

      assert.match(root.textContent ?? "", /Delete task 'sample-prep'\?/);
      assert.match(root.textContent ?? "", /depend on this task inherit its blockers/);
      assert.ok(getByRole(root, "button", /^Cancel$/));
      assert.ok(getByRole(root, "button", /^Remove from Chart$/));
      assert.ok(getByRole(root, "button", /^Delete Directory$/));

      submitDialog(root, window, /^Remove from Chart$/);
      await waitForHandlers();

      assert.deepEqual(deleteRequests, [
        {
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          taskId: "sample-prep",
          mode: "remove-from-chart",
        },
      ]);
      assert.equal(
        (getByRole(root, "button", /^Removing\.\.\.$/) as HTMLButtonElement).disabled,
        true,
      );
      assert.equal(
        (getByRole(root, "button", /^Delete Directory$/) as HTMLButtonElement).disabled,
        true,
      );

      deleteTaskRequest.resolve({ project: workflowProject(["analysis-run"]) });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(queryByRole(root, "treeitem", /^sample-prep$/), null);
    },
  );
});

test("deleting a task directory submits the directory mode with its own progress label", async () => {
  const deleteTaskRequest = deferred<{ project: OpenProjectResult }>();
  const deleteRequests: Array<{ projectRoot: string; taskId: string; mode: string }> = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      taskMutation: {
        deleteTaskAction: async (projectRoot, taskId, mode) => {
          deleteRequests.push({ projectRoot, taskId, mode });
          return deleteTaskRequest.promise;
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openDeleteTaskDialog(root, window);

      submitDialog(root, window, /^Delete Directory$/);
      await waitForHandlers();

      assert.deepEqual(deleteRequests, [
        {
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          taskId: "sample-prep",
          mode: "delete-directory",
        },
      ]);
      assert.equal(
        (getByRole(root, "button", /^Deleting\.\.\.$/) as HTMLButtonElement).disabled,
        true,
      );
      assert.equal(
        (getByRole(root, "button", /^Remove from Chart$/) as HTMLButtonElement).disabled,
        true,
      );

      deleteTaskRequest.resolve({ project: workflowProject(["analysis-run"]) });
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(queryByRole(root, "treeitem", /^sample-prep$/), null);
    },
  );
});

test("cancelling task delete closes the dialog without changing the selected task", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openDeleteTaskDialog(root, window);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Delete Directory$/), null);
      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("task delete backend errors stay visible and recoverable", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      taskMutation: {
        deleteTaskAction: async () => {
          throw new Error("Task directory is locked.");
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openDeleteTaskDialog(root, window);

      submitDialog(root, window, /^Delete Directory$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.match(
        getByRole(root, "alert", /Task directory is locked\./).textContent ?? "",
        /Task directory is locked\./,
      );
      assert.equal(
        (getByRole(root, "button", /^Delete Directory$/) as HTMLButtonElement).disabled,
        false,
      );
      assert.ok(getByRole(root, "button", /^Cancel$/));
    },
  );
});

test("cancelling task rename closes the dialog without changing the selected task", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openRenameTaskDialog(root, window);

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(queryByRole(root, "button", /^Rename Task$/), null);
      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("task rename errors stay visible and accessible after a rejected submit", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(["sample-prep", "analysis-run"]),
        onboardProjectAction: async () => workflowProject(["sample-prep", "analysis-run"])
      },
      taskMutation: {
        renameTaskAction: async () => {
          throw new Error("Task 'Sample review' already exists.");
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      await openRenameTaskDialog(root, window);

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Sample review";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Rename Task$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.match(root.textContent ?? "", /Task 'Sample review' already exists\./);
      assert.ok(getByRole(root, "textbox", /^Task name$/));
      assert.equal((getByRole(root, "button", /^Rename Task$/) as HTMLButtonElement).disabled, false);
    },
  );
});
