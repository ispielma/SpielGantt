import { Window } from "happy-dom";

import {
  mountReactApp,
  type OpenProjectResult,
} from "../../src/main.ts";
import {
  InMemoryRememberedProjectsSettings,
  type RememberedProjectRecord,
} from "../../src/remembered-projects.ts";
import { getByRole, type AccessibleRole } from "./accessibility.ts";
import { resolveTestShellDeps, type TestShellDeps } from "./frontend-shell-deps.ts";

export { InMemoryRememberedProjectsSettings };
export type { OpenProjectResult, RememberedProjectRecord };
export { accessibleName, elementRole, getByRole, interactiveElements } from "./accessibility.ts";
export type { AccessibleRole } from "./accessibility.ts";
export {
  defaultTestShellDeps,
  resolveTestShellDeps,
} from "./frontend-shell-deps.ts";
export type { ShellDeps, ShellDepsInput, TestShellDeps } from "./frontend-shell-deps.ts";
export {
  alignmentCollisionPlan,
  alignmentPlan,
  deferred,
  projectFixture,
  readyHealth,
  rememberedProjectRecord,
  taskFixture,
  workflowAnchors,
  workflowEventNodes,
  workflowFixture,
  workflowTaskFixture,
} from "./frontend-fixtures.ts";

export const waitForHandlers = () => new Promise((resolve) => setTimeout(resolve, 0));

let domHarnessQueue = Promise.resolve();

async function acquireDomHarnessLock(): Promise<() => void> {
  let releaseLock!: () => void;
  const nextLock = new Promise<void>((resolve) => {
    releaseLock = resolve;
  });
  const previousLock = domHarnessQueue;
  domHarnessQueue = domHarnessQueue.then(() => nextLock);
  await previousLock;
  return releaseLock;
}

export function mountTestShell(root: HTMLElement, deps: TestShellDeps = {}) {
  return mountReactApp(root, resolveTestShellDeps(deps));
}

export async function renderTestShell(deps: TestShellDeps = {}) {
  const releaseLock = await acquireDomHarnessLock();
  const window = new Window();
  const root = window.document.createElement("div");
  window.document.body.append(root);

  try {
    const unmount = await mountTestShell(root, deps);
    await waitForRole(root, "tree", /^Projects$/);
    let cleanedUp = false;

    return {
      window,
      root,
      cleanup: async () => {
        if (cleanedUp) {
          return;
        }

        cleanedUp = true;
        try {
          await unmount();
          root.remove();
        } finally {
          releaseLock();
        }
      },
    };
  } catch (error) {
    root.remove();
    releaseLock();
    throw error;
  }
}

export async function withRenderedShell<T>(
  deps: TestShellDeps,
  run: (shell: Awaited<ReturnType<typeof renderTestShell>>) => Promise<T>,
): Promise<T> {
  const shell = await renderTestShell(deps);
  try {
    return await run(shell);
  } finally {
    await shell.cleanup();
  }
}

function regexpMatches(pattern: RegExp, value: string): boolean {
  pattern.lastIndex = 0;
  return pattern.test(value);
}

function visibleText(element: Element): string {
  return (element.textContent ?? "").replace(/\s+/g, " ").trim();
}

function visibleTreeItemClickTarget(treeItem: HTMLElement, name: RegExp): HTMLElement {
  const descendants = Array.from(treeItem.querySelectorAll<HTMLElement>("*")).reverse();
  const visibleLabel = descendants.find((candidate) => regexpMatches(name, visibleText(candidate)));
  return visibleLabel ?? treeItem;
}

export function clickTreeItem(root: Element, name: RegExp): void {
  const treeItem = getByRole(root, "treeitem", name);
  const ownerWindow = treeItem.ownerDocument.defaultView;
  if (!ownerWindow) {
    throw new Error("expected tree item to belong to a document with a window");
  }

  visibleTreeItemClickTarget(treeItem, name).dispatchEvent(
    new ownerWindow.MouseEvent("click", { bubbles: true, cancelable: true }),
  );
}

export function setNativeValue(
  element: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
  value: string,
): void {
  const prototype =
    element instanceof element.ownerDocument.defaultView!.HTMLTextAreaElement
      ? element.ownerDocument.defaultView!.HTMLTextAreaElement.prototype
      : element instanceof element.ownerDocument.defaultView!.HTMLSelectElement
        ? element.ownerDocument.defaultView!.HTMLSelectElement.prototype
        : element.ownerDocument.defaultView!.HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
  if (!setter) {
    element.value = value;
    return;
  }
  setter.call(element, value);
}

export async function waitForRole(
  root: Element,
  role: AccessibleRole,
  name: RegExp,
  attempts = 5,
): Promise<HTMLElement> {
  let lastError: unknown;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      return getByRole(root, role, name);
    } catch (error) {
      lastError = error;
      await waitForHandlers();
    }
  }

  throw lastError;
}

export async function openProjectFromProjectsMenu(root: HTMLElement): Promise<void> {
  const ownerWindow = root.ownerDocument.defaultView;
  if (!ownerWindow) {
    throw new Error("expected test root to belong to a document with a window");
  }

  (await waitForRole(root, "button", /^Add project$/)).dispatchEvent(
    new ownerWindow.Event("click", { bubbles: true }),
  );
  await waitForHandlers();
  (await waitForRole(root, "menuitem", /^Use an existing folder$/)).dispatchEvent(
    new ownerWindow.Event("click", { bubbles: true }),
  );
  await waitForHandlers();
  await waitForHandlers();
}
