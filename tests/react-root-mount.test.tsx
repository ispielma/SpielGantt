import assert from "node:assert/strict";
import test from "node:test";
import {
  getByRole,
  openProjectFromProjectsMenu,
  projectFixture,
  renderTestShell,
  workflowEventNodes,
  workflowFixture,
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

test("React root mounts the existing shell and keeps the projects tree reachable", async () => {
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["start", "finished"],
    eventReferences: [
      { id: "start", referencedTaskIds: [], blockerTaskIds: [], blockedTaskIds: [] },
      { id: "finished", referencedTaskIds: [], blockerTaskIds: [], blockedTaskIds: [] },
    ],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      event_nodes: workflowEventNodes(["start", "finished"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [],
    }),
  });

  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project
    },
    desktop: {
      pickProjectFolder: async () => project.projectRoot
    }
  });

  try {
    assert.ok(getByRole(root, "tree", /^Projects$/));
    assert.ok(getByRole(root, "button", /^Add project$/));
    assert.match(root.textContent ?? "", /Choose a SpielGantt project folder to begin\./);
    assert.doesNotMatch(root.textContent ?? "", /Scientific Workflow Planner/i);

    await openProjectFromProjectsMenu(root);

    assert.equal(
      getByRole(root, "treeitem", /^workflow$/).getAttribute("aria-selected"),
      "true",
    );
    assert.ok(getByRole(root, "button", /^Select event start$/));
    assert.ok(getByRole(root, "button", /^Select event finished$/));
    assert.doesNotMatch(
      getByAccessibleLabel(root, "Project workspace").textContent ?? "",
      /No tasks discovered/,
    );
  } finally {
    await cleanup();
  }
});
