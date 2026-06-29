import type { OpenProjectResult } from "./shell-types.ts";

export interface MutationWorkflowEffects {
  setActionError: (error: unknown) => void;
  setOperationIdle: () => void;
  render: () => void;
}

export type MutationSuccessDisposition = "close" | "keep-open";

interface DialogMutationState<TSubmitting> {
  errorMessage: string | null;
  submitting: TSubmitting;
}

interface DialogMutationOptions<TState extends DialogMutationState<TSubmitting>, TSubmitting, TResult>
  extends MutationWorkflowEffects {
  getState: () => TState;
  setState: (state: TState) => void;
  close: () => void;
  submitting: TSubmitting;
  idleSubmitting: TSubmitting;
  prepareState?: (state: TState) => TState;
  invoke: () => Promise<TResult>;
  onSuccess: (
    result: TResult,
  ) => Promise<void | MutationSuccessDisposition> | void | MutationSuccessDisposition;
  afterClose?: () => Promise<void> | void;
}

interface ValidationFailureOptions<TState extends { errorMessage: string | null }> {
  getState: () => TState;
  setState: (state: TState) => void;
  render: () => void;
  message: string;
  prepareState?: (state: TState) => TState;
}

interface RefreshMutationOptions {
  projectPath: string;
  project: OpenProjectResult;
  currentProjectRoot: () => string | null;
  refreshProjectState: (project: OpenProjectResult) => void;
  refreshRememberedProjectTasks: (projectPath: string) => Promise<void>;
  treatReturnedProjectRootAsActive?: boolean;
}

export function mutationErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function showMutationValidationFailure<TState extends { errorMessage: string | null }>(
  options: ValidationFailureOptions<TState>,
): void {
  const currentState = options.getState();
  const state = options.prepareState?.(currentState) ?? currentState;
  options.setState({
    ...state,
    errorMessage: options.message,
  });
  options.render();
}

export async function runDialogMutation<
  TState extends DialogMutationState<TSubmitting>,
  TSubmitting,
  TResult,
>(options: DialogMutationOptions<TState, TSubmitting, TResult>): Promise<void> {
  const currentState = options.getState();
  const submittingState = options.prepareState?.(currentState) ?? currentState;
  options.setState({
    ...submittingState,
    errorMessage: null,
    submitting: options.submitting,
  });
  options.render();

  try {
    const result = await options.invoke();
    const disposition = await options.onSuccess(result);
    if (disposition !== "keep-open") {
      options.setOperationIdle();
      options.close();
      await options.afterClose?.();
    }
  } catch (error) {
    options.setState({
      ...options.getState(),
      submitting: options.idleSubmitting,
      errorMessage: mutationErrorMessage(error),
    });
    options.setActionError(error);
  }

  options.render();
}

export async function refreshProjectAfterMutation(
  options: RefreshMutationOptions,
): Promise<void> {
  const activeProjectRoot = options.currentProjectRoot();
  const returnedProjectIsRequestedProject =
    options.treatReturnedProjectRootAsActive === true &&
    options.project.projectRoot === options.projectPath;

  if (activeProjectRoot === options.projectPath || returnedProjectIsRequestedProject) {
    options.refreshProjectState(options.project);
    return;
  }

  await options.refreshRememberedProjectTasks(options.projectPath);
}
