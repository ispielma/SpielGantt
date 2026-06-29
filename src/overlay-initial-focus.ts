import type { DeleteProjectDialogState } from "./delete-project-dialog.tsx";
import type { DeleteTaskDialogState } from "./delete-task-dialog.tsx";
import type {
  DeleteEventDialogState,
  EventDialogState,
  NewProjectDialogState,
  RenameEventDialogState,
  RenameTaskDialogState,
  TaskDialogState,
} from "./shell-dialogs.tsx";
import type { TaskFromFolderDialogState } from "./task-from-folder-dialog.tsx";

export interface OverlayFocusState {
  newProjectDialogState: NewProjectDialogState;
  taskDialogState: TaskDialogState;
  taskFromFolderDialogState: TaskFromFolderDialogState;
  renameTaskDialogState: RenameTaskDialogState;
  eventDialogState: EventDialogState;
  renameEventDialogState: RenameEventDialogState;
  deleteEventDialogState: DeleteEventDialogState;
  deleteTaskDialogState: DeleteTaskDialogState;
  deleteProjectDialogState: DeleteProjectDialogState;
}

type FocusScheduler = (focus: () => void) => void;

function selectorForOverlayFocus(state: OverlayFocusState): string | null {
  if (state.newProjectDialogState.open) {
    return "[data-testid='new-project-name']";
  }
  if (state.taskDialogState.open) {
    return "[data-testid='new-task-name']";
  }
  if (state.taskFromFolderDialogState.open) {
    return "[data-testid='task-from-folder-candidate']";
  }
  if (state.renameTaskDialogState.open) {
    return "[data-testid='rename-task-name']";
  }
  if (state.eventDialogState.open) {
    return "[data-testid='new-event-name']";
  }
  if (state.renameEventDialogState.open) {
    return "[data-testid='rename-event-name']";
  }
  if (state.deleteEventDialogState.open) {
    return state.deleteEventDialogState.referencedTaskIds.length
      ? "[data-testid='cancel-delete-event']"
      : "[data-testid='confirm-delete-event']";
  }
  if (state.deleteTaskDialogState.open) {
    return "[data-testid='cancel-delete-task']";
  }
  if (state.deleteProjectDialogState.open) {
    return "[data-testid='delete-project-confirmation']";
  }
  return null;
}

export class OverlayInitialFocusController {
  constructor(
    private readonly root: HTMLElement,
    private readonly scheduleFocus: FocusScheduler = (focus) => {
      const ownerWindow = root.ownerDocument.defaultView;
      if (ownerWindow) {
        ownerWindow.setTimeout(focus, 0);
      } else {
        focus();
      }
    },
  ) {}

  focusInitialControl(state: OverlayFocusState): void {
    const selector = selectorForOverlayFocus(state);
    if (!selector) {
      return;
    }

    this.scheduleFocus(() => {
      this.root.querySelector<HTMLElement>(selector)?.focus();
    });
  }
}
