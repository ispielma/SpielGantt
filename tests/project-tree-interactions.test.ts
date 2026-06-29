import assert from "node:assert/strict";
import test from "node:test";

import {
  accessibleName,
  clickTreeItem,
  alignmentCollisionPlan,
  alignmentPlan,
  elementRole,
  getByRole,
  InMemoryRememberedProjectsSettings,
  interactiveElements,
  openProjectFromProjectsMenu,
  projectFixture,
  readyHealth,
  rememberedProjectRecord,
  renderTestShell,
  taskFixture,
  waitForHandlers,
  withRenderedShell,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
} from "./support/frontend-shell.ts";

function queryByAccessibleLabel(root: Element, name: RegExp): HTMLElement | null {
  return (
    Array.from(root.querySelectorAll<HTMLElement>("[aria-label]")).find((element) =>
      name.test(accessibleName(element)),
    ) ?? null
  );
}

function getByAccessibleLabel(root: Element, name: RegExp): HTMLElement {
  const match = queryByAccessibleLabel(root, name);
  if (!match) {
    throw new Error(`expected element labelled ${name} to be present`);
  }
  return match;
}

function menuItemNames(root: Element): string[] {
  return Array.from((root.ownerDocument?.body ?? root).querySelectorAll<HTMLElement>("*"))
    .filter((element) => elementRole(element) === "menuitem")
    .map((element) => accessibleName(element));
}

function getProjectMenuItem(root: Element, name: RegExp): HTMLElement {
  return getByRole(root.ownerDocument?.body ?? root, "menuitem", name);
}

function getIndependentControl(
  root: Element,
  role: "button",
  name: RegExp,
): HTMLElement {
  const control = getByRole(root, role, name);
  assert.equal(
    control.closest("[role='treeitem']"),
    null,
    `${accessibleName(control)} must be exposed independently from navigation tree rows`,
  );
  return control;
}

function getExactTreeRow(root: Element, name: RegExp, expectedName: string): HTMLElement {
  const row = getByRole(root, "treeitem", name);
  assert.equal(accessibleName(row), expectedName);
  return row;
}

function rect(top: number, height: number): DOMRect {
  return {
    bottom: top + height,
    height,
    left: 0,
    right: 240,
    top,
    width: 240,
    x: 0,
    y: top,
    toJSON: () => ({}),
  } as DOMRect;
}

function assertCleanTreeRowName(root: Element, name: RegExp, expectedName: string): HTMLElement {
  const row = getExactTreeRow(root, name, expectedName);
  assert.equal(
    row.getAttribute("aria-label"),
    expectedName,
    `${expectedName} should expose its full row name through aria-label`,
  );
  assert.equal(
    row.getAttribute("title"),
    null,
    `${expectedName} should not duplicate its row name through a native title`,
  );
  assert.deepEqual(
    Array.from(row.querySelectorAll("[title]")).map((element) => element.getAttribute("title")),
    [],
    `${expectedName} should not repeat row text in descendant native titles`,
  );
  assert.doesNotMatch(
    accessibleName(row),
    /Project actions|Task creation options|Create event/i,
    `${expectedName} row name should not include sidebar action labels`,
  );
  return row;
}

function workflowProject() {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["Samples ready", "Analysis complete"],
    eventReferences: [
      {
        id: "Samples ready",
        referencedTaskIds: ["analysis-run"],
        blockerTaskIds: [],
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

function archiveProject() {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/archive",
    projectRoot: "/tmp/fixture-spielgantt/archive",
    events: ["Archive ready"],
    tasks: [
      taskFixture("archive-cleanup", {
        path: "/tmp/fixture-spielgantt/archive/archive-cleanup",
        status: "unblocked",
      }),
    ],
  });
}

test("only the active remembered project renders its task and event tree", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", [], null, true),
    rememberedProjectRecord("/tmp/fixture-spielgantt/archive", [], null, true),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject(),
        onboardProjectAction: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^workflow$/));
      assert.ok(getByRole(root, "treeitem", /^archive$/));
      assert.throws(() => getByRole(root, "treeitem", /^sample-prep$/));
      assert.throws(() => getByRole(root, "treeitem", /^archive-cleanup$/));

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^sample-prep$/));
      assert.ok(getByRole(root, "treeitem", /^Samples ready$/));
      assert.throws(() => getByRole(root, "treeitem", /^archive-cleanup$/));
      assert.throws(() => getByRole(root, "treeitem", /^Archive ready$/));
    },
  );
});

test("unresolved sidebar task and event rows keep stable accessible names", async () => {
  const longTaskName =
    "2026_04_15_RbK_New_Directions_with_a_deliberately_long_sidebar_label";
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["start", "Long unresolved event label", "finished"],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "Long unresolved event label", "finished"],
      event_nodes: workflowEventNodes(["start", "Long unresolved event label", "finished"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture(longTaskName, {
          effective_anchors: workflowAnchors(null, null),
        }),
      ],
    }),
    tasks: [
      taskFixture("ordinary-task", {
        path: "/tmp/fixture-spielgantt/workflow/ordinary-task",
        status: "unblocked",
      }),
      taskFixture(longTaskName, {
        path: `/tmp/fixture-spielgantt/workflow/${longTaskName}`,
        status: "unblocked",
      }),
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

      const diagnosticTaskRow = getByRole(root, "treeitem", new RegExp(`^${longTaskName}$`));
      const diagnosticEventRow = getByRole(root, "treeitem", /^Long unresolved event label$/);

      assert.equal(accessibleName(diagnosticTaskRow), longTaskName);
      assert.equal(accessibleName(diagnosticEventRow), "Long unresolved event label");
    },
  );
});

test("sidebar tree rows avoid duplicate native names while preserving full long labels", async () => {
  const longTaskName =
    "2026_04_15_RbK_New_Directions_with_a_deliberately_long_sidebar_label";
  const longEventName =
    "Long unresolved event label that should stay fully available to assistive technology";
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: [longEventName],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: [longEventName],
      event_nodes: [
        {
          id: longEventName,
          boundary_role: "ordinary",
          chart_order: 0,
          placement_ready: false,
          placement_status: "incomplete",
          placement_messages: ["Event placement is unresolved"],
        },
      ],
      tasks: [
        workflowTaskFixture(longTaskName, {
          effective_anchors: workflowAnchors(null, null, ["Task placement is unresolved"]),
        }),
      ],
    }),
    tasks: [
      taskFixture(longTaskName, {
        path: `/tmp/fixture-spielgantt/workflow/${longTaskName}`,
        status: "unblocked",
      }),
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

      assertCleanTreeRowName(root, /^workflow$/, "workflow");
      assertCleanTreeRowName(root, /^Tasks$/, "Tasks");
      assertCleanTreeRowName(root, /^Events$/, "Events");
      assertCleanTreeRowName(root, new RegExp(`^${longTaskName}$`), longTaskName);
      assertCleanTreeRowName(root, new RegExp(`^${longEventName}$`), longEventName);
    },
  );

  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/Missing Project", [], null, true),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => {
          throw new Error("project folder is gone");
        },
        onboardProjectAction: async () => {
          throw new Error("project folder is gone");
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      await waitForHandlers();
      await waitForHandlers();

      assertCleanTreeRowName(root, /^Missing Project$/, "Missing Project");
    },
  );
});

test("switching remembered projects stores only the active project expanded", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow"),
    rememberedProjectRecord("/tmp/fixture-spielgantt/archive"),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject(),
        onboardProjectAction: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();
      assert.ok(getByRole(root, "treeitem", /^sample-prep$/));

      clickTreeItem(root, /^archive$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^archive-cleanup$/));
      assert.throws(() => getByRole(root, "treeitem", /^sample-prep$/));
      const expandedRecords = (await rememberedProjects.listProjects())
        .filter((record) => record.expanded)
        .map((record) => record.projectPath);
      assert.deepEqual(expandedRecords, ["/tmp/fixture-spielgantt/archive"]);
    },
  );
});

test("project task and event selection stay scoped to the active project tree", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow"),
    rememberedProjectRecord("/tmp/fixture-spielgantt/archive"),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject(),
        onboardProjectAction: async (path) =>
          path.endsWith("/archive") ? archiveProject() : workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      clickTreeItem(root, /^analysis-run$/);
      await waitForHandlers();
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"),
        "false",
      );

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "false",
      );

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "true");
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.throws(() => getByRole(root, "treeitem", /^archive-cleanup$/));
    },
  );
});

test("clicking the active project row keeps that project expanded in storage", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow"),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^analysis-run$/));
      assert.deepEqual(
        (await rememberedProjects.listProjects()).map((record) => ({
          projectPath: record.projectPath,
          expanded: record.expanded,
        })),
        [{ projectPath: "/tmp/fixture-spielgantt/workflow", expanded: true }],
      );
    },
  );
});

test("clicking task and event section rows toggles their subtrees", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow"),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^analysis-run$/));
      assert.ok(getByRole(root, "treeitem", /^Samples ready$/));

      clickTreeItem(root, /^Tasks$/);
      await waitForHandlers();

      assert.throws(() => getByRole(root, "treeitem", /^analysis-run$/));
      assert.ok(getByRole(root, "treeitem", /^Samples ready$/));

      clickTreeItem(root, /^Tasks$/);
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^analysis-run$/));

      clickTreeItem(root, /^Events$/);
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^analysis-run$/));
      assert.throws(() => getByRole(root, "treeitem", /^Samples ready$/));

      clickTreeItem(root, /^Events$/);
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^Samples ready$/));
    },
  );
});

test("clicking a remembered project restores the last selected task when it is still valid", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(
      "/tmp/fixture-spielgantt/workflow",
      ["sample-prep", "analysis-run"],
      "analysis-run",
    ),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "false");
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("generic right-clicks are suppressed instead of opening the webview context menu", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        loadHealth: async () => readyHealth()
      }
    },
    async ({ root, window }) => {
      const planningSurface = getByAccessibleLabel(root, /^Planning surface$/);
      const event = new window.MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: 120,
        clientY: 120,
      });

      planningSurface.dispatchEvent(event);
      await waitForHandlers();

      assert.equal(event.defaultPrevented, true);
      assert.deepEqual(menuItemNames(root), []);
    },
  );
});

test("opening a remembered project skips the alignment dialog when no folder changes are required", async () => {
  const openedProjects: string[] = [];
  const watchedProjects: string[] = [];
  const windowTitles: string[] = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"], "analysis-run"),
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
        },
        previewAlignmentAction: async () => ({ operations: [], preflightIssues: [] })
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        setWindowTitle: async (title) => {
          windowTitles.push(title);
        }
      },
      projectWatch: {
        subscribeProjectSessionChanges: async (projectPath) => {
          watchedProjects.push(projectPath);
          return {
            status: { watching: true, message: "watching" },
            unsubscribe: () => {},
          };
        }
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(openedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.deepEqual(watchedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.deepEqual(windowTitles, ["SpielGantt - workflow"]);
      assert.doesNotMatch(root.textContent ?? "", /Confirm Folder Alignment/);
      assert.equal(getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"), "false");
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        (await rememberedProjects.listProjects()).find(
          (record) => record.projectPath === "/tmp/fixture-spielgantt/workflow",
        )?.lastSelectedTaskName,
        "analysis-run",
      );
    },
  );
});

test("opening a remembered project shows planned alignment renames in a dialog", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        previewAlignmentAction: async () => alignmentPlan()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();

      assert.match(root.textContent ?? "", /Planned renames/);
      assert.match(root.textContent ?? "", /Calibrate Laser/);
      assert.match(root.textContent ?? "", /analysis notes/);
      assert.match(root.textContent ?? "", /\/tmp\/fixture-spielgantt\/Calibrate Laser/);
    },
  );
});

test("starting a new project closes an open alignment dialog", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        previewAlignmentAction: async () => alignmentPlan()
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        resolveDefaultProjectParent: async () => "/tmp/fixture-spielgantt"
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      assert.match(root.textContent ?? "", /Confirm Folder Alignment/);

      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Start from scratch$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.doesNotMatch(root.textContent ?? "", /Confirm Folder Alignment/);
      assert.ok(getByRole(root, "button", /^Create Project$/));
    },
  );
});

test("clicking a remembered project reuses alignment confirmation before opening", async () => {
  const openedProjects: string[] = [];
  const appliedProjects: string[] = [];
  const watchedProjects: string[] = [];
  const windowTitles: string[] = [];
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
        },
        previewAlignmentAction: async () => alignmentPlan(),
        applyAlignmentAction: async (projectPath) => {
          appliedProjects.push(projectPath);
          return {
            project: workflowProject(),
            renames: [],
            issues: [],
            applied: true,
          };
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        setWindowTitle: async (title) => {
          windowTitles.push(title);
        }
      },
      projectWatch: {
        subscribeProjectSessionChanges: async (projectPath) => {
          watchedProjects.push(projectPath);
          return {
            status: { watching: true, message: "watching" },
            unsubscribe: () => {},
          };
        }
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();

      assert.deepEqual(openedProjects, []);
      assert.match(root.textContent ?? "", /Confirm Folder Alignment/);

      getByRole(root, "button", /^Apply Alignment and Open$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(openedProjects, []);
      assert.deepEqual(appliedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.deepEqual(watchedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.deepEqual(windowTitles, ["SpielGantt - workflow"]);
      assert.equal(
        getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("alignment collisions are shown clearly and keep apply disabled", async () => {
  const appliedProjects: string[] = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        previewAlignmentAction: async () => alignmentCollisionPlan(),
        applyAlignmentAction: async (projectPath) => {
          appliedProjects.push(projectPath);
          return {
            project: workflowProject(),
            renames: [],
            issues: [],
            applied: true,
          };
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();

      const applyButton = getByRole(root, "button", /^Apply Alignment and Open$/) as HTMLButtonElement;
      assert.equal(applyButton.disabled, true);
      assert.match(root.textContent ?? "", /Folder collisions/);
      assert.match(root.textContent ?? "", /\/tmp\/fixture-spielgantt\/Calibrate Laser/);

      applyButton.dispatchEvent(new window.Event("click", { bubbles: true }));
      await waitForHandlers();

      assert.deepEqual(appliedProjects, []);
    },
  );
});

test("canceling alignment closes the dialog without applying the plan", async () => {
  const openedProjects: string[] = [];
  const appliedProjects: string[] = [];
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
        },
        previewAlignmentAction: async () => alignmentPlan(),
        applyAlignmentAction: async (projectPath) => {
          appliedProjects.push(projectPath);
          return {
            project: workflowProject(),
            renames: [],
            issues: [],
            applied: true,
          };
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.Event("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.doesNotMatch(root.textContent ?? "", /Confirm Folder Alignment/);
      assert.deepEqual(openedProjects, []);
      assert.deepEqual(appliedProjects, []);
    },
  );
});

test("keyboard tree navigation keeps the active project expanded while selecting tasks", async () => {
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

      const projectRow = getByRole(root, "treeitem", /^workflow$/);
      projectRow.focus();
      projectRow.dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.match(getByRole(root, "tree", /^Projects$/).textContent ?? "", /analysis-run/);

      getByRole(root, "treeitem", /^workflow$/).dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.match(getByRole(root, "tree", /^Projects$/).textContent ?? "", /analysis-run/);
      assert.equal(window.document.activeElement, getByRole(root, "treeitem", /^workflow$/));

      getByRole(root, "treeitem", /^analysis-run$/).dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("external project refresh updates silently when it succeeds", async () => {
  let projectChanged: (() => Promise<void>) | null = null;
  let openCount = 0;

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => {
          openCount += 1;
          return projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/workflow",
            projectRoot: "/tmp/fixture-spielgantt/workflow",
            tasks: [
              taskFixture(openCount === 1 ? "analysis-run" : "analysis-complete", {
                path:
                  openCount === 1
                    ? "/tmp/fixture-spielgantt/workflow/analysis-run"
                    : "/tmp/fixture-spielgantt/workflow/analysis-complete",
              }),
            ],
          });
        },
        onboardProjectAction: async () => {
          openCount += 1;
          return projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/workflow",
            projectRoot: "/tmp/fixture-spielgantt/workflow",
            tasks: [
              taskFixture(openCount === 1 ? "analysis-run" : "analysis-complete", {
                path:
                  openCount === 1
                    ? "/tmp/fixture-spielgantt/workflow/analysis-run"
                    : "/tmp/fixture-spielgantt/workflow/analysis-complete",
              }),
            ],
          });
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      },
      projectWatch: {
        subscribeProjectSessionChanges: async (_projectRoot, refreshProject) => {
          projectChanged = refreshProject;
          return {
            status: { watching: true, message: "watching" },
            unsubscribe: () => {},
          };
        }
      }
    },
    async ({ root }) => {
      await openProjectFromProjectsMenu(root);

      assert.ok(projectChanged, "opening a project should subscribe to project changes");
      await projectChanged();
      await waitForHandlers();

      assert.match(getByRole(root, "tree", /^Projects$/).textContent ?? "", /analysis-complete/);
      assert.doesNotMatch(root.textContent ?? "", /Refreshed project/);
    },
  );
});

test("project action fallback menu can create a task for a remembered project", async () => {
  const createdTasks: Array<{ projectPath: string; taskId: string }> = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      taskMutation: {
        createTaskAction: async (projectPath, taskId) => {
          createdTasks.push({ projectPath, taskId });
          return {
            project: workflowProject(),
          };
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getProjectMenuItem(root, /^Create task in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      const taskNameInput = getByRole(root, "textbox", /^Task name$/) as HTMLInputElement;
      taskNameInput.value = "Data review";
      taskNameInput.dispatchEvent(new window.Event("input", { bubbles: true }));
      const createTaskButton = getByRole(root, "button", /^Create Task$/) as HTMLButtonElement;
      createTaskButton.form?.dispatchEvent(
        new window.Event("submit", { bubbles: true, cancelable: true }),
      );
      await waitForHandlers();

      assert.deepEqual(createdTasks, [
        {
          projectPath: "/tmp/fixture-spielgantt/workflow",
          taskId: "Data review",
        },
      ]);
    },
  );
});

test("project action fallback menu can create a task from a project folder", async () => {
  const listedProjects: string[] = [];
  const adoptedFolders: Array<{ projectPath: string; folderPath: string; taskId: string }> = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);
  const adoptedProject = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    tasks: [
      ...workflowProject().tasks,
      taskFixture("Alpha Samples", {
        path: "/tmp/fixture-spielgantt/workflow/Alpha Samples",
        projectRelativePath: "Alpha Samples",
      }),
    ],
  });

  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      taskMutation: {
        listAdoptableTaskFoldersAction: async (projectPath) => {
          listedProjects.push(projectPath);
          return [
            {
              folderPath: "/tmp/fixture-spielgantt/workflow/Alpha Samples",
              projectRelativePath: "Alpha Samples",
              taskId: "Alpha Samples",
            },
            {
              folderPath: "/tmp/fixture-spielgantt/workflow/zeta notes",
              projectRelativePath: "zeta notes",
              taskId: "zeta notes",
            },
          ];
        },
        adoptTaskAction: async (projectPath, folderPath, taskId) => {
          adoptedFolders.push({ projectPath, folderPath, taskId });
          return { project: adoptedProject };
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getProjectMenuItem(root, /^Create task from folder in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(listedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.match(root.textContent ?? "", /Create Task from Folder/);
      assert.ok(
        window.document.activeElement === getByRole(root, "button", /^Alpha Samples$/),
        "task-from-folder dialog should focus the first candidate",
      );

      const okButton = getByRole(root, "button", /^OK$/) as HTMLButtonElement;
      assert.equal(okButton.disabled, true);
      getByRole(root, "button", /^Alpha Samples$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.equal(okButton.disabled, false);

      okButton.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(adoptedFolders, [
        {
          projectPath: "/tmp/fixture-spielgantt/workflow",
          folderPath: "/tmp/fixture-spielgantt/workflow/Alpha Samples",
          taskId: "Alpha Samples",
        },
      ]);
      assert.equal(
        getByRole(root, "treeitem", /^Alpha Samples$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("right-clicking a remembered project exposes project actions through accessible menu items", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      getByRole(root, "treeitem", /^workflow$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getProjectMenuItem(root, /^Refresh project workflow$/i));
      assert.ok(getProjectMenuItem(root, /^Reveal project workflow in Finder$/i));
      assert.ok(getProjectMenuItem(root, /^Create task in workflow$/i));
      assert.ok(getProjectMenuItem(root, /^Create task from folder in workflow$/i));
      assert.ok(getProjectMenuItem(root, /^Create event in workflow$/i));
    },
  );
});

test("missing remembered projects render without cached task or event children and expose recovery actions", async () => {
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord(
      "/tmp/fixture-spielgantt/moved-workflow",
      ["stale-task"],
      "stale-task",
      true,
      ["Stale event"],
    ),
  ]);

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => {
          throw new Error("invalid project path '/tmp/fixture-spielgantt/moved-workflow'");
        },
        onboardProjectAction: async () => {
          throw new Error("invalid project path '/tmp/fixture-spielgantt/moved-workflow'");
        }
      },
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      }
    },
    async ({ root, window }) => {
      await waitForHandlers();
      await waitForHandlers();

      assert.ok(getByRole(root, "treeitem", /^moved-workflow$/));
      assert.throws(() => getByRole(root, "treeitem", /^stale-task$/));
      assert.throws(() => getByRole(root, "treeitem", /^Stale event$/));
      assert.doesNotMatch(root.textContent ?? "", /invalid project path/);

      getByRole(root, "button", /^Project actions for moved-workflow$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.deepEqual(menuItemNames(root), [
        "Find project moved-workflow",
        "Remove project moved-workflow from sidebar",
      ]);
    },
  );
});

test("task and event section rows expose right-aligned create actions", async () => {
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

      getExactTreeRow(root, /^workflow$/, "workflow");
      const tasksSection = getExactTreeRow(root, /^Tasks$/, "Tasks");
      const eventsSection = getExactTreeRow(root, /^Events$/, "Events");

      assert.equal(
        getByRole(root, "button", /^Project actions for workflow$/).getAttribute("aria-haspopup"),
        "menu",
      );
      assert.ok(tasksSection);
      assert.ok(eventsSection);
      assert.ok(getIndependentControl(root, "button", /^Task creation options for workflow$/i));
      assert.ok(getIndependentControl(root, "button", /^Create event in workflow$/i));

      getIndependentControl(root, "button", /^Task creation options for workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Create task in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "textbox", /^Task name$/));
      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getIndependentControl(root, "button", /^Create event in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "textbox", /^Event name$/));
      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getProjectMenuItem(root, /^Create task in workflow$/i));
      assert.ok(getProjectMenuItem(root, /^Create task from folder in workflow$/i));
      assert.ok(getProjectMenuItem(root, /^Create event in workflow$/i));
    },
  );
});

test("section create actions align with section rows when subtrees are expanded", async () => {
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
      const originalRect = window.HTMLElement.prototype.getBoundingClientRect;
      window.HTMLElement.prototype.getBoundingClientRect = function getBoundingClientRect() {
        if (this.classList.contains("remembered-projects")) {
          return rect(100, 220);
        }
        if (this.classList.contains("projects-tree-node-row-section")) {
          return this.textContent?.includes("Events") ? rect(198, 24) : rect(126, 24);
        }
        if (this.getAttribute("role") === "treeitem") {
          const name = this.getAttribute("aria-label");
          if (name === "Tasks") {
            return rect(126, 72);
          }
          if (name === "Events") {
            return rect(198, 72);
          }
        }
        return originalRect.call(this);
      };

      try {
        await openProjectFromProjectsMenu(root);
        await waitForHandlers();

        const taskSlot = root.querySelector<HTMLElement>(
          ".project-tree-section-action-slot[data-section='tasks']",
        );
        const eventSlot = root.querySelector<HTMLElement>(
          ".project-tree-section-action-slot[data-section='events']",
        );

        assert.ok(taskSlot);
        assert.ok(eventSlot);
        assert.equal(taskSlot.style.top, "26px");
        assert.equal(taskSlot.style.height, "24px");
        assert.equal(eventSlot.style.top, "98px");
        assert.equal(eventSlot.style.height, "24px");
      } finally {
        window.HTMLElement.prototype.getBoundingClientRect = originalRect;
      }
    },
  );
});

test("independent section create controls support keyboard activation", async () => {
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

      const taskCreateButton = getIndependentControl(
        root,
        "button",
        /^Task creation options for workflow$/i,
      );
      taskCreateButton.focus();
      assert.equal(window.document.activeElement, taskCreateButton);
      taskCreateButton.dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
      );
      await waitForHandlers();

      assert.equal(taskCreateButton.getAttribute("aria-expanded"), "true");
      assert.ok(getByRole(root, "menuitem", /^Create task in workflow$/i));

      getByRole(root, "tree", /^Projects$/).dispatchEvent(
        new window.MouseEvent("mousedown", { bubbles: true, clientX: 260, clientY: 12 }),
      );
      await waitForHandlers();

      const eventCreateButton = getIndependentControl(
        root,
        "button",
        /^Create event in workflow$/i,
      );
      eventCreateButton.focus();
      assert.equal(window.document.activeElement, eventCreateButton);
      eventCreateButton.dispatchEvent(
        new window.KeyboardEvent("keydown", { key: " ", bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "textbox", /^Event name$/));
    },
  );
});

test("tasks section create menu offers plain task and folder adoption choices", async () => {
  const listedProjects: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      taskMutation: {
        listAdoptableTaskFoldersAction: async (projectPath) => {
          listedProjects.push(projectPath);
          return [
            {
              folderPath: "/tmp/fixture-spielgantt/workflow/Alpha Samples",
              projectRelativePath: "Alpha Samples",
              taskId: "Alpha Samples",
            },
          ];
        }
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      getExactTreeRow(root, /^workflow$/, "workflow");
      getExactTreeRow(root, /^Tasks$/, "Tasks");
      getIndependentControl(root, "button", /^Task creation options for workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "menuitem", /^Create task from folder in workflow$/i));
      getByRole(root, "tree", /^Projects$/).dispatchEvent(
        new window.MouseEvent("mousedown", { bubbles: true, clientX: 260, clientY: 12 }),
      );
      await waitForHandlers();
      assert.deepEqual(menuItemNames(root), []);

      getIndependentControl(root, "button", /^Task creation options for workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "menuitem", /^Create task in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "textbox", /^Task name$/));

      getByRole(root, "button", /^Cancel$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getIndependentControl(root, "button", /^Task creation options for workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      getByRole(root, "menuitem", /^Create task from folder in workflow$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(listedProjects, ["/tmp/fixture-spielgantt/workflow"]);
      assert.match(root.textContent ?? "", /Create Task from Folder/);
      assert.ok(getByRole(root, "button", /^Alpha Samples$/));
    },
  );
});

test("project tree rows expose clean names and independent section create controls", async () => {
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

      const projectRow = getExactTreeRow(root, /^workflow$/, "workflow");
      const tasksSection = getExactTreeRow(root, /^Tasks$/, "Tasks");
      const eventsSection = getExactTreeRow(root, /^Events$/, "Events");
      getExactTreeRow(root, /^sample-prep$/, "sample-prep");
      getExactTreeRow(root, /^Samples ready$/, "Samples ready");

      assert.ok(getByRole(root, "button", /^Project actions for workflow$/));

      assert.equal(accessibleName(projectRow), "workflow");
      assert.equal(accessibleName(tasksSection), "Tasks");
      assert.equal(accessibleName(eventsSection), "Events");
      assert.ok(getIndependentControl(root, "button", /^Task creation options for workflow$/));
      assert.ok(getIndependentControl(root, "button", /^Create event in workflow$/));
    },
  );
});

test("project tree task and event leaves expose only one selectable tree item", async () => {
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

      for (const name of [/^analysis-run$/, /^Samples ready$/]) {
        const row = getByRole(root, "treeitem", name);

        assert.deepEqual(
          interactiveElements(row).map((element) => elementRole(element)),
          [],
          `tree leaf ${name} should not expose extra checkbox-like or duplicate controls`,
        );
      }
    },
  );
});

test("keyboard project action fallback can open the project folder", async () => {
  const openedProjectFolders: string[] = [];
  const rememberedProjects = new InMemoryRememberedProjectsSettings([
    rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", ["sample-prep"]),
  ]);

  await withRenderedShell(
    {
      projectSession: {
        loadRememberedProjectsSettings: async () => rememberedProjects
      },
      desktop: {
        openProjectFolderAction: async (projectPath) => {
          openedProjectFolders.push(projectPath);
        }
      }
    },
    async ({ root, window }) => {
      getByRole(root, "treeitem", /^workflow$/).dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "ContextMenu", bubbles: true }),
      );
      await waitForHandlers();

      getProjectMenuItem(root, /^Reveal project workflow in Finder$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.deepEqual(openedProjectFolders, ["/tmp/fixture-spielgantt/workflow"]);
    },
  );
});

test("clicking task and event rows selects the clicked navigator item", async () => {
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

      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );
    },
  );
});

test("task and event rows support keyboard activation across the navigator", async () => {
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

      const taskRow = getByRole(root, "treeitem", /^sample-prep$/);
      const eventRow = getByRole(root, "treeitem", /^Samples ready$/);

      taskRow.focus();
      assert.equal(window.document.activeElement, taskRow);
      taskRow.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(taskRow.getAttribute("aria-selected"), "true");

      eventRow.focus();
      assert.equal(window.document.activeElement, eventRow);
      eventRow.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(eventRow.getAttribute("aria-selected"), "true");
    },
  );
});

test("project, task, and event selection stay mutually exclusive while the inspector follows the active object", async () => {
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

      const projectRow = getByRole(root, "treeitem", /^workflow$/);
      const taskRow = getByRole(root, "treeitem", /^sample-prep$/);
      const eventRow = getByRole(root, "treeitem", /^Samples ready$/);

      assert.equal(projectRow.getAttribute("aria-selected"), "true");
      assert.equal(taskRow.getAttribute("aria-selected"), "false");
      assert.equal(eventRow.getAttribute("aria-selected"), "false");
      assert.match(getByAccessibleLabel(root, /^Project inspector$/).textContent ?? "", /workflow/);
      assert.equal(queryByAccessibleLabel(root, /^Task inspector$/), null);
      assert.equal(queryByAccessibleLabel(root, /^Event inspector$/), null);

      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(projectRow.getAttribute("aria-selected"), "false");
      assert.equal(taskRow.getAttribute("aria-selected"), "true");
      assert.equal(eventRow.getAttribute("aria-selected"), "false");
      assert.equal(queryByAccessibleLabel(root, /^Event inspector$/), null);
      assert.match(getByAccessibleLabel(root, /^Task inspector$/).textContent ?? "", /sample-prep/);

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(projectRow.getAttribute("aria-selected"), "false");
      assert.equal(taskRow.getAttribute("aria-selected"), "false");
      assert.equal(eventRow.getAttribute("aria-selected"), "true");
      assert.equal(queryByAccessibleLabel(root, /^Task inspector$/), null);
      assert.match(getByAccessibleLabel(root, /^Event inspector$/).textContent ?? "", /Samples ready/);

      clickTreeItem(root, /^workflow$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(projectRow.getAttribute("aria-selected"), "true");
      assert.equal(taskRow.getAttribute("aria-selected"), "false");
      assert.equal(eventRow.getAttribute("aria-selected"), "false");
      assert.match(getByAccessibleLabel(root, /^Project inspector$/).textContent ?? "", /workflow/);
      assert.equal(queryByAccessibleLabel(root, /^Task inspector$/), null);
      assert.equal(queryByAccessibleLabel(root, /^Event inspector$/), null);
    },
  );
});

test("timeline task and event selection stay mutually exclusive while the inspector follows the active object", async () => {
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

      const analysisTaskBar = getByRole(
        root,
        "button",
        /^Task analysis-run spans Samples ready to Analysis complete on the event axis with semantic binding nodes at Samples ready and Analysis complete$/,
      );
      analysisTaskBar.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      await waitForHandlers();

      assert.equal(analysisTaskBar.getAttribute("aria-pressed"), "true");
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(queryByAccessibleLabel(root, /^Event inspector$/), null);
      assert.match(getByAccessibleLabel(root, /^Task inspector$/).textContent ?? "", /analysis-run/);

      const samplesReadyRail = getByRole(root, "button", /^Select event Samples ready$/);
      samplesReadyRail.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(samplesReadyRail.getAttribute("aria-pressed"), "true");
      assert.equal(analysisTaskBar.getAttribute("aria-pressed"), "false");
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "false",
      );
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(queryByAccessibleLabel(root, /^Task inspector$/), null);
      assert.match(getByAccessibleLabel(root, /^Event inspector$/).textContent ?? "", /Samples ready/);

      analysisTaskBar.dispatchEvent(new window.MouseEvent("click", { bubbles: true }));
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(analysisTaskBar.getAttribute("aria-pressed"), "true");
      assert.equal(samplesReadyRail.getAttribute("aria-pressed"), "false");
      assert.equal(
        getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.equal(queryByAccessibleLabel(root, /^Event inspector$/), null);
      assert.match(getByAccessibleLabel(root, /^Task inspector$/).textContent ?? "", /analysis-run/);
    },
  );
});

test("timeline placement issues stay inside task navigation and inspector panes", async () => {
  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () =>
          projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/workflow",
            projectRoot: "/tmp/fixture-spielgantt/workflow",
            events: ["Samples ready", "Analysis complete"],
            workflow: workflowFixture({
              project_root: "/tmp/fixture-spielgantt/workflow",
              events: ["Samples ready", "Analysis complete"],
              event_nodes: workflowEventNodes(["Samples ready", "Analysis complete"]),
              tasks: [
                workflowTaskFixture("sample-prep", {
                  effective_anchors: workflowAnchors(null, "Samples ready"),
                  valid_ends_at_targets: ["Samples ready", "Analysis complete"],
                }),
              ],
            }),
            tasks: [
              taskFixture("sample-prep", {
                path: "/tmp/fixture-spielgantt/workflow/sample-prep",
                endsAt: "Samples ready",
              }),
            ],
          }),
        onboardProjectAction: async () =>
          projectFixture({
            selectedPath: "/tmp/fixture-spielgantt/workflow",
            projectRoot: "/tmp/fixture-spielgantt/workflow",
            events: ["Samples ready", "Analysis complete"],
            workflow: workflowFixture({
              project_root: "/tmp/fixture-spielgantt/workflow",
              events: ["Samples ready", "Analysis complete"],
              event_nodes: workflowEventNodes(["Samples ready", "Analysis complete"]),
              tasks: [
                workflowTaskFixture("sample-prep", {
                  effective_anchors: workflowAnchors(null, "Samples ready"),
                  valid_ends_at_targets: ["Samples ready", "Analysis complete"],
                }),
              ],
            }),
            tasks: [
              taskFixture("sample-prep", {
                path: "/tmp/fixture-spielgantt/workflow/sample-prep",
                endsAt: "Samples ready",
              }),
            ],
          })
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      assert.throws(() =>
        getByRole(root, "button", /^Task sample-prep has undetermined timeline placement$/),
      );
      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );
      assert.match(
        getByAccessibleLabel(root, /^Task inspector$/).textContent ?? "",
        /Complete this task's event-axis workflow placement\./,
      );
    },
  );
});

test("task context menu exposes row actions through the sidebar menu", async () => {
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

      const taskRow = getByRole(root, "treeitem", /^sample-prep$/);
      taskRow.dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "menu", /^Task actions for sample-prep$/i));
      const openTaskFolderItem = getByRole(root, "menuitem", /^Open task folder sample-prep$/i);
      assert.ok(openTaskFolderItem);
      assert.ok(getByRole(root, "menuitem", /^Rename task sample-prep$/i));
      assert.ok(getByRole(root, "menuitem", /^Delete task sample-prep$/i));
    },
  );
});

test("sidebar leaf action menu dispatches task and event actions", async () => {
  const openedTaskFolders: string[] = [];

  await withRenderedShell(
    {
      projectLifecycle: {
        openSelectedProject: async () => workflowProject(),
        onboardProjectAction: async () => workflowProject()
      },
      desktop: {
        pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow",
        openTaskFolderAction: async (path) => {
          openedTaskFolders.push(path);
        }
      }
    },
    async ({ root, window }) => {
      await openProjectFromProjectsMenu(root);

      getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Open task folder sample-prep$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.deepEqual(openedTaskFolders, ["/tmp/fixture-spielgantt/workflow/sample-prep"]);

      getByRole(root, "treeitem", /^Samples ready$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      getByRole(root, "menuitem", /^Rename event Samples ready$/i).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "button", /^Rename Event$/));
    },
  );
});

test("timeline task context menu exposes delete task actions", async () => {
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

      const timelineTask = getByRole(
        root,
        "button",
        /^Task analysis-run spans Samples ready to Analysis complete on the event axis/,
      );
      timelineTask.dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 240, clientY: 120 }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "menu", /^Task actions for analysis-run$/i));
      assert.ok(getByRole(root, "menuitem", /^Open task folder analysis-run$/i));
      assert.ok(getByRole(root, "menuitem", /^Rename task analysis-run$/i));
      assert.ok(getByRole(root, "menuitem", /^Delete task analysis-run$/i));
    },
  );
});

test("task context menus close when clicking outside the menu", async () => {
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

      getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Open task folder sample-prep$/i));

      getByRole(root, "tree", /^Projects$/).dispatchEvent(
        new window.MouseEvent("mousedown", { bubbles: true, clientX: 260, clientY: 12 }),
      );
      await waitForHandlers();

      assert.deepEqual(menuItemNames(root), []);

      const timelineTask = getByRole(
        root,
        "button",
        /^Task analysis-run spans Samples ready to Analysis complete on the event axis/,
      );
      timelineTask.dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 240, clientY: 120 }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Open task folder analysis-run$/i));

      getByRole(root, "tree", /^Projects$/).dispatchEvent(
        new window.MouseEvent("mousedown", { bubbles: true, clientX: 260, clientY: 12 }),
      );
      await waitForHandlers();

      assert.deepEqual(menuItemNames(root), []);
    },
  );
});

test("project and event sidebar menus close when clicking outside the menu", async () => {
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
      const clickOutsideMenu = async () => {
        getByRole(root, "tree", /^Projects$/).dispatchEvent(
          new window.MouseEvent("mousedown", { bubbles: true, clientX: 260, clientY: 12 }),
        );
        await waitForHandlers();
      };

      getByRole(root, "button", /^Add project$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Start from scratch$/));

      await clickOutsideMenu();
      assert.deepEqual(menuItemNames(root), []);

      await openProjectFromProjectsMenu(root);

      getByRole(root, "button", /^Project actions for workflow$/).dispatchEvent(
        new window.MouseEvent("click", { bubbles: true }),
      );
      await waitForHandlers();
      assert.ok(getProjectMenuItem(root, /^Refresh project workflow$/i));

      await clickOutsideMenu();
      assert.deepEqual(menuItemNames(root), []);

      getByRole(root, "treeitem", /^Samples ready$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      assert.ok(getByRole(root, "menuitem", /^Rename event Samples ready$/i));

      await clickOutsideMenu();
      assert.deepEqual(menuItemNames(root), []);
    },
  );
});

test("right-clicking a task row selects that task before opening its action menu", async () => {
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

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.ok(getByRole(root, "menuitem", /^Open task folder sample-prep$/i));
      assert.ok(getByRole(root, "menuitem", /^Rename task sample-prep$/i));
    },
  );
});

test("keyboard context-menu access opens task row actions with the task selected", async () => {
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

      clickTreeItem(root, /^Samples ready$/);
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "treeitem", /^sample-prep$/).dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "ContextMenu", bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^sample-prep$/).getAttribute("aria-selected"),
        "true",
      );
      assert.equal(
        getByRole(root, "treeitem", /^Samples ready$/).getAttribute("aria-selected"),
        "false",
      );
      assert.ok(getByRole(root, "menuitem", /^Open task folder sample-prep$/i));
      assert.ok(getByRole(root, "menuitem", /^Rename task sample-prep$/i));
    },
  );
});

test("keyboard context-menu access opens event row actions with the event selected", async () => {
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

      clickTreeItem(root, /^sample-prep$/);
      await waitForHandlers();
      await waitForHandlers();

      getByRole(root, "treeitem", /^Analysis complete$/).dispatchEvent(
        new window.KeyboardEvent("keydown", { key: "ContextMenu", bubbles: true }),
      );
      await waitForHandlers();
      await waitForHandlers();

      assert.equal(
        getByRole(root, "treeitem", /^Analysis complete$/).getAttribute("aria-selected"),
        "true",
      );
      assert.ok(getByRole(root, "menuitem", /^Rename event Analysis complete$/i));
      assert.ok(getByRole(root, "menuitem", /^Delete event Analysis complete$/i));
    },
  );
});

test("event context menu exposes event actions through accessible menu items", async () => {
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

      getByRole(root, "treeitem", /^Samples ready$/).dispatchEvent(
        new window.MouseEvent("contextmenu", { bubbles: true, clientX: 24, clientY: 12 }),
      );
      await waitForHandlers();

      assert.ok(getByRole(root, "menu", /^Event actions for Samples ready$/i));
      assert.ok(getByRole(root, "menuitem", /^Rename event Samples ready$/i));
      assert.ok(getByRole(root, "menuitem", /^Delete event Samples ready$/i));
    },
  );
});
