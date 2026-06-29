import {
  CreateEventDialog,
  CreateTaskDialog,
  DeleteEventDialog,
  NewProjectDialog,
  RenameEventDialog,
  RenameTaskDialog,
  TaskFolderAlignmentDialog,
  type DeleteEventDialogState,
  type EventDialogState,
  type NewProjectDialogState,
  type RenameEventDialogState,
  type RenameTaskDialogState,
  type TaskDialogState,
  type TaskFolderAlignmentDialogState,
} from "./shell-dialogs.tsx";
import {
  DeleteProjectDialog,
  type DeleteProjectDialogState,
} from "./delete-project-dialog.tsx";
import { DeleteTaskDialog, type DeleteTaskDialogState } from "./delete-task-dialog.tsx";
import type { TaskDeleteMode } from "./shell-types.ts";
import {
  TaskFromFolderDialog,
  type TaskFromFolderDialogState,
} from "./task-from-folder-dialog.tsx";

export type OverlayAction =
  | { kind: "new-project-name"; value: string }
  | { kind: "choose-new-project-parent" }
  | { kind: "create-task-name"; value: string }
  | { kind: "rename-task-name"; value: string }
  | { kind: "create-event-name"; value: string }
  | { kind: "rename-event-name"; value: string }
  | { kind: "delete-project-confirmation"; value: string }
  | { kind: "cancel-new-project" }
  | { kind: "submit-new-project"; projectName: string }
  | { kind: "cancel-new-task" }
  | { kind: "submit-new-task"; taskName: string }
  | { kind: "cancel-rename-task" }
  | { kind: "submit-rename-task"; taskName: string }
  | { kind: "cancel-new-event" }
  | { kind: "submit-new-event"; eventName: string }
  | { kind: "cancel-rename-event" }
  | { kind: "submit-rename-event"; eventName: string }
  | { kind: "cancel-delete-event" }
  | { kind: "cancel-delete-project" }
  | { kind: "submit-delete-project" }
  | { kind: "submit-delete-event" }
  | { kind: "cancel-delete-task" }
  | { kind: "submit-delete-task"; mode: TaskDeleteMode }
  | { kind: "select-task-from-folder"; folderPath: string }
  | { kind: "cancel-task-from-folder" }
  | { kind: "submit-task-from-folder" }
  | { kind: "cancel-alignment" }
  | { kind: "apply-alignment" };

interface ShellOverlaysProps {
  newProjectDialogState: NewProjectDialogState;
  taskDialogState: TaskDialogState;
  renameTaskDialogState: RenameTaskDialogState;
  eventDialogState: EventDialogState;
  renameEventDialogState: RenameEventDialogState;
  deleteEventDialogState: DeleteEventDialogState;
  deleteProjectDialogState: DeleteProjectDialogState;
  deleteTaskDialogState: DeleteTaskDialogState;
  taskFromFolderDialogState: TaskFromFolderDialogState;
  alignmentDialogState: TaskFolderAlignmentDialogState;
  onAction: (action: OverlayAction) => Promise<void> | void;
}

export function ShellOverlays(props: ShellOverlaysProps) {
  const {
    newProjectDialogState,
    taskDialogState,
    renameTaskDialogState,
    eventDialogState,
    renameEventDialogState,
    deleteEventDialogState,
    deleteProjectDialogState,
    deleteTaskDialogState,
    taskFromFolderDialogState,
    alignmentDialogState,
    onAction,
  } = props;

  return (
    <div>
      <NewProjectDialog
        state={newProjectDialogState}
        onProjectNameChange={(value) => onAction({ kind: "new-project-name", value })}
        onChooseParentDestination={() => onAction({ kind: "choose-new-project-parent" })}
        onCancel={() => onAction({ kind: "cancel-new-project" })}
        onSubmit={(projectName) => onAction({ kind: "submit-new-project", projectName })}
      />
      <CreateTaskDialog
        state={taskDialogState}
        onTaskNameChange={(value) => onAction({ kind: "create-task-name", value })}
        onCancel={() => onAction({ kind: "cancel-new-task" })}
        onSubmit={(taskName) => onAction({ kind: "submit-new-task", taskName })}
      />
      <RenameTaskDialog
        state={renameTaskDialogState}
        onTaskNameChange={(value) => onAction({ kind: "rename-task-name", value })}
        onCancel={() => onAction({ kind: "cancel-rename-task" })}
        onSubmit={(taskName) => onAction({ kind: "submit-rename-task", taskName })}
      />
      <CreateEventDialog
        state={eventDialogState}
        onEventNameChange={(value) => onAction({ kind: "create-event-name", value })}
        onCancel={() => onAction({ kind: "cancel-new-event" })}
        onSubmit={(eventName) => onAction({ kind: "submit-new-event", eventName })}
      />
      <RenameEventDialog
        state={renameEventDialogState}
        onEventNameChange={(value) => onAction({ kind: "rename-event-name", value })}
        onCancel={() => onAction({ kind: "cancel-rename-event" })}
        onSubmit={(eventName) => onAction({ kind: "submit-rename-event", eventName })}
      />
      <DeleteEventDialog
        state={deleteEventDialogState}
        onCancel={() => onAction({ kind: "cancel-delete-event" })}
        onSubmit={() => onAction({ kind: "submit-delete-event" })}
      />
      <DeleteProjectDialog
        state={deleteProjectDialogState}
        onConfirmationTextChange={(value) =>
          onAction({ kind: "delete-project-confirmation", value })
        }
        onCancel={() => onAction({ kind: "cancel-delete-project" })}
        onSubmit={() => onAction({ kind: "submit-delete-project" })}
      />
      <DeleteTaskDialog
        state={deleteTaskDialogState}
        onCancel={() => onAction({ kind: "cancel-delete-task" })}
        onSubmit={(mode) => onAction({ kind: "submit-delete-task", mode })}
      />
      <TaskFromFolderDialog
        state={taskFromFolderDialogState}
        onSelect={(folderPath) => onAction({ kind: "select-task-from-folder", folderPath })}
        onCancel={() => onAction({ kind: "cancel-task-from-folder" })}
        onSubmit={() => onAction({ kind: "submit-task-from-folder" })}
      />
      <TaskFolderAlignmentDialog
        state={alignmentDialogState}
        onCancel={() => onAction({ kind: "cancel-alignment" })}
        onApply={() => onAction({ kind: "apply-alignment" })}
      />
    </div>
  );
}
