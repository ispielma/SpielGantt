import {
  ConfirmationDialog,
  NameMutationDialog,
} from "./dialog-primitives.tsx";
import type {
  DeleteEventDialogState,
  EventDialogState,
  RenameEventDialogState,
} from "./shell-dialogs.tsx";

interface EventDialogProps {
  state: EventDialogState;
  onEventNameChange: (eventName: string) => void;
  onCancel: () => void;
  onSubmit: (eventName: string) => void;
}

export function CreateEventDialog({
  state,
  onEventNameChange,
  onCancel,
  onSubmit,
}: EventDialogProps) {
  if (!state.open) {
    return null;
  }

  return (
    <NameMutationDialog
      dialogTestId="new-event-dialog-overlay"
      formTestId="new-event-form"
      formClassName="new-event-form"
      inputTestId="new-event-name"
      errorTestId="new-event-error"
      titleId="new-event-dialog-title"
      errorId="new-event-error"
      title="New Event"
      fieldName="event-name"
      inputLabel="Event name"
      value={state.eventName}
      errorMessage={state.errorMessage}
      submitting={state.submitting}
      cancelTestId="cancel-new-event"
      submitTestId="create-new-event"
      submitLabel="Create Event"
      submittingLabel="Creating Event..."
      onNameChange={onEventNameChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}

interface RenameEventDialogProps {
  state: RenameEventDialogState;
  onEventNameChange: (eventName: string) => void;
  onCancel: () => void;
  onSubmit: (eventName: string) => void;
}

export function RenameEventDialog({
  state,
  onEventNameChange,
  onCancel,
  onSubmit,
}: RenameEventDialogProps) {
  if (!state.open) {
    return null;
  }

  return (
    <NameMutationDialog
      dialogTestId="rename-event-dialog-overlay"
      formTestId="rename-event-form"
      formClassName="new-event-form"
      inputTestId="rename-event-name"
      errorTestId="rename-event-error"
      titleId="rename-event-dialog-title"
      errorId="rename-event-error"
      title="Rename Event"
      fieldName="event-name"
      inputLabel="Event name"
      value={state.eventName}
      errorMessage={state.errorMessage}
      submitting={state.submitting}
      cancelTestId="cancel-rename-event"
      submitTestId="rename-selected-event"
      submitLabel="Rename Event"
      submittingLabel="Renaming Event..."
      referencedTaskIds={state.referencedTaskIds}
      onNameChange={onEventNameChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}

interface DeleteEventDialogProps {
  state: DeleteEventDialogState;
  onCancel: () => void;
  onSubmit: () => void;
}

export function DeleteEventDialog({ state, onCancel, onSubmit }: DeleteEventDialogProps) {
  if (!state.open) {
    return null;
  }

  const blocked = state.referencedTaskIds.length > 0;

  return (
    <ConfirmationDialog
      dialogTestId="delete-event-dialog-overlay"
      formTestId="delete-event-form"
      formClassName="delete-event-form"
      titleId="delete-event-dialog-title"
      title="Delete Event"
      errorId="delete-event-error"
      errorTestId="delete-event-error"
      errorMessage={state.errorMessage}
      message={blocked
        ? `Cannot delete event '${state.eventName}' because it is referenced by tasks.`
        : `Delete event '${state.eventName}'?`}
      referencedTaskIds={state.referencedTaskIds}
      cancelTestId="cancel-delete-event"
      cancelLabel={blocked ? "Close" : "Cancel"}
      actions={blocked
        ? []
        : [
            {
              testId: "confirm-delete-event",
              label: "Delete Event",
              submittingLabel: "Deleting Event...",
              isSubmitting: state.submitting,
              disabled: state.submitting,
              type: "submit",
            },
          ]}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}
