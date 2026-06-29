import type { FormEvent, ReactNode } from "react";

import { Button, Group, Modal, Stack, Text, TextInput } from "@mantine/core";

interface NameMutationDialogProps {
  dialogTestId: string;
  formTestId: string;
  formClassName: string;
  inputTestId: string;
  errorTestId: string;
  titleId: string;
  errorId: string;
  title: string;
  fieldName: string;
  inputLabel: string;
  value: string;
  errorMessage: string | null;
  submitting: boolean;
  cancelTestId: string;
  submitTestId: string;
  submitLabel: string;
  submittingLabel: string;
  referencedTaskIds?: string[];
  onNameChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: (value: string) => void;
}

interface ConfirmationDialogAction {
  testId: string;
  label: string;
  submittingLabel?: string;
  isSubmitting?: boolean;
  disabled?: boolean;
  type?: "button" | "submit";
  variant?: "default";
  color?: "red";
  onClick?: () => void;
}

interface ConfirmationDialogProps {
  dialogTestId: string;
  titleId: string;
  title: string;
  formTestId?: string;
  formClassName?: string;
  errorId: string;
  errorTestId: string;
  errorMessage: string | null;
  message: ReactNode;
  details?: ReactNode;
  referencedTaskIds?: string[];
  cancelTestId: string;
  cancelLabel?: string;
  cancelDisabled?: boolean;
  actions: ConfirmationDialogAction[];
  trapFocus?: boolean;
  onCancel: () => void;
  onSubmit?: () => void;
}

export function modalTitle(title: string, id: string) {
  return (
    <Text component="span" id={id} size="xl" fw={650}>
      {title}
    </Text>
  );
}

function confirmationActions(actions: ConfirmationDialogAction[]) {
  return actions.map((action) => (
    <Button
      key={action.testId}
      data-testid={action.testId}
      type={action.type ?? "button"}
      variant={action.variant}
      color={action.color}
      onClick={action.onClick}
      disabled={action.disabled}
    >
      {action.isSubmitting && action.submittingLabel ? action.submittingLabel : action.label}
    </Button>
  ));
}

export function ConfirmationDialog(props: ConfirmationDialogProps) {
  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    props.onSubmit?.();
  };
  const content = (
    <Stack gap="md">
      <Text>{props.message}</Text>
      {props.details}
      {renderReferencedTasks(props.referencedTaskIds ?? [])}
      {props.errorMessage ? (
        <Text
          c="red"
          id={props.errorId}
          className="dialog-error"
          data-testid={props.errorTestId}
          role="alert"
          aria-live="polite"
          size="sm"
        >
          {props.errorMessage}
        </Text>
      ) : null}
      <Group justify="flex-end">
        <Button
          data-testid={props.cancelTestId}
          type="button"
          variant="default"
          onClick={props.onCancel}
          disabled={props.cancelDisabled}
        >
          {props.cancelLabel ?? "Cancel"}
        </Button>
        {confirmationActions(props.actions)}
      </Group>
    </Stack>
  );

  return (
    <div data-testid={props.dialogTestId}>
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        trapFocus={props.trapFocus}
        title={modalTitle(props.title, props.titleId)}
      >
        {props.formTestId ? (
          <form
            aria-describedby={props.errorMessage ? props.errorId : undefined}
            className={props.formClassName}
            data-testid={props.formTestId}
            onSubmit={handleSubmit}
          >
            {content}
          </form>
        ) : (
          content
        )}
      </Modal>
    </div>
  );
}

export function NameMutationDialog(props: NameMutationDialogProps) {
  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const FormDataCtor = event.currentTarget.ownerDocument.defaultView?.FormData ?? FormData;
    const formData = new FormDataCtor(event.currentTarget);
    props.onSubmit(String(formData.get(props.fieldName) ?? "").trim());
  };

  return (
    <div data-testid={props.dialogTestId}>
      <Modal
        centered
        opened
        onClose={() => {}}
        withCloseButton={false}
        withinPortal={false}
        title={modalTitle(props.title, props.titleId)}
      >
        <form
          aria-describedby={props.errorMessage ? props.errorId : undefined}
          className={props.formClassName}
          data-testid={props.formTestId}
          onSubmit={handleSubmit}
        >
          <Stack gap="md">
            <TextInput
              id={props.inputTestId}
              data-testid={props.inputTestId}
              name={props.fieldName}
              autoComplete="off"
              defaultValue={props.value}
              label={props.inputLabel}
              aria-label={props.inputLabel}
              onChange={(event) => props.onNameChange(event.currentTarget.value)}
            />
            {props.errorMessage ? (
              <Text
                c="red"
                id={props.errorId}
                className="dialog-error"
                data-testid={props.errorTestId}
                aria-live="polite"
                role="alert"
                size="sm"
              >
                {props.errorMessage}
              </Text>
            ) : null}
            {renderReferencedTasks(props.referencedTaskIds ?? [])}
            <Group justify="flex-end">
              <Button
                data-testid={props.cancelTestId}
                type="button"
                variant="default"
                onClick={props.onCancel}
              >
                Cancel
              </Button>
              <Button
                data-testid={props.submitTestId}
                type="submit"
                disabled={props.submitting}
              >
                {props.submitting ? props.submittingLabel : props.submitLabel}
              </Button>
            </Group>
          </Stack>
        </form>
      </Modal>
    </div>
  );
}

export function renderReferencedTasks(taskIds: string[]) {
  if (taskIds.length === 0) {
    return null;
  }

  return (
    <section aria-label="Referencing tasks" className="event-reference-list">
      <Text component="h3" fw={600} size="sm">
        Referencing tasks
      </Text>
      <ul className="event-reference-items">
        {taskIds.map((taskId) => (
          <li key={taskId}>{taskId}</li>
        ))}
      </ul>
    </section>
  );
}
