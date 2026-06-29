import assert from "node:assert/strict";
import test from "node:test";
import { Window } from "happy-dom";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

import {
  buildTimelineViewProps,
  type TimelineWorkflow,
  type TimelineTask,
} from "../src/timeline.ts";
import { TimelineView, type TimelineViewProps } from "../src/timeline-panel.tsx";
import {
  accessibleName,
  elementRole,
  getByRole,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
} from "./support/frontend-shell.ts";

const eventTask: TimelineTask = {
  id: "sample-prep",
  dependencies: ["START"],
  endsAt: "MOT",
};

const eventWorkflow: TimelineWorkflow = {
  ...workflowFixture({
    project_root: "/tmp/fixture-spielgantt/workflow",
    events: ["START", "MOT"],
    event_nodes: workflowEventNodes(["START", "MOT"], {
      START: "start_boundary",
      MOT: "finish_boundary",
    }),
    tasks: [
      workflowTaskFixture("sample-prep", {
        dependency_references: [{ id: "START", kind: "event", valid: true }],
        ends_at_reference: { id: "MOT", valid: true },
        determination_status: "fully_determined",
        effective_anchors: workflowAnchors("START", "MOT"),
        valid_ends_at_targets: ["START", "MOT"],
      }),
    ],
  }),
};

function renderTimelineView(props: TimelineViewProps): HTMLElement {
  const window = new Window();
  const root = window.document.createElement("div");
  root.innerHTML = renderToStaticMarkup(React.createElement(TimelineView, props));
  return root;
}

function roleNames(root: Element, role: ReturnType<typeof elementRole>): string[] {
  return Array.from(root.querySelectorAll("*"))
    .filter((element) => elementRole(element) === role)
    .map(accessibleName);
}

function timelineVisualContractElements(root: Element, testId: string): Element[] {
  return Array.from(root.querySelectorAll(`[data-testid='${testId}']`));
}

function timelineRailGuideCount(root: Element): number {
  return timelineVisualContractElements(root, "timeline-event-rail-guide").length;
}

function eventRailNames(root: Element): string[] {
  return roleNames(root, "button")
    .filter((name) => name.startsWith("Select event "))
    .map((name) => name.replace(/^Select event /, ""));
}

test("empty project timeline renders start and finish boundary rails from workflow event nodes", () => {
  const timeline = buildTimelineViewProps(
    [],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      event_nodes: workflowEventNodes(["Kickoff", "Released"], {
        Kickoff: "start_boundary",
        Released: "finish_boundary",
      }),
      tasks: [],
    }),
  );

  const root = renderTimelineView(timeline);

  assert.equal(timeline.hasEvents, true);
  assert.deepEqual(eventRailNames(root), ["Kickoff", "Released"]);
  assert.equal(timelineRailGuideCount(root), 2);
});

test("timeline renders disconnected ordinary events between boundary rails", () => {
  const timeline = buildTimelineViewProps(
    [],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "finished", "samples ready"],
      event_nodes: workflowEventNodes(["start", "finished", "samples ready"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [],
    }),
  );

  const root = renderTimelineView(timeline);

  assert.deepEqual(eventRailNames(root), ["start", "samples ready", "finished"]);
  assert.equal(timelineRailGuideCount(root), 3);
});

test("timeline ignores legacy workflow events missing from backend event nodes", () => {
  const timeline = buildTimelineViewProps(
    [],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "legacy only", "finished"],
      event_nodes: workflowEventNodes(["start", "finished"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [],
    }),
  );

  const root = renderTimelineView(timeline);

  assert.deepEqual(eventRailNames(root), ["start", "finished"]);
  assert.equal(timelineRailGuideCount(root), 2);
});

test("timeline uses workflow chart order for constrained ordinary events inside boundaries", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "process-data",
        dependencies: ["samples ready"],
        endsAt: "data ready",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "finished", "data ready", "samples ready"],
      event_nodes: workflowEventNodes(["start", "finished", "samples ready", "data ready"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("process-data", {
          dependency_references: [
            { id: "samples ready", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "data ready", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("samples ready", "data ready"),
        }),
      ],
    }),
  );

  const root = renderTimelineView(timeline);

  assert.deepEqual(eventRailNames(root), [
    "start",
    "samples ready",
    "data ready",
    "finished",
  ]);
  assert.equal(
    timeline.placedTasks.find((task) => task.taskId === "process-data")?.gridColumn,
    "2 / 3",
  );
  assert.equal(timeline.layoutConflicts.length, 0);
});

test("timeline keeps the finish boundary out of the middle of a partially built chart", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "prepare-samples",
        dependencies: ["start"],
        endsAt: "samples ready",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["start", "finished", "samples ready"],
      event_nodes: workflowEventNodes(["start", "finished", "samples ready"], {
        start: "start_boundary",
        finished: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("prepare-samples", {
          dependency_references: [{ id: "start", kind: "event", valid: true }],
          ends_at_reference: { id: "samples ready", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("start", "samples ready"),
        }),
      ],
    }),
  );

  const root = renderTimelineView(timeline);

  assert.deepEqual(eventRailNames(root), ["start", "samples ready", "finished"]);
  assert.equal(timeline.placedTasks[0]?.gridColumn, "1 / 2");
  assert.ok(
    getByRole(
      root,
      "button",
      /^Task prepare-samples spans start to samples ready on the event axis/,
    ),
  );
});

test("event timeline renders static bars without drag controls", () => {
  const timeline = buildTimelineViewProps([eventTask], eventWorkflow);

  assert.equal(timeline.hasEvents, true);
  assert.equal(timeline.placedTasks[0]?.taskId, "sample-prep");
  assert.equal(timeline.placedTasks[0]?.layoutState, "event-span");
  assert.equal(timeline.placedTasks[0]?.startEventId, "START");
  assert.equal(timeline.placedTasks[0]?.endEventId, "MOT");
});

test("event timeline treats events as boundary lines around intervals", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "analyze-results",
        dependencies: ["Fluorescence captured"],
        endsAt: "Analysis complete",
      },
      {
        id: "literature-review",
        dependencies: ["Workflow started"],
        endsAt: "Protocol selected",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: [
        "Workflow started",
        "Protocol selected",
        "Samples ready",
        "Fluorescence captured",
        "Analysis complete",
      ],
      event_nodes: workflowEventNodes([
        "Workflow started",
        "Protocol selected",
        "Samples ready",
        "Fluorescence captured",
        "Analysis complete",
      ]),
      tasks: [
        workflowTaskFixture("analyze-results", {
          dependency_references: [
            { id: "Fluorescence captured", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Analysis complete", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors(
            "Fluorescence captured",
            "Analysis complete",
          ),
        }),
        workflowTaskFixture("literature-review", {
          dependency_references: [
            { id: "Workflow started", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Protocol selected", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Workflow started", "Protocol selected"),
        }),
      ],
    }),
  );

  assert.equal(
    timeline.eventRailColumns.split(" ").length,
    4,
    "five event lines should define four visible intervals",
  );
  assert.doesNotMatch(
    timeline.eventRailColumns,
    /1fr/,
    "event intervals should use content-sized columns instead of uniform fractions",
  );
  assert.equal(
    timeline.placedTasks.find((task) => task.taskId === "analyze-results")?.gridColumn,
    "4 / 5",
    "a task should span from the Fluorescence captured line to the Analysis complete line",
  );
  assert.equal(
    timeline.placedTasks.find((task) => task.taskId === "literature-review")?.gridColumn,
    "1 / 2",
    "a task should span from Workflow started to Protocol selected",
  );
});

test("event timeline sizes each interval to the task labels it contains", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "A",
        dependencies: ["START"],
        endsAt: "MID",
      },
      {
        id: "long-analysis-task",
        dependencies: ["MID"],
        endsAt: "DONE",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "MID", "DONE"],
      event_nodes: workflowEventNodes(["START", "MID", "DONE"], {
        START: "start_boundary",
        DONE: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("A", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "MID", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "MID"),
        }),
        workflowTaskFixture("long-analysis-task", {
          dependency_references: [{ id: "MID", kind: "event", valid: true }],
          ends_at_reference: { id: "DONE", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("MID", "DONE"),
        }),
      ],
    }),
  );

  const intervalWidths = timeline.eventRailColumns
    .split(" ")
    .map((column) => Number.parseInt(column, 10));

  assert.equal(intervalWidths.length, 2);
  assert.ok(
    intervalWidths[1] > intervalWidths[0],
    `the longer task label should make its interval wider: ${timeline.eventRailColumns}`,
  );
});

test("timeline bars expose task statuses for status color styling", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "blocked task",
        dependencies: ["START"],
        endsAt: "DONE",
        status: "blocked",
      },
      {
        id: "finished task",
        dependencies: ["START"],
        endsAt: "DONE",
        status: "done",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "DONE"],
      event_nodes: workflowEventNodes(["START", "DONE"], {
        START: "start_boundary",
        DONE: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("blocked task", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "DONE", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "DONE"),
        }),
        workflowTaskFixture("finished task", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "DONE", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "DONE"),
        }),
      ],
    }),
  );

  const root = renderTimelineView(timeline);
  assert.equal(
    getByRole(root, "button", /^Task blocked task spans START to DONE on the event axis/)
      .getAttribute("data-task-status"),
    "blocked",
  );
  assert.equal(
    getByRole(root, "button", /^Task finished task spans START to DONE on the event axis/)
      .getAttribute("data-task-status"),
    "done",
  );
});

test("timeline bars mark backend workflow diagnostics without hiding the task", () => {
  const diagnostic =
    "task 'analyze-results' is done but depends on event 'FLUORESCENCE' reached by blocked task 'collect-fluorescence'";
  const timeline = buildTimelineViewProps(
    [
      {
        id: "collect-fluorescence",
        dependencies: ["START"],
        endsAt: "FLUORESCENCE",
        status: "blocked",
      },
      {
        id: "analyze-results",
        dependencies: ["FLUORESCENCE"],
        endsAt: "DONE",
        status: "done",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "FLUORESCENCE", "DONE"],
      event_nodes: workflowEventNodes(["START", "FLUORESCENCE", "DONE"], {
        START: "start_boundary",
        DONE: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("collect-fluorescence", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "FLUORESCENCE", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "FLUORESCENCE"),
        }),
        workflowTaskFixture("analyze-results", {
          dependency_references: [{ id: "FLUORESCENCE", kind: "event", valid: true }],
          ends_at_reference: { id: "DONE", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("FLUORESCENCE", "DONE", [diagnostic]),
          validation_diagnostics: [diagnostic],
        }),
      ],
    }),
  );

  const analyzeResults = timeline.placedTasks.find((task) => task.taskId === "analyze-results");
  assert.equal(analyzeResults?.layoutState, "event-span");
  assert.deepEqual(analyzeResults?.warningDiagnostics, [diagnostic]);

  const root = renderTimelineView(timeline);
  const warningBar = getByRole(
    root,
    "button",
    /^Task analyze-results spans FLUORESCENCE to DONE on the event axis.*Warning:/,
  );
  assert.match(warningBar.textContent ?? "", /analyze-results/);
});

test("event timeline orders rows by when placed tasks occur", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "analyze-results",
        dependencies: ["Fluorescence captured"],
        endsAt: "Analysis complete",
      },
      {
        id: "collect-fluorescence",
        dependencies: ["Samples ready"],
        endsAt: "Fluorescence captured",
      },
      {
        id: "literature-review",
        dependencies: ["Workflow started"],
        endsAt: "Protocol selected",
      },
      {
        id: "prepare-samples",
        dependencies: ["Protocol selected"],
        endsAt: "Samples ready",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: [
        "Workflow started",
        "Protocol selected",
        "Samples ready",
        "Fluorescence captured",
        "Analysis complete",
      ],
      event_nodes: workflowEventNodes([
        "Workflow started",
        "Protocol selected",
        "Samples ready",
        "Fluorescence captured",
        "Analysis complete",
      ]),
      tasks: [
        workflowTaskFixture("analyze-results", {
          dependency_references: [
            { id: "Fluorescence captured", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Analysis complete", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors(
            "Fluorescence captured",
            "Analysis complete",
          ),
        }),
        workflowTaskFixture("collect-fluorescence", {
          dependency_references: [
            { id: "Samples ready", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Fluorescence captured", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Samples ready", "Fluorescence captured"),
        }),
        workflowTaskFixture("literature-review", {
          dependency_references: [
            { id: "Workflow started", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Protocol selected", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Workflow started", "Protocol selected"),
        }),
        workflowTaskFixture("prepare-samples", {
          dependency_references: [
            { id: "Protocol selected", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "Samples ready", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Protocol selected", "Samples ready"),
        }),
      ],
    }),
  );

  assert.deepEqual(
    timeline.placedTasks.map((task) => task.taskId),
    [
      "literature-review",
      "prepare-samples",
      "collect-fluorescence",
      "analyze-results",
    ],
  );
});

test("event timeline keeps task blockers above their dependents in the same interval", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "a-dependent",
        dependencies: ["z-blocker", "START"],
        endsAt: "MOT",
      },
      {
        id: "z-blocker",
        dependencies: ["START"],
        endsAt: "MOT",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "MOT"],
      event_nodes: workflowEventNodes(["START", "MOT"], {
        START: "start_boundary",
        MOT: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("a-dependent", {
          dependency_references: [
            { id: "z-blocker", kind: "task", valid: true },
            { id: "START", kind: "event", valid: true },
          ],
          ends_at_reference: { id: "MOT", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "MOT"),
        }),
        workflowTaskFixture("z-blocker", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "MOT", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "MOT"),
        }),
      ],
    }),
  );

  assert.deepEqual(
    timeline.placedTasks.map((task) => task.taskId),
    ["z-blocker", "a-dependent"],
  );
});

test("event timeline places task dependency chains between event anchors", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "make chart",
        dependencies: ["Workflow started"],
        endsAt: null,
      },
      {
        id: "literature-review",
        dependencies: ["make chart"],
        endsAt: "Protocol selected",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["Workflow started", "Protocol selected"],
      event_nodes: workflowEventNodes(["Workflow started", "Protocol selected"]),
      tasks: [
        workflowTaskFixture("make chart", {
          dependency_references: [
            { id: "Workflow started", kind: "event", valid: true },
          ],
          ends_at_reference: null,
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Workflow started", "Protocol selected"),
        }),
        workflowTaskFixture("literature-review", {
          dependency_references: [
            { id: "make chart", kind: "task", valid: true },
          ],
          ends_at_reference: { id: "Protocol selected", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Workflow started", "Protocol selected"),
        }),
      ],
    }),
  );

  assert.deepEqual(
    timeline.placedTasks.map((task) => [
      task.taskId,
      task.gridColumn,
      task.inlineStartPercent,
      task.inlineSizePercent,
    ]),
    [
      ["make chart", "1 / 2", 0, 50],
      ["literature-review", "1 / 2", 50, 50],
    ],
    "task-to-task dependencies should split their shared event interval by dependency order",
  );
});

test("event timeline uses backend-provided effective anchors for semantic placement", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "make chart",
        dependencies: ["Workflow started"],
        endsAt: null,
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["Workflow started", "Protocol selected"],
      event_nodes: workflowEventNodes(["Workflow started", "Protocol selected"]),
      tasks: [
        workflowTaskFixture("make chart", {
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("Workflow started", "Protocol selected"),
        }),
      ],
    }),
  );

  assert.deepEqual(
    timeline.placedTasks.map((task) => [task.taskId, task.gridColumn]),
    [["make chart", "1 / 2"]],
    "timeline placement should consume Rust-owned effective anchors instead of walking raw references",
  );
});

test("timeline renders supplied workflow diagnostics", () => {
  const diagnostics = [
    "task 'analysis-run' depends on missing task or event id 'MISSING-BLOCKER'",
    "task 'analysis-run' ends_at references missing event id 'MISSING-EVENT'",
  ];
  const timeline = buildTimelineViewProps(
    [
      {
        id: "analysis-run",
        dependencies: ["START"],
        endsAt: "MOT",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "MOT"],
      event_nodes: workflowEventNodes(["START", "MOT"], {
        START: "start_boundary",
        MOT: "finish_boundary",
      }),
      validation: { valid: false, diagnostics: [] },
      tasks: [
        workflowTaskFixture("analysis-run", {
          dependency_references: [
            {
              id: "MISSING-BLOCKER",
              kind: "task",
              valid: false,
              diagnostic:
                "task 'analysis-run' depends on missing task or event id 'MISSING-BLOCKER'",
            },
          ],
          ends_at_reference: {
            id: "MISSING-EVENT",
            valid: false,
            diagnostic:
              "task 'analysis-run' ends_at references missing event id 'MISSING-EVENT'",
          },
          valid_ends_at_targets: ["START", "MOT"],
          validation_diagnostics: diagnostics,
          effective_anchors: workflowAnchors(null, null, diagnostics),
        }),
      ],
    }),
  );

  assert.equal(timeline.layoutConflicts[0]?.taskId, "analysis-run");
  assert.match(timeline.layoutConflicts[0]?.message ?? "", /MISSING-BLOCKER/);
  assert.match(timeline.layoutConflicts[0]?.message ?? "", /MISSING-EVENT/);
});

test("timeline connector cues follow workflow event references", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "sample-prep",
        dependencies: [],
        endsAt: null,
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "MOT"],
      event_nodes: workflowEventNodes(["START", "MOT"], {
        START: "start_boundary",
        MOT: "finish_boundary",
      }),
      tasks: [
        workflowTaskFixture("sample-prep", {
          dependency_references: [{ id: "START", kind: "event", valid: true }],
          ends_at_reference: { id: "MOT", valid: true },
          determination_status: "fully_determined",
          effective_anchors: workflowAnchors("START", "MOT"),
          valid_ends_at_targets: ["START", "MOT"],
        }),
      ],
    }),
  );

  assert.equal(timeline.placedTasks[0]?.startConnectorEventId, "START");
  assert.equal(timeline.placedTasks[0]?.endConnectorEventId, "MOT");
});

test("timeline view exposes selectable event rails and task-event relationships", () => {
  const root = renderTimelineView({
    hasEvents: true,
    events: ["START", "Samples ready", "Analysis complete"],
    eventRailColumns: "repeat(2, minmax(120px, 1fr))",
    placedTasks: [
      {
        taskId: "prepare-samples",
        layoutState: "event-span",
        dependencyIds: ["START"],
        startEventId: "START",
        endEventId: "Samples ready",
        gridColumn: "1 / 2",
        startConnectorEventId: "START",
        endConnectorEventId: "Samples ready",
      },
      {
        taskId: "incubate",
        layoutState: "event-span",
        dependencyIds: ["START"],
        startEventId: "START",
        endEventId: "Samples ready",
        gridColumn: "1 / 2",
        startConnectorEventId: "START",
        endConnectorEventId: null,
      },
    ],
    unanchoredTasks: [],
    layoutConflicts: [],
    selectedEventId: "Samples ready",
  });

  assert.equal(getByRole(root, "button", /^Select event START$/).getAttribute("aria-pressed"), "false");
  assert.equal(
    getByRole(root, "button", /^Select event Samples ready$/).getAttribute("aria-pressed"),
    "true",
  );
  assert.ok(getByRole(root, "button", /^Select event Analysis complete$/));
  assert.ok(
    getByRole(
      root,
      "button",
      /^Task prepare-samples spans START to Samples ready on the event axis with semantic binding nodes at START and Samples ready$/,
    ),
  );
  assert.ok(
    getByRole(
      root,
      "button",
      /^Task incubate spans START to Samples ready on the event axis with a semantic binding node at START$/,
    ),
  );
  const buttonNames = roleNames(root, "button");
  assert.equal(buttonNames.filter((name) => name.startsWith("Select event ")).length, 3);
  assert.equal(buttonNames.filter((name) => name.startsWith("Task ")).length, 2);
  assert.equal(
    timelineVisualContractElements(root, "timeline-event-connector").length,
    0,
    "same-interval tasks should not render the connector visual contract marker",
  );
  assert.equal(
    timelineVisualContractElements(root, "dependency-count").length,
    0,
    "same-interval task labels should not render the dependency-count visual contract marker",
  );
  assert.equal(
    timelineRailGuideCount(root),
    3,
    "the rail-guide visual contract should render one guide per event line",
  );
  assert.doesNotMatch(
    getByRole(
      root,
      "button",
      /^Task prepare-samples spans START to Samples ready on the event axis with semantic binding nodes at START and Samples ready$/,
    ).textContent ?? "",
    /•|1/,
  );
});

test("timeline preserves long event names in visible labels and accessible controls", () => {
  const root = renderTimelineView({
    hasEvents: true,
    events: [
      "Workflow started",
      "Protocol selected",
      "Samples ready",
      "Fluorescence captured",
      "Analysis complete",
    ],
    eventRailColumns: "repeat(4, minmax(120px, 1fr))",
    placedTasks: [],
    unanchoredTasks: [],
    layoutConflicts: [],
  });

  for (const eventId of [
    "Workflow started",
    "Protocol selected",
    "Samples ready",
    "Fluorescence captured",
    "Analysis complete",
  ]) {
    assert.ok(getByRole(root, "button", new RegExp(`^Select event ${eventId}$`)));
    assert.match(root.textContent ?? "", new RegExp(eventId));
  }
});

test("timeline keeps unresolved task diagnostics out of the global chart", () => {
  const timeline = buildTimelineViewProps(
    [
      {
        id: "sample-prep",
        dependencies: [],
        endsAt: null,
      },
      {
        id: "analysis-run",
        dependencies: ["START"],
        endsAt: "MOT",
      },
    ],
    workflowFixture({
      project_root: "/tmp/fixture-spielgantt/workflow",
      events: ["START", "MOT"],
      event_nodes: workflowEventNodes(["START", "MOT"], {
        START: "start_boundary",
        MOT: "finish_boundary",
      }),
      validation: { valid: false, diagnostics: [] },
      tasks: [
        workflowTaskFixture("sample-prep"),
        workflowTaskFixture("analysis-run", {
          dependency_references: [
            {
              id: "MISSING-BLOCKER",
              kind: "task",
              valid: false,
              diagnostic:
                "task 'analysis-run' depends on missing task or event id 'MISSING-BLOCKER'",
            },
          ],
          ends_at_reference: {
            id: "MISSING-EVENT",
            valid: false,
            diagnostic:
              "task 'analysis-run' ends_at references missing event id 'MISSING-EVENT'",
          },
          valid_ends_at_targets: ["START", "MOT"],
          validation_diagnostics: [
            "task 'analysis-run' depends on missing task or event id 'MISSING-BLOCKER'",
            "task 'analysis-run' ends_at references missing event id 'MISSING-EVENT'",
          ],
        }),
      ],
    }),
  );

  assert.equal(timeline.unanchoredTasks[0]?.taskId, "sample-prep");
  assert.equal(timeline.layoutConflicts[0]?.taskId, "analysis-run");

  const root = renderTimelineView(timeline);
  assert.throws(() =>
    getByRole(root, "group", /^Timeline placement issues$/),
    "timeline diagnostics should stay in sidebar markers and task inspector panes, not global chart cards",
  );
  assert.throws(
    () => getByRole(root, "button", /^Task sample-prep has undetermined timeline placement$/),
    "unanchored task diagnostics should not render as top-level timeline action cards",
  );
  assert.throws(
    () => getByRole(root, "button", /^Task analysis-run has timeline placement diagnostics$/),
    "layout conflict diagnostics should not render as top-level timeline action cards",
  );
});

test("timeline view makes narrow-width overflow explicit without hiding full event names", () => {
  const root = renderTimelineView({
    hasEvents: true,
    events: [
      "Workflow started",
      "Protocol selected",
      "Samples ready",
      "Fluorescence captured",
      "Analysis complete",
    ],
    eventRailColumns: "repeat(4, minmax(120px, 1fr))",
    placedTasks: [
      {
        taskId: "collect-fluorescence",
        layoutState: "event-span",
        dependencyIds: ["Samples ready"],
        startEventId: "Samples ready",
        endEventId: "Fluorescence captured",
        gridColumn: "3 / 4",
        startConnectorEventId: "Samples ready",
        endConnectorEventId: "Fluorescence captured",
      },
    ],
    unanchoredTasks: [],
    layoutConflicts: [],
  });

  assert.doesNotMatch(root.textContent ?? "", /Scroll to later events/);
  assert.equal(
    timelineRailGuideCount(root),
    5,
    "the rail-guide visual contract should preserve all event lines at narrow width",
  );
  assert.ok(getByRole(root, "button", /^Select event Fluorescence captured$/));
  assert.ok(
    getByRole(
      root,
      "button",
      /^Task collect-fluorescence spans Samples ready to Fluorescence captured on the event axis with semantic binding nodes at Samples ready and Fluorescence captured$/,
    ),
  );
});
