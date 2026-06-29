import assert from "node:assert/strict";
import test from "node:test";
import {
  clickTreeItem,
  getByRole,
  InMemoryRememberedProjectsSettings,
  interactiveElements,
  openProjectFromProjectsMenu,
  projectFixture,
  readyHealth,
  rememberedProjectRecord,
  taskFixture,
  waitForHandlers,
  withRenderedShell,
  workflowAnchors,
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

test("opening an existing project can fake only lifecycle and GUI session capabilities", async () => {
  const projectRoot = "/tmp/fixture-spielgantt/narrow-open";
  const openedProject = projectFixture({
    selectedPath: projectRoot,
    projectRoot,
    tasks: [
      taskFixture("sample-prep", {
        path: `${projectRoot}/sample-prep`,
      }),
    ],
  });
  const watchedProjectRoots: string[] = [];

  await withRenderedShell(
    {
      desktop: {
        pickProjectFolder: async () => projectRoot,
      },
      projectLifecycle: {
        onboardProjectAction: async () => openedProject,
        openSelectedProject: async () => openedProject,
      },
      projectWatch: {
        subscribeProjectSessionChanges: async (watchedProjectRoot) => {
          watchedProjectRoots.push(watchedProjectRoot);
          return {
            status: { watching: true, message: "test watch" },
            unsubscribe: () => {},
          };
        },
      },
    },
    async ({ root }) => {
      await openProjectFromProjectsMenu(root);

      assert.equal(getByRole(root, "treeitem", /^narrow-open$/).getAttribute("aria-selected"), "true");
      assert.ok(getByRole(root, "treeitem", /^sample-prep$/));
      assert.deepEqual(watchedProjectRoots, [projectRoot]);
    },
  );
});

test("shell opens with a compact app shell and an understandable empty state", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        loadHealth: async () => readyHealth()
      }
    },
    async ({ root }) => {
      assert.ok(getByRole(root, "tree", /^Projects$/));
      assert.ok(getByRole(root, "button", /^Add project$/));
      assert.match(root.textContent ?? "", /Choose a SpielGantt project folder to begin\./);
      assert.equal(queryByAccessibleLabel(root, "Tasks"), null);
    },
  );
});

test("clicking a remembered project opens and selects that project", async () => {
  const openedProjects: string[] = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
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
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(openedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "true");
      assert.equal(getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"), "false");
      assert.match(getByAccessibleLabel(root, "Project inspector").textContent ?? "", /workflow/);
    },
  );
});

test("rendered GUI controls have accessible names", async () => {
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
    async ({ root }) => {
      await openProjectFromProjectsMenu(root);

      for (const element of interactiveElements(root)) {
        const name =
          element.getAttribute("aria-label") ??
          element.getAttribute("title") ??
          element.textContent ??
          "";
        assert.notEqual(name.trim(), "", element.outerHTML);
      }
    },
  );
});
