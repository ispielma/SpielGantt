import {
  Button,
  Group,
  NativeSelect,
  SimpleGrid,
  Stack,
  Text,
  Textarea,
} from "@mantine/core";
import { IconCalendarPlus } from "@tabler/icons-react";
import type { ChangeEvent, FormEvent } from "react";

import {
  InspectorSection,
  InspectorToken,
  InspectorTokenList,
  TaskOperationAlert,
} from "./inspector-components.tsx";
import {
  taskStatusOptions,
  type OperationState,
  type ProjectTask,
  type TaskEdit,
  type TaskStatus,
} from "./shell-types.ts";
import { TaskPlacementWarning } from "./task-placement-indicator.tsx";
import type { TimelineWorkflowTask } from "./timeline.ts";
import type {
  TaskDependencyControlState,
  TaskEndsAtControlState,
} from "./task-inspector-workflow.ts";

interface TaskEditDraft {
  status: TaskStatus;
  readmeContent: string;
}

export function taskEditDraftFromTask(task: ProjectTask): TaskEditDraft {
  return {
    status: task.status ?? "unblocked",
    readmeContent: task.readmeContent,
  };
}

export function taskEditFromDraft(task: ProjectTask, draft: TaskEditDraft): TaskEdit {
  return {
    status: draft.status,
    readmeContent: draft.readmeContent,
    expectedReadmeVersion: task.readmeVersion,
  };
}

function taskStatusFromSelectValue(value: string): TaskStatus {
  return taskStatusOptions.find((option) => option.value === value)?.value ?? "unblocked";
}

export function TaskMetadataSection(props: {
  draft: TaskEditDraft;
  onPersistDraft: (nextDraft: TaskEditDraft) => Promise<void> | void;
}) {
  const { draft, onPersistDraft } = props;

  return (
    <SimpleGrid className="task-status-editor" cols={1} data-testid="edit-task-form" spacing="sm">
      <NativeSelect
        id="edit-status"
        data-testid="edit-status"
        name="status"
        label="Status"
        aria-label="Task status"
        value={draft.status}
        onChange={(event) => {
          void onPersistDraft({
            ...draft,
            status: taskStatusFromSelectValue(event.currentTarget.value),
          });
        }}
      >
        {taskStatusOptions.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </NativeSelect>
    </SimpleGrid>
  );
}

export function TaskOperationStatus(props: { operationState: OperationState }) {
  if (props.operationState.status !== "error") {
    return null;
  }

  return (
    <TaskOperationAlert testId="task-action-status">
      {props.operationState.message}
    </TaskOperationAlert>
  );
}

export function TaskReadmeSection(props: {
  draft: TaskEditDraft;
  onChangeDraft: (nextDraft: TaskEditDraft) => void;
  onPersistDraft: (nextDraft: TaskEditDraft) => Promise<void> | void;
}) {
  const { draft, onChangeDraft, onPersistDraft } = props;

  return (
    <div className="task-readme-editor" data-testid="task-readme-section">
      <Textarea
        id="edit-readme"
        data-testid="edit-readme"
        name="readme"
        label="README"
        classNames={{
          input: "task-readme-control-input",
          root: "task-readme-control",
          wrapper: "task-readme-control-wrapper",
        }}
        minRows={6}
        spellCheck={false}
        aria-label="Task README"
        value={draft.readmeContent}
        onInput={(event) => {
          onChangeDraft({ ...draft, readmeContent: event.currentTarget.value });
        }}
        onBlur={(event) => {
          void onPersistDraft({ ...draft, readmeContent: event.currentTarget.value });
        }}
      />
    </div>
  );
}

export function TaskEndsAtSection(props: {
  task: ProjectTask;
  validEndEventTargets: string[];
  taskEndsAtControlState: TaskEndsAtControlState;
  onSetTaskEndsAt: (taskId: string, eventId: string | null, clear: boolean) => Promise<void> | void;
  onSelectTaskEndsAtEvent: (eventId: string) => void;
  onCreateEndEvent: (taskId: string) => void;
}) {
  const {
    task,
    validEndEventTargets,
    taskEndsAtControlState,
    onSetTaskEndsAt,
    onSelectTaskEndsAtEvent,
    onCreateEndEvent,
  } = props;
  const currentEndsAt = task.endsAt ?? "";
  const selectedEndsAt = taskEndsAtControlState.selectedEventId || currentEndsAt;

  const handleEndsAtSelection = (event: ChangeEvent<HTMLSelectElement>) => {
    const eventId = event.currentTarget.value;
    onSelectTaskEndsAtEvent(eventId);
    void onSetTaskEndsAt(task.id, eventId || null, eventId === "");
  };

  return (
    <Stack className="task-ends-at-control" data-testid="task-ends-at-control" gap="xs">
      <Group align="flex-end" gap="xs">
        <NativeSelect
          id="task-ends-at-event-id"
          data-testid="task-ends-at-event-id"
          name="event-id"
          label="End event"
          aria-label="End at event"
          value={selectedEndsAt}
          disabled={taskEndsAtControlState.submitting || validEndEventTargets.length === 0}
          onChange={handleEndsAtSelection}
        >
          <option value="">No end event set</option>
          {validEndEventTargets.map((eventId) => (
            <option key={eventId} value={eventId}>
              Event: {eventId}
            </option>
          ))}
        </NativeSelect>
        <Button
          aria-label="Create event and set as end event"
          disabled={taskEndsAtControlState.submitting}
          leftSection={<IconCalendarPlus aria-hidden="true" focusable={false} size={16} />}
          type="button"
          variant="default"
          onClick={() => onCreateEndEvent(task.id)}
        >
          New
        </Button>
      </Group>
      {taskEndsAtControlState.errorMessage ? (
        <Text
          aria-live="polite"
          c="yellow.9"
          data-testid="task-ends-at-error"
          size="sm"
        >
          {taskEndsAtControlState.errorMessage}
        </Text>
      ) : null}
    </Stack>
  );
}

export function TaskDependenciesSection(props: {
  task: ProjectTask;
  workflowTask: TimelineWorkflowTask | null;
  validEndEventTargets: string[];
  taskEndsAtControlState: TaskEndsAtControlState;
  taskDependencyControlState: TaskDependencyControlState;
  onSetTaskEndsAt: (taskId: string, eventId: string | null, clear: boolean) => Promise<void> | void;
  onSelectTaskEndsAtEvent: (eventId: string) => void;
  onCreateEndEvent: (taskId: string) => void;
  onSelectTaskDependencyBlocker: (blockerId: string) => void;
  onAddDependency: (taskId: string, blockerId: string) => Promise<void> | void;
  onRemoveDependency: (taskId: string, blockerId: string) => Promise<void> | void;
}) {
  const {
    task,
    workflowTask,
    validEndEventTargets,
    taskEndsAtControlState,
    taskDependencyControlState,
    onSetTaskEndsAt,
    onSelectTaskEndsAtEvent,
    onCreateEndEvent,
    onSelectTaskDependencyBlocker,
    onAddDependency,
    onRemoveDependency,
  } = props;
  const dependencyTargets = task.dependencyTargets;
  const blockedTasks = task.blocks.map((candidate) => candidate.id);
  const dependencyTargetGroups = [
    {
      label: "Task targets",
      options: dependencyTargets.filter((target) => target.kind === "task"),
    },
    {
      label: "Event targets",
      options: dependencyTargets.filter((target) => target.kind === "event"),
    },
  ].filter((group) => group.options.length > 0);
  const selectedBlockerId = dependencyTargets.some(
    (target) => target.id === taskDependencyControlState.selectedBlockerId,
  )
    ? taskDependencyControlState.selectedBlockerId
    : "";

  const handleDependencySubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onAddDependency(task.id, selectedBlockerId);
  };

  return (
    <InspectorSection legend="Dependencies" testId="task-dependencies-section">
      <Stack gap="sm">
        <TaskPlacementWarning workflowTask={workflowTask} />
        <form data-testid="add-dependency-form" onSubmit={handleDependencySubmit}>
          <Group align="flex-end" className="dependency-add-row" gap="xs">
            <NativeSelect
              id="dependency-blocker-id"
              data-testid="dependency-blocker-id"
              name="blocker-id"
              label="Dependency target"
              classNames={{
                input: "dependency-blocker-control-input",
                root: "dependency-blocker-control",
                wrapper: "dependency-blocker-control-wrapper",
              }}
              aria-label="Dependency target"
              value={selectedBlockerId}
              disabled={dependencyTargets.length === 0}
              onChange={(event) => {
                onSelectTaskDependencyBlocker(event.currentTarget.value);
              }}
            >
              {dependencyTargetGroups.length > 0 ? (
                <>
                  <option value="">Choose dependency target</option>
                  {dependencyTargetGroups.map((group) => (
                    <optgroup key={group.label} label={group.label}>
                      {group.options.map((target) => (
                        <option key={`${target.kind}:${target.id}`} value={target.id}>
                          {target.kind === "task" ? `Task: ${target.id}` : `Event: ${target.id}`}
                        </option>
                      ))}
                    </optgroup>
                  ))}
                </>
              ) : (
                <option value="">No valid task or event targets</option>
              )}
            </NativeSelect>
            <Button
              data-testid="add-dependency"
              type="submit"
              aria-label="Add dependency target to selected task"
              disabled={dependencyTargets.length === 0}
            >
              Add
            </Button>
          </Group>
        </form>
        <Stack gap={4}>
          <Text c="dimmed" fw={600} size="xs" tt="uppercase">
            Blockers
          </Text>
          {task.dependencies.length > 0 ? (
          <Stack aria-label="Remove blockers" component="ul" gap="xs" m={0} p={0}>
            {task.dependencies.map((dependency) => (
              <Group className="inspector-token-list-item" component="li" gap="xs" key={dependency}>
                <InspectorToken>{dependency}</InspectorToken>
                <Button
                  type="button"
                  data-testid="remove-dependency"
                  data-blocker-id={dependency}
                  aria-label={`Remove blocker ${dependency}`}
                  variant="default"
                  onClick={() => {
                    void onRemoveDependency(task.id, dependency);
                  }}
                >
                  Remove
                </Button>
              </Group>
            ))}
          </Stack>
          ) : (
            <Text c="dimmed" size="sm">
              None
            </Text>
          )}
        </Stack>
        <TaskEndsAtSection
          task={task}
          validEndEventTargets={validEndEventTargets}
          taskEndsAtControlState={taskEndsAtControlState}
          onSetTaskEndsAt={onSetTaskEndsAt}
          onSelectTaskEndsAtEvent={onSelectTaskEndsAtEvent}
          onCreateEndEvent={onCreateEndEvent}
        />
        <Stack gap={4}>
          <Text c="dimmed" fw={600} size="xs" tt="uppercase">
            Blocks
          </Text>
          <InspectorTokenList items={blockedTasks} />
        </Stack>
      </Stack>
    </InspectorSection>
  );
}
