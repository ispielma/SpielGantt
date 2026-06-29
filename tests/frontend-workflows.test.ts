import assert from "node:assert/strict";
import test from "node:test";
import {
  clickTreeItem,
  getByRole,
  InMemoryRememberedProjectsSettings,
  openProjectFromProjectsMenu,
  projectFixture,
  renderTestShell,
  rememberedProjectRecord,
  setNativeValue,
  taskFixture,
  waitForHandlers,
  withRenderedShell,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
  type OpenProjectResult,
} from "./support/frontend-shell.ts";

function getByAccessibleLabel(root: Element, label: string): HTMLElement {
  const match = Array.from(root.querySelectorAll<HTMLElement>("[aria-label]")).find(
    (element) => element.getAttribute("aria-label") === label,
  );
  if (!match) {
    throw new Error(`expected element labelled ${label} to be present`);
  }
  return match;
}

function queryByAccessibleLabel(root: Element, label: string): HTMLElement | null {
  try {
    return getByAccessibleLabel(root, label);
  } catch {
    return null;
  }
}

function queryByRole(root: Element, role: Parameters<typeof getByRole>[1], name: RegExp) {
  try {
    return getByRole(root, role, name);
  } catch {
    return null;
  }
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

function submitDialog(root: HTMLElement, window: Window, buttonName: RegExp): void {
  getByRole(root, "button", buttonName).dispatchEvent(
    new window.MouseEvent("click", { bubbles: true, cancelable: true }),
  );
}

function workflowProject(): OpenProjectResult {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["Samples ready", "Analysis complete"],
    eventReferences: [
      {
        id: "Samples ready",
        referencedTaskIds: ["sample-prep", "analysis-run"],
        blockerTaskIds: ["sample-prep"],
        blockedTaskIds: ["analysis-run"],
      },
      {
        id: "Analysis complete",
        referencedTaskIds: ["analysis-run"],
        blockerTaskIds: ["analysis-run"],
        blockedTaskIds: [],
      },
    ],
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
          valid_ends_at_targets: ["Samples ready", "Analysis complete"],
        }),
      ],
    }),
    tasks: [
      taskFixture("sample-prep", {
        path: "/tmp/fixture-spielgantt/workflow/sample-prep",
        blocks: [{ id: "analysis-run", kind: "task" }],
        status: "unblocked",
      }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        dependencies: ["Samples ready"],
        endsAt: "Analysis complete",
        status: "unblocked",
        readmeContent: "# Analysis\n",
        readmeVersion: "v1",
      }),
    ],
  });
}

test("creating a task after editing another task selects a clean new task inspector", async () => {
  const projectRoot = "/tmp/fixture-spielgantt/workflow";
  const initialProject = projectFixture({
    selectedPath: projectRoot,
    projectRoot,
    events: ["Samples ready", "Analysis complete"],
    eventReferences: [
      {
        id: "Samples ready",
        referencedTaskIds: [],
        blockerTaskIds: [],
        blockedTaskIds: [],
      },
      {
        id: "Analysis complete",
        referencedTaskIds: [],
        blockerTaskIds: [],
        blockedTaskIds: [],
      },
    ],
    workflow: workflowFixture({
      project_root: projectRoot,
      events: ["Samples ready", "Analysis complete"],
      event_nodes: workflowEventNodes(["Samples ready", "Analysis complete"]),
      tasks: [
        workflowTaskFixture("task-a", {
          valid_dependency_targets: ["Samples ready"],
          valid_ends_at_targets: ["Samples ready", "Analysis complete"],
        }),
      ],
    }),
    tasks: [
      taskFixture("task-a", {
        path: `${projectRoot}/task-a`,
        dependencyTargets: [{ id: "Samples ready", kind: "event" }],
      }),
    ],
  });
  const taskAWithEndEvent = projectFixture({
    ...initialProject,
    tasks: initialProject.tasks.map((task) =>
      task.id === "task-a" ? { ...task, endsAt: "Analysis complete" } : task,
    ),
  });
  const projectWithTaskB = projectFixture({
    ...taskAWithEndEvent,
    workflow: workflowFixture({
      ...taskAWithEndEvent.workflow!,
      tasks: [
        ...taskAWithEndEvent.workflow!.tasks,
        workflowTaskFixture("task-b", {
          valid_dependency_targets: ["Samples ready"],
          valid_ends_at_targets: ["Samples ready", "Analysis complete"],
        }),
      ],
    }),
    tasks: [
      ...taskAWithEndEvent.tasks,
      taskFixture("task-b", {
        path: `${projectRoot}/task-b`,
        dependencyTargets: [{ id: "Samples ready", kind: "event" }],
      }),
    ],
  });

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => initialProject,
        onboardProjectAction: async () => initialProject
      },
      taskMutation: {
        setTaskEndsAtAction: async () => ({ project: taskAWithEndEvent }),
        createTaskAction: async () => ({ project: projectWithTaskB })
      },
      desktop: {
        pickProjectFolder: async () => projectRoot
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      clickTreeItem(root, /^task-a$/);
      await waitForHandlers();

      const taskAEndEvent = getByRole(root, "combobox", /^End at event$/) as HTMLSelectElement;
      setNativeValue(taskAEndEvent, "Analysis complete");
      taskAEndEvent.dispatchEvent(new window.Event("change", { bubbles: true }));
      await waitForHandlers();

      const taskADependencyTarget = getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement;
      setNativeValue(taskADependencyTarget, "Samples ready");
      taskADependencyTarget.dispatchEvent(new window.Event("change", { bubbles: true }));
      await waitForHandlers();

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );

      await openCreateTaskDialog(root, window);
      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      setNativeValue(taskNameInput, "task-b");
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      submitDialog(root, window, /^Create Task$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(getByRole(root, "treeitem", /^task-b$/).getAttribute("aria-selected"), "true");
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.equal(queryByAccessibleLabel(root, "Event inspector"), null);

      const taskBInspector = getByAccessibleLabel(root, "Task inspector");
      assert.match(taskBInspector.textContent ?? "", /task-b/);
      assert.equal((getByRole(root, "combobox", /^End at event$/) as HTMLSelectElement).value, "");
      assert.equal((getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement).value, "");
    },
  );
});

test("task selection works from the sidebar and from the timeline", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^analysis-run$/);
      await waitForHandlers();
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );

      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      const analysisTimelineBar = getByRole(
        root,
        "button",
        /^Task analysis-run spans Samples ready to Analysis complete on the event axis with semantic binding nodes at Samples ready and Analysis complete$/,
      );
      analysisTimelineBar.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
      analysisTimelineBar.dispatchEvent(
        new window.MouseEvent("contextmenu", {
          bubbles: true,
          cancelable: true,
          clientX: 420,
          clientY: 180,
        }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Open task folder analysis-run$/i));
      assert.ok(getByRole(root, "menuitem", /^Rename task analysis-run$/i));
      assert.match(root.textContent ?? "", /unblocked/);
    },
  );
});

test("clicking an event in the active remembered project selects the event", async () => {
  const openedProjects: string[] = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(
      "/tmp/fixture-spielgantt/workflow",
      ["sample-prep", "analysis-run"],
      "sample-prep",
      true,
      ["Samples ready", "Analysis complete"],
    ),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) => {
          openedProjects.push(path);
          return workflowProject();
        },
        onboardProjectAction: async (path) => {
          openedProjects.push(path);
          return workflowProject();
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      await waitForHandlers();
      const initialOpenCount = openedProjects.length;

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      clickTreeItem(root, /^Analysis complete$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(
        openedProjects.slice(initialOpenCount),
        ["/tmp/fixture-spielgantt/workflow"],
      );
      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "false");
      assert.equal(
        getByRole(root, "treeitem", /^Analysis complete$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(queryByAccessibleLabel(root, "Task inspector"), null);
      assert.match(
        getByAccessibleLabel(root, "Event inspector").textContent ?? "",
        /Analysis complete/,
      );
      assert.doesNotMatch(root.textContent ?? "", /Selected event 'Analysis complete'/);
    },
  );
});

test("active project expansion does not leak across React app remounts", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings();

  {
    const { root, cleanup } = await renderTestShell({
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    });
    try {
      await waitForHandlers();
      await openProjectFromProjectsMenu(root);

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.match(getByRole(root, "tree", /^Projects$/).textContent ?? "", /analysis-run/);
    } finally {
      await cleanup();
    }
  }

  {
    const { root, cleanup } = await renderTestShell({
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    });
    try {
      await waitForHandlers();

      assert.doesNotMatch(getByRole(root, "tree", /^Projects$/).textContent ?? "", /analysis-run/);
    } finally {
      await cleanup();
    }
  }
});

test("selected task metadata and README edits use the mounted shell action", async () => {
  const editedTasks: Array<unknown> = [];
  const project = workflowProject();

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => project,
        onboardProjectAction: async () => project
      },
      taskMutation: {
        editTaskAction: async (projectRoot, taskId, edit) => {
          editedTasks.push({ projectRoot, taskId, edit });
          return {
            project: {
              ...project,
              tasks: project.tasks.map((task) =>
                task.id === taskId
                  ? {
                      ...task,
                      status: "done",
                      readmeContent: "# Analysis\n\n- reported\n",
                      readmeVersion: "v2",
                    }
                  : task,
              ),
            },
          };
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);
      clickTreeItem(root, /^analysis-run$/);
      await waitForHandlers();

      const status = getByRole(root, "combobox", /^Task status$/) as HTMLSelectElement;
      setNativeValue(status, "done");
      status.dispatchEvent(new window.Event("change", { bubbles: true }));
      await waitForHandlers();

      assert.equal(queryByRole(root, "textbox", /^Task progress$/), null);

      const readme = getByRole(root, "textbox", /^Task README$/) as HTMLTextAreaElement;
      setNativeValue(readme, "# Analysis\n\n- reported\n");
      readme.dispatchEvent(new window.Event("input", { bubbles: true }));
      readme.dispatchEvent(new window.Event("blur", { bubbles: true }));
      readme.dispatchEvent(new window.Event("focusout", { bubbles: true }));
      await waitForHandlers();

      assert.deepEqual(editedTasks.at(-1), {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        edit: {
          status: "done",
          readmeContent: "# Analysis\n\n- reported\n",
          expectedReadmeVersion: "v2",
        },
      });
      assert.doesNotMatch(root.textContent ?? "", /Saved task 'analysis-run'/);
      assert.equal(queryByRole(root, "textbox", /^Task progress$/), null);
    },
  );
});

test("task inspector renders backend-provided dependency relationships", async () => {
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    tasks: [
      taskFixture("sample-prep", {
        blocks: [{ id: "analysis-run", kind: "task" }],
        dependencyTargets: [{ id: "Samples ready", kind: "event" }],
      }),
      taskFixture("analysis-run"),
    ],
  });

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => project,
        onboardProjectAction: async () => project
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root }) => {
      await openProjectFromProjectsMenu(root);
      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      await waitForHandlers();

      const inspector = getByAccessibleLabel(root, "Task inspector");
      assert.match(inspector.textContent ?? "", /sample-prep/);
      assert.match(inspector.textContent ?? "", /Event: Samples ready/);
    },
  );
});
