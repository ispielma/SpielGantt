import type { RememberedProjectRecord } from "./remembered-projects.ts";
import type { OpenProjectResult, OperationState, ProjectOpenState } from "./shell-types.ts";

export const noProjectOpen: ProjectOpenState = { status: "none" };
export const idleOperation: OperationState = { status: "idle" };

export function projectStateFromResult(project: OpenProjectResult): ProjectOpenState {
  return project.valid
    ? { status: "valid", project }
    : { status: "invalid", project };
}

export function replaceRememberedProjectRecord(
  records: RememberedProjectRecord[],
  record: RememberedProjectRecord,
): RememberedProjectRecord[] {
  const nextRecords = [...records];
  const existingIndex = nextRecords.findIndex(
    (candidate) => candidate.projectPath === record.projectPath,
  );

  if (existingIndex === -1) {
    nextRecords.push({ ...record });
    return nextRecords;
  }

  nextRecords[existingIndex] = { ...record };
  return nextRecords;
}

export function eventReferencingTaskIdsFromProject(
  project: OpenProjectResult,
  eventId: string,
): string[] {
  return (
    project.eventReferences.find((eventReferences) => eventReferences.id === eventId)
      ?.referencedTaskIds ?? []
  );
}
