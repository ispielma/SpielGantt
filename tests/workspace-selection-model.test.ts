import assert from "node:assert/strict";
import test from "node:test";

import { buildWorkspaceSelectionModel } from "../src/workspace-selection-model.ts";
import {
  projectFixture,
  taskFixture,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
} from "./support/frontend-shell.ts";
import type { OperationState, ProjectReadmeEdit, TaskEdit } from "../src/shell-types.ts";

function commandCapabilities(operationState: OperationState = { status: "idle" }) {
  return {
    operationState,
    taskEndsAtControlState: {
      errorMessage: null,
      submitting: false,
      selectedEventId: "",
    },
    taskDependencyControlState: {
      selectedBlockerId: "",
    },
    onEditProjectReadme: (_edit: ProjectReadmeEdit) => {},
    onEditTask: (_taskId: string, _edit: TaskEdit) => {},
    onSetTaskEndsAt: (_taskId: string, _eventId: string | null, _clear: boolean) => {},
    onSelectTaskEndsAtEvent: (_eventId: string) => {},
    onCreateTaskEndEvent: (_taskId: string) => {},
    onSelectTaskDependencyBlocker: (_blockerId: string) => {},
    onAddDependency: (_taskId: string, _blockerId: string) => {},
    onRemoveDependency: (_taskId: string, _blockerId: string) => {},
  };
}

test("workspace selection model derives timeline and inspector props from project selection", () => {
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    projectReadmeContent: "# Workflow\n",
    projectReadmeVersion: "project-v1",
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
        workflowTaskFixture("sample-prep", {
          effective_anchors: workflowAnchors(null, "Samples ready"),
        }),
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
        dependencyTargets: [{ id: "Samples ready", kind: "event" }],
        readmeContent: "# Analysis\n",
        readmeVersion: "v1",
      }),
    ],
  });
  const commands = commandCapabilities({ status: "error", message: "backend refused edit" });

  const projectSelection = buildWorkspaceSelectionModel({
    projectState: { status: "valid", project },
    selectedTaskId: null,
    selectedEventId: null,
    commands,
  });
  assert.equal(projectSelection.projectRoot, "/tmp/fixture-spielgantt/workflow");
  assert.equal(projectSelection.inspector.kind, "project");
  assert.equal(projectSelection.inspector.props.project, project);
  assert.equal(projectSelection.inspector.props.onEditProjectReadme, commands.onEditProjectReadme);
  assert.deepEqual(projectSelection.timelineProps.selectedTaskId, null);
  assert.deepEqual(projectSelection.timelineProps.selectedEventId, null);

  const taskSelection = buildWorkspaceSelectionModel({
    projectState: { status: "valid", project },
    selectedTaskId: "analysis-run",
    selectedEventId: null,
    commands,
  });
  assert.equal(taskSelection.inspector.kind, "task");
  assert.equal(taskSelection.inspector.props.task.id, "analysis-run");
  assert.equal(taskSelection.inspector.props.workflowTask?.id, "analysis-run");
  assert.deepEqual(taskSelection.inspector.props.validEndEventTargets, [
    "Samples ready",
    "Analysis complete",
  ]);
  assert.equal(taskSelection.inspector.props.operationState, commands.operationState);
  assert.equal(taskSelection.inspector.props.onEditTask, commands.onEditTask);
  assert.equal(taskSelection.inspector.props.onAddDependency, commands.onAddDependency);
  assert.equal(taskSelection.timelineProps.selectedTaskId, "analysis-run");

  const eventSelection = buildWorkspaceSelectionModel({
    projectState: { status: "valid", project },
    selectedTaskId: "analysis-run",
    selectedEventId: "Samples ready",
    commands,
  });
  assert.equal(eventSelection.inspector.kind, "event");
  assert.deepEqual(eventSelection.inspector.props, {
    eventId: "Samples ready",
    blockerTaskIds: ["sample-prep"],
    blockedTaskIds: ["analysis-run"],
    placementStatus: "ready",
  });
  assert.equal(eventSelection.timelineProps.selectedEventId, "Samples ready");
  assert.equal(eventSelection.timelineProps.selectedTaskId, null);

  const staleSelection = buildWorkspaceSelectionModel({
    projectState: { status: "valid", project },
    selectedTaskId: "missing-task",
    selectedEventId: "missing-event",
    commands,
  });
  assert.equal(staleSelection.inspector.kind, "project");
  assert.equal(staleSelection.timelineProps.selectedTaskId, null);
  assert.equal(staleSelection.timelineProps.selectedEventId, null);

  const emptySelection = buildWorkspaceSelectionModel({
    projectState: { status: "none" },
    selectedTaskId: "analysis-run",
    selectedEventId: "Samples ready",
    commands,
  });
  assert.equal(emptySelection.projectRoot, null);
  assert.equal(emptySelection.inspector.kind, "empty");
  assert.equal(emptySelection.inspector.eyebrow, "No project open");
  assert.equal(
    emptySelection.inspector.title,
    "Choose a SpielGantt project folder to begin.",
  );
  assert.equal(emptySelection.timelineProps, null);
});
