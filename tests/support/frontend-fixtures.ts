import type { OpenProjectResult } from "../../src/main.ts";
import type { RememberedProjectRecord } from "../../src/remembered-projects.ts";

export function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, resolve, reject };
}

export function readyHealth() {
  return {
    appName: "spielgantt",
    version: "1.0.0-rc.1",
    core: "ready",
  };
}

export function rememberedProjectRecord(
  projectPath: string,
  _taskNames: string[] = [],
  lastSelectedTaskName: string | null = null,
  expanded = false,
  _eventNames: string[] = [],
): RememberedProjectRecord {
  return {
    projectPath,
    expanded,
    lastOpenedAt: "2026-06-14T12:00:00.000Z",
    lastSelectedTaskName,
  };
}

export function taskFixture(
  id: string,
  overrides: Partial<OpenProjectResult["tasks"][number]> = {},
): OpenProjectResult["tasks"][number] {
  const defaultProjectRoot = "/tmp/fixture-spielgantt";
  const projectRoot =
    overrides.path?.endsWith(`/${id}`)
      ? overrides.path.slice(0, -1 * (`/${id}`).length)
      : defaultProjectRoot;
  return {
    id,
    path: `${projectRoot}/${id}`,
    projectRelativePath: id,
    dependencies: [],
    blocks: [],
    dependencyReferences: [],
    dependencyTargets: [],
    status: null,
    readmeContent: "",
    readmeVersion: "",
    ...overrides,
  };
}

export function workflowTaskFixture(
  id: string,
  overrides: Partial<NonNullable<OpenProjectResult["workflow"]>["tasks"][number]> = {},
): NonNullable<OpenProjectResult["workflow"]>["tasks"][number] {
  const dependencyReferences = overrides.dependency_references ?? [];
  const endsAtReference = overrides.ends_at_reference ?? null;
  const validationDiagnostics = overrides.validation_diagnostics ?? [];
  const effectiveAnchors = overrides.effective_anchors ?? {
    upstream: null,
    downstream: null,
    diagnostics: [],
  };
  const placementMessages =
    overrides.placement_messages
    ?? Array.from(new Set([...effectiveAnchors.diagnostics, ...validationDiagnostics]));
  const determinationStatus =
    overrides.determination_status
    ?? (effectiveAnchors.upstream && effectiveAnchors.downstream
      ? "fully_determined"
      : "undetermined");
  const placementStatus =
    overrides.placement_status
    ?? (validationDiagnostics.length > 0
      ? "diagnostic"
      : determinationStatus === "fully_determined" && placementMessages.length === 0
        ? "ready"
        : "incomplete");
  return {
    id,
    determination_status: determinationStatus,
    placement_ready: placementStatus === "ready",
    placement_status: placementStatus,
    placement_messages: placementMessages,
    dependency_references: dependencyReferences,
    ends_at_reference: endsAtReference,
    effective_anchors: effectiveAnchors,
    valid_dependency_targets: [],
    valid_ends_at_targets: [],
    unresolved_references: [],
    invalid_references: [],
    validation_diagnostics: validationDiagnostics,
    ...overrides,
  };
}

export function workflowAnchors(
  upstream: string | null,
  downstream: string | null,
  diagnostics: string[] = [],
): NonNullable<OpenProjectResult["workflow"]>["tasks"][number]["effective_anchors"] {
  return { upstream, downstream, diagnostics };
}

type WorkflowEventNode = NonNullable<OpenProjectResult["workflow"]>["event_nodes"][number];

export function workflowEventNodes(
  eventIds: string[],
  boundaryRoles: Partial<Record<string, WorkflowEventNode["boundary_role"]>> = {},
): WorkflowEventNode[] {
  return eventIds.map((id, chart_order) => ({
    id,
    boundary_role: boundaryRoles[id] ?? "ordinary",
    chart_order,
    placement_ready: true,
    placement_status: "ready",
    placement_messages: [],
  }));
}

export function workflowFixture(
  overrides: Partial<NonNullable<OpenProjectResult["workflow"]>> = {},
): NonNullable<OpenProjectResult["workflow"]> {
  return {
    schema_version: 1,
    project_root: "/tmp/fixture-spielgantt",
    events: [],
    event_nodes: [],
    edges: [],
    validation: { valid: true, diagnostics: [] },
    tasks: [],
    ...overrides,
  };
}

export function projectFixture(
  overrides: Partial<OpenProjectResult> = {},
): OpenProjectResult {
  const projectRoot =
    "projectRoot" in overrides ? overrides.projectRoot ?? null : "/tmp/fixture-spielgantt";
  return {
    selectedPath: overrides.selectedPath ?? projectRoot ?? "/tmp/fixture-spielgantt",
    projectRoot,
    valid: true,
    issues: [],
    projectReadmeContent: "",
    projectReadmeVersion: "",
    events: [],
    eventReferences: [],
    workflow: null,
    tasks: [],
    ...overrides,
  };
}

export function alignmentPlan() {
  return {
    operations: [
      {
        renameTaskFolder: {
          taskId: "Calibrate Laser",
          from: "/tmp/fixture-spielgantt/analysis notes",
          to: "/tmp/fixture-spielgantt/Calibrate Laser",
        },
      },
    ],
    preflightIssues: [],
  };
}

export function alignmentCollisionPlan() {
  return {
    operations: alignmentPlan().operations,
    preflightIssues: [
      {
        targetAlreadyExists: "/tmp/fixture-spielgantt/Calibrate Laser",
      },
    ],
  };
}
