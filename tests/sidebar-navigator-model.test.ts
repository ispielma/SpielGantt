import assert from "node:assert/strict";
import test from "node:test";

import {
  buildSidebarNavigatorModel,
  type SidebarNavigatorNode,
} from "../src/sidebar-tree-model.ts";
import {
  projectFixture,
  rememberedProjectRecord,
  taskFixture,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
} from "./support/frontend-shell.ts";

function nodeSummary(node: SidebarNavigatorNode): unknown {
  return {
    kind: node.meta.kind,
    label: node.label,
    accessibleName: node.meta.accessibleName,
    selected: node.selected,
    expanded: node.expanded,
    diagnostic: node.meta.diagnostic ?? false,
    children: node.children?.map(nodeSummary) ?? [],
  };
}

test("sidebar navigator normalizes projects and routes navigation, keyboard, context menu, and command intents", () => {
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["start", "analysis-complete"],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "analysis-complete"],
      event_nodes: workflowEventNodes(["start", "analysis-complete"]).map((event) => ({
        ...event,
        placement_ready: false,
        placement_status: "incomplete",
        placement_messages: [`event '${event.id}' is incomplete in the backend placement contract`],
      })),
      tasks: [
        workflowTaskFixture("analysis-run", {
          effective_anchors: workflowAnchors(null, null),
        }),
      ],
    }),
    tasks: [
      taskFixture("sample-prep", {
        path: "/tmp/fixture-spielgantt/workflow/sample-prep",
      }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
      }),
    ],
  });

  const navigator = buildSidebarNavigatorModel({
    records: [
      rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", [], null, true),
      rememberedProjectRecord(
        "/tmp/fixture-spielgantt/moved-workflow",
        ["stale-task"],
        "stale-task",
        true,
        ["stale-event"],
      ),
    ],
    projectState: { status: "valid", project },
    selectedTaskId: "analysis-run",
    selectedEventId: null,
    sidebarState: {
      refreshInFlightProjectPaths: [],
      scanFailures: {
        "/tmp/fixture-spielgantt/moved-workflow": "folder is missing",
      },
      projectContents: {},
    },
    sectionExpandedState: {},
  });

  assert.deepEqual(navigator.nodes.map(nodeSummary), [
    {
      kind: "project",
      label: "workflow",
      accessibleName: "workflow",
      selected: false,
      expanded: true,
      diagnostic: false,
      children: [
        {
          kind: "section",
          label: "Tasks",
          accessibleName: "Tasks",
          selected: false,
          expanded: true,
          diagnostic: false,
          children: [
            {
              kind: "task",
              label: "sample-prep",
              accessibleName: "sample-prep",
              selected: false,
              expanded: undefined,
              diagnostic: false,
              children: [],
            },
            {
              kind: "task",
              label: "analysis-run",
              accessibleName: "analysis-run",
              selected: true,
              expanded: undefined,
              diagnostic: true,
              children: [],
            },
          ],
        },
        {
          kind: "section",
          label: "Events",
          accessibleName: "Events",
          selected: false,
          expanded: true,
          diagnostic: false,
          children: [
            {
              kind: "event",
              label: "start",
              accessibleName: "start",
              selected: false,
              expanded: undefined,
              diagnostic: true,
              children: [],
            },
            {
              kind: "event",
              label: "analysis-complete",
              accessibleName: "analysis-complete",
              selected: false,
              expanded: undefined,
              diagnostic: true,
              children: [],
            },
          ],
        },
      ],
    },
    {
      kind: "project",
      label: "moved-workflow",
      accessibleName: "moved-workflow",
      selected: false,
      expanded: false,
      diagnostic: false,
      children: [],
    },
  ]);

  const taskNode = navigator.findNodeByLabel("analysis-run");
  const eventsSection = navigator.findNodeByLabel("Events");
  const missingProject = navigator.findNodeByLabel("moved-workflow");
  assert.ok(taskNode);
  assert.ok(eventsSection);
  assert.ok(missingProject);

  assert.deepEqual(navigator.activationIntentForValue(taskNode.value), {
    kind: "select-task",
    projectPath: "/tmp/fixture-spielgantt/workflow",
    taskId: "analysis-run",
  });
  assert.deepEqual(navigator.keyboardIntentForNode(eventsSection.value, "ArrowLeft"), {
    kind: "set-section-expanded",
    value: eventsSection.value,
    expanded: false,
  });
  assert.deepEqual(navigator.contextMenuIntentForValue(taskNode.value, "keyboard"), {
    kind: "open-task-context-menu",
    projectPath: "/tmp/fixture-spielgantt/workflow",
    taskId: "analysis-run",
    source: "keyboard",
  });
  assert.deepEqual(
    navigator.projectCommandIntentForValue(missingProject.value, {
      kind: "remove-remembered-project",
    }),
    {
      kind: "run-project-command",
      projectPath: "/tmp/fixture-spielgantt/moved-workflow",
      command: { kind: "remove-remembered-project" },
    },
  );
});

test("sidebar diagnostics consume backend placement statuses instead of deriving from anchors", () => {
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["start", "checkpoint", "finished"],
    workflow: workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "checkpoint", "finished"],
      event_nodes: [
        {
          id: "start",
          boundary_role: "start_boundary",
          chart_order: 0,
          placement_ready: true,
          placement_status: "ready",
          placement_messages: [],
        },
        {
          id: "checkpoint",
          boundary_role: "ordinary",
          chart_order: 1,
          placement_ready: true,
          placement_status: "ready",
          placement_messages: [],
        },
        {
          id: "finished",
          boundary_role: "finish_boundary",
          chart_order: 2,
          placement_ready: true,
          placement_status: "ready",
          placement_messages: [],
        },
      ],
      tasks: [
        workflowTaskFixture("analysis-run", {
          determination_status: "undetermined",
          placement_ready: true,
          placement_status: "ready",
          placement_messages: [],
          effective_anchors: workflowAnchors(null, null, [
            "legacy anchor diagnostic that should not drive sidebar warnings",
          ]),
        }),
      ],
    }),
    tasks: [
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
      }),
    ],
  });

  const navigator = buildSidebarNavigatorModel({
    records: [rememberedProjectRecord("/tmp/fixture-spielgantt/workflow", [], null, true)],
    projectState: { status: "valid", project },
    selectedTaskId: null,
    selectedEventId: null,
    sidebarState: {
      refreshInFlightProjectPaths: [],
      scanFailures: {},
      projectContents: {},
    },
    sectionExpandedState: {},
  });

  assert.equal(
    navigator.findNodeByLabel("analysis-run")?.meta.diagnostic,
    false,
    "task sidebar warnings should follow backend placement_status, not local anchor diagnostics",
  );
  assert.equal(
    navigator.findNodeByLabel("checkpoint")?.meta.diagnostic,
    false,
    "event sidebar warnings should follow backend event placement_status, not local anchor scans",
  );
});
