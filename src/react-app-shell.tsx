import { AppShell, MantineProvider } from "@mantine/core";
import { useEffect, useRef, useState, type ComponentProps } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";

import { ShellActionsProvider, useShellActions } from "./react-action-adapters.tsx";
import { mountShell } from "./shell-controller.ts";
import { ShellOverlays } from "./shell-overlays.tsx";
import {
  RememberedProjectsSection,
  type RememberedProjectsSectionProps,
} from "./sidebar-tree.tsx";
import { ProjectWorkspace } from "./work-area-panel.tsx";
import type { ShellDepsInput } from "./shell-types.ts";
import { useNativeWindowBounds } from "./use-native-window-bounds.ts";
import { useRememberedProjectsState } from "./use-remembered-projects-state.ts";

type DomGlobalName =
  | "window"
  | "document"
  | "navigator"
  | "Element"
  | "HTMLElement"
  | "Node"
  | "Event"
  | "MouseEvent"
  | "CustomEvent"
  | "FormData"
  | "getComputedStyle"
  | "requestAnimationFrame"
  | "cancelAnimationFrame";

type ReactShellHostProps = {
  onMounted: () => void;
  onError: (error: unknown) => void;
};

type ProjectWorkspaceProps = ComponentProps<typeof ProjectWorkspace>;
type ShellOverlaysProps = ComponentProps<typeof ShellOverlays>;

function installDomGlobals(ownerWindow: Window & typeof globalThis): () => void {
  const previousGlobals = new Map<DomGlobalName, PropertyDescriptor | undefined>();
  const replacedGlobals = new Set<DomGlobalName>();
  const nextGlobals: Record<DomGlobalName, unknown> = {
    window: ownerWindow,
    document: ownerWindow.document,
    navigator: ownerWindow.navigator,
    Element: ownerWindow.Element,
    HTMLElement: ownerWindow.HTMLElement,
    Node: ownerWindow.Node,
    Event: ownerWindow.Event,
    MouseEvent: ownerWindow.MouseEvent,
    CustomEvent: ownerWindow.CustomEvent,
    FormData: ownerWindow.FormData,
    getComputedStyle: ownerWindow.getComputedStyle.bind(ownerWindow),
    requestAnimationFrame:
      ownerWindow.requestAnimationFrame?.bind(ownerWindow) ??
      ((callback: FrameRequestCallback) => ownerWindow.setTimeout(() => callback(Date.now()), 16)),
    cancelAnimationFrame:
      ownerWindow.cancelAnimationFrame?.bind(ownerWindow) ??
      ownerWindow.clearTimeout.bind(ownerWindow),
  };

  for (const [name, value] of Object.entries(nextGlobals) as Array<[DomGlobalName, unknown]>) {
    const previousDescriptor = Object.getOwnPropertyDescriptor(globalThis, name);
    previousGlobals.set(name, previousDescriptor);
    if (
      previousDescriptor &&
      !previousDescriptor.configurable &&
      (globalThis as typeof globalThis & Record<DomGlobalName, unknown>)[name] === value
    ) {
      continue;
    }

    Object.defineProperty(globalThis, name, {
      configurable: true,
      writable: true,
      value,
    });
    replacedGlobals.add(name);
  }

  return () => {
    for (const name of replacedGlobals) {
      const previousDescriptor = previousGlobals.get(name);
      if (previousDescriptor) {
        Object.defineProperty(globalThis, name, previousDescriptor);
      } else {
        delete (globalThis as typeof globalThis & Record<DomGlobalName, unknown>)[name];
      }
    }
  };
}

function ReactShellHost({ onMounted, onError }: ReactShellHostProps) {
  const shellActions = useShellActions();
  const rememberedProjectsState = useRememberedProjectsState(
    shellActions.projectSession.loadRememberedProjectsSettings,
  );
  const mountedRef = useRef(false);
  const shellRootRef = useRef<HTMLDivElement | null>(null);
  const navigationHostRef = useRef<HTMLDivElement | null>(null);
  const workAreaHostRef = useRef<HTMLDivElement | null>(null);
  const overlayHostRef = useRef<HTMLDivElement | null>(null);
  const [sidebarModel, setSidebarModel] = useState<RememberedProjectsSectionProps | null>(null);
  const [workAreaModel, setWorkAreaModel] = useState<ProjectWorkspaceProps | null>(null);
  const [overlaysModel, setOverlaysModel] = useState<ShellOverlaysProps | null>(null);
  const selectionStateRef = useRef({
    selectedTaskId: null as string | null,
    selectedEventId: null as string | null,
  });

  const renderSidebar = (nextModel: RememberedProjectsSectionProps) => {
    setSidebarModel(nextModel);
  };

  const renderWorkArea = (nextModel: ProjectWorkspaceProps) => {
    setWorkAreaModel(nextModel);
  };

  const renderOverlays = (nextModel: ShellOverlaysProps) => {
    setOverlaysModel(nextModel);
  };

  const treeSelectionState = {
    getState: () => ({
      selectedTaskId: selectionStateRef.current.selectedTaskId,
      selectedEventId: selectionStateRef.current.selectedEventId,
    }),
    setState: ({
      selectedTaskId,
      selectedEventId,
    }: {
      selectedTaskId: string | null;
      selectedEventId: string | null;
    }) => {
      selectionStateRef.current = {
        selectedTaskId,
        selectedEventId,
      };
      if (sidebarModel) {
        renderSidebar({
          ...sidebarModel,
          selectedTaskId,
          selectedEventId,
        });
      }
    },
  };

  useEffect(() => {
    if (
      mountedRef.current ||
      !shellRootRef.current ||
      !navigationHostRef.current ||
      !workAreaHostRef.current ||
      !overlayHostRef.current ||
      !rememberedProjectsState.ready
    ) {
      return;
    }

    mountedRef.current = true;
    void mountShell(
      shellRootRef.current,
      shellActions,
      {
        navigationHost: navigationHostRef.current,
        workAreaHost: workAreaHostRef.current,
        overlayHost: overlayHostRef.current,
        renderNavigation: renderSidebar,
        renderWorkArea,
        renderOverlays,
        treeSelectionState,
      },
      rememberedProjectsState,
    ).then(onMounted, onError);
  }, [onError, onMounted, rememberedProjectsState, shellActions]);

  useNativeWindowBounds(shellRootRef, [sidebarModel, workAreaModel]);

  return (
    <MantineProvider defaultColorScheme="light">
      <div
        data-testid="react-shell-host"
        ref={shellRootRef}
        onContextMenuCapture={(event) => {
          event.preventDefault();
        }}
      >
        <AppShell
          className="app-shell"
          data-testid="mantine-app-shell"
          navbar={{ width: 280, breakpoint: 0 }}
          padding={0}
          withBorder={false}
        >
          <AppShell.Navbar p={0}>
            <div
              aria-label="Project navigation"
              className="project-rail"
              data-testid="shell-sidebar-host"
              ref={navigationHostRef}
            >
              {sidebarModel ? <RememberedProjectsSection {...sidebarModel} /> : null}
            </div>
          </AppShell.Navbar>
          <AppShell.Main className="app-shell-main">
            <div
              aria-label="Planning surface"
              className="work-area"
              data-testid="shell-main-host"
              ref={workAreaHostRef}
            >
              {workAreaModel ? <ProjectWorkspace {...workAreaModel} /> : null}
            </div>
          </AppShell.Main>
        </AppShell>
        <div data-testid="shell-overlay-host" ref={overlayHostRef}>
          {overlaysModel ? <ShellOverlays {...overlaysModel} /> : null}
        </div>
      </div>
    </MantineProvider>
  );
}

export async function mountReactApp(
  root: HTMLElement,
  deps: ShellDepsInput = {},
): Promise<() => Promise<void>> {
  const ownerWindow = root.ownerDocument.defaultView;
  if (!ownerWindow) {
    throw new Error("expected app root to belong to a document with a window");
  }

  const reactRoot = createRoot(root);
  const restoreDomGlobals = installDomGlobals(ownerWindow as Window & typeof globalThis);
  let cleanedUp = false;

  const cleanup = async () => {
    if (cleanedUp) {
      return;
    }

    cleanedUp = true;
    try {
      flushSync(() => {
        reactRoot.unmount();
      });
      await new Promise<void>((resolve) => {
        ownerWindow.setTimeout(resolve, 0);
      });
    } finally {
      restoreDomGlobals();
    }
  };

  try {
    await new Promise<void>((resolve, reject) => {
      reactRoot.render(
        <ShellActionsProvider deps={deps}>
          <ReactShellHost onMounted={resolve} onError={reject} />
        </ShellActionsProvider>,
      );
    });
    await new Promise<void>((resolve) => {
      ownerWindow.setTimeout(resolve, 0);
    });
    return cleanup;
  } catch (error) {
    await cleanup();
    throw error;
  }
}
