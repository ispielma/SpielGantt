import assert from "node:assert/strict";
import test from "node:test";

import { taskStatusOptions } from "../src/shell-types.ts";
import {
  accessibleName,
  clickTreeItem,
  getByRole,
  openProjectFromProjectsMenu,
  projectFixture,
  renderTestShell,
  setNativeValue,
  taskFixture,
  waitForHandlers,
  workflowAnchors,
  workflowFixture,
  workflowTaskFixture,
} from "./support/frontend-shell.ts";

function queryByRole(root: Element, role: Parameters<typeof getByRole>[1], name: RegExp) {
  try {
    return getByRole(root, role, name);
  } catch {
    return null;
  }
}

function getByAccessibleLabel(root: Element, label: string): HTMLElement {
  const match = Array.from(root.querySelectorAll<HTMLElement>("[aria-label]")).find(
    (element) => element.getAttribute("aria-label") === label,
  );
  if (!match) {
    throw new Error(`expected element labelled ${label} to be present`);
  }
  return match;
}

function queryByAccessibleLabel(root: Element, label: string): HTMLElement | null {
  try {
    return getByAccessibleLabel(root, label);
  } catch {
    return null;
  }
}

function normalizedText(element: Element): string {
  return (element.textContent ?? "").replace(/\s+/g, " ").trim();
}

function regexpMatches(pattern: RegExp, value: string): boolean {
  pattern.lastIndex = 0;
  return pattern.test(value);
}

function queryByText(container: Element, text: RegExp): HTMLElement | null {
  return (
    Array.from(container.querySelectorAll<HTMLElement>("*")).find((element) => {
      if (!regexpMatches(text, normalizedText(element))) {
        return false;
      }

      return !Array.from(element.children).some((child) =>
        regexpMatches(text, normalizedText(child)),
      );
    }) ?? null
  );
}

function getByText(container: Element, text: RegExp): HTMLElement {
  const match = queryByText(container, text);
  if (!match) {
    throw new Error(`expected visible text ${text} to be present`);
  }
  return match;
}

function selectOptionNames(select: HTMLSelectElement): string[] {
  return Array.from(select.children)
    .filter((element) => element.matches("option"))
    .map(accessibleName)
    .filter((name) => name.length > 0);
}

function workflowProject() {
  return projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    projectReadmeContent: "# Workflow\n\n- project note\n",
    projectReadmeVersion: "project-v1",
    events: ["Samples ready"],
    eventReferences: [
      {
        id: "Samples ready",
        referencedTaskIds: ["sample-prep"],
        blockerTaskIds: ["sample-prep"],
        blockedTaskIds: ["analysis-run"],
      },
    ],
    tasks: [
      taskFixture("sample-prep", {
        path: "/tmp/fixture-spielgantt/workflow/sample-prep",
        status: "unblocked",
      }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        status: "unblocked",
        readmeContent: "# Analysis\n",
        readmeVersion: "v1",
      }),
    ],
  });
}

test("project inspector shows and edits the project README when no task or event is selected", async () => {
  const readmeEdits: Array<unknown> = [];
  const project = workflowProject();
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project,
      editProjectReadmeAction: async (projectRoot, edit) => {
        readmeEdits.push({ projectRoot, edit });
        return {
          project: {
            ...project,
            projectReadmeContent: edit.readmeContent,
            projectReadmeVersion: "project-v2",
          },
        };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    const projectInspector = getByAccessibleLabel(root, "Project inspector");
    const readme = getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement;
    assert.ok(projectInspector, "project inspector should render for the project row selection");
    assert.equal(readme.getAttribute("spellcheck"), "false");
    assert.equal(readme.value, "# Workflow\n\n- project note\n");
    assert.equal(queryByRole(root, "textbox", /^Task README$/), null);
    assert.equal(queryByRole(root, "combobox", /^Task status$/), null);
    assert.doesNotMatch(projectInspector.textContent ?? "", /Tasks/);
    assert.doesNotMatch(projectInspector.textContent ?? "", /Events/);

    setNativeValue(readme, "# Workflow\n\n- saved project note\n");
    readme.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitForHandlers();
    assert.deepEqual(readmeEdits, [], "project README should not persist before blur");

    readme.dispatchEvent(new window.Event("blur", { bubbles: true }));
    readme.dispatchEvent(new window.Event("focusout", { bubbles: true }));
    await waitForHandlers();

    assert.deepEqual(readmeEdits, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        edit: {
          readmeContent: "# Workflow\n\n- saved project note\n",
          expectedReadmeVersion: "project-v1",
        },
      },
    ]);
    assert.equal(
      (getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement).value,
      "# Workflow\n\n- saved project note\n",
      "successful saves should refresh the project README textbox"
    );

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();
    assert.ok(getByAccessibleLabel(root, "Task inspector"));
    assert.equal(queryByRole(root, "textbox", /^Project README$/), null);

    clickTreeItem(root, /^Samples ready$/);
    await waitForHandlers();
    assert.ok(getByAccessibleLabel(root, "Event inspector"));
    assert.equal(queryByRole(root, "textbox", /^Project README$/), null);

    clickTreeItem(root, /^workflow$/);
    await waitForHandlers();
    assert.equal(
      (getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement).value,
      "# Workflow\n\n- saved project note\n",
    );
  } finally {
    await cleanup();
  }
});

test("project README stale-version conflicts stay visible at the editor with retry and revert", async () => {
  const project = workflowProject();
  const editAttempts: Array<unknown> = [];
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project,
      editProjectReadmeAction: async (_projectRoot, edit) => {
        editAttempts.push(edit);
        throw new Error("stale README version");
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    const readme = getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement;
    setNativeValue(readme, "# Workflow\n\n- conflicted project note\n");
    readme.dispatchEvent(new window.Event("input", { bubbles: true }));
    readme.dispatchEvent(new window.Event("blur", { bubbles: true }));
    readme.dispatchEvent(new window.Event("focusout", { bubbles: true }));
    await waitForHandlers();
    await waitForHandlers();

    assert.equal(
      (getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement).value,
      "# Workflow\n\n- conflicted project note\n",
      "project README conflicts should keep the draft visible instead of pretending it saved",
    );
    const projectInspector = getByAccessibleLabel(root, "Project inspector");
    assert.match(projectInspector.textContent ?? "", /stale README version/);

    getByRole(root, "button", /^Retry project README save$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();
    assert.equal(editAttempts.length, 2, "retry should submit the still-visible project README draft");

    getByRole(root, "button", /^Revert project README changes$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();
    assert.equal(
      (getByRole(root, "textbox", /^Project README$/) as HTMLTextAreaElement).value,
      "# Workflow\n\n- project note\n",
      "revert should restore the last backend-accepted project README",
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector omits redundant summary and folder actions", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => workflowProject(),
      onboardProjectAction: async () => workflowProject()
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    assert.equal(queryByRole(root, "button", /^Open task folder/i), null);
    assert.doesNotMatch(root.textContent ?? "", /Selected task/);
    assert.doesNotMatch(root.textContent ?? "", /Project path/);
    assert.equal(queryByAccessibleLabel(root, "Task path"), null);
    assert.match(root.textContent ?? "", /analysis-run/);
  } finally {
    await cleanup();
  }
});

test("selected task inspector exposes task metadata, relationship controls, and README editing", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["START", "DONE"],
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              projectRelativePath: "phase-1/analysis-run",
              status: "unblocked",
              dependencies: ["START"],
              dependencyTargets: [{ id: "START", kind: "event" }],
              endsAt: "DONE",
              readmeContent: "# Analysis\n",
              readmeVersion: "v1",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["START", "DONE"],
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              projectRelativePath: "phase-1/analysis-run",
              status: "unblocked",
              dependencies: ["START"],
              dependencyTargets: [{ id: "START", kind: "event" }],
              endsAt: "DONE",
              readmeContent: "# Analysis\n",
              readmeVersion: "v1",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const taskInspector = getByAccessibleLabel(root, "Task inspector");
    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    const readmeEditor = getByRole(root, "textbox", /^Task README$/);
    const statusEditor = getByRole(root, "combobox", /^Task status$/);

    assert.match(taskInspector.textContent ?? "", /analysis-run/);
    assert.doesNotMatch(taskInspector.textContent ?? "", /phase-1\/analysis-run/);
    assert.ok(statusEditor, "task status control should remain available");
    assert.deepEqual(
      Array.from((statusEditor as HTMLSelectElement).options).map((option) => option.value),
      taskStatusOptions.map((option) => option.value),
      "task status choices should match the canonical status contract",
    );
    assert.ok(dependencySection, "dependency controls should render for the selected task");
    assert.ok(readmeEditor, "README editing should remain available");
  } finally {
    await cleanup();
  }
});

test("selected task inspector preserves markdown notes in the README editor", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              readmeContent: "# Analysis\n\n- preserve markdown notes\n",
              readmeVersion: "v1",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              readmeContent: "# Analysis\n\n- preserve markdown notes\n",
              readmeVersion: "v1",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const readmeEditor = getByRole(root, "textbox", /^Task README$/) as HTMLTextAreaElement;

    assert.equal(
      readmeEditor.value,
      "# Analysis\n\n- preserve markdown notes\n",
      "README content should preserve multiline markdown text",
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector renders dependency and ends-at controls from backend-provided data", async () => {
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["START", "MOT", "DONE"],
          workflow: workflowFixture({
            events: ["START", "MOT", "DONE"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                valid_ends_at_targets: ["START", "MOT", "DONE"],
              }),
            ],
          }),
          tasks: [
            taskFixture("sample-prep", {
              path: "/tmp/fixture-spielgantt/workflow/sample-prep",
            }),
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["sample-prep"],
              blocks: [{ id: "report-results", kind: "task" }],
              dependencyTargets: [
                { id: "sample-prep", kind: "task" },
                { id: "START", kind: "event" },
                { id: "MOT", kind: "event" },
              ],
              endsAt: "DONE",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["START", "MOT", "DONE"],
          workflow: workflowFixture({
            events: ["START", "MOT", "DONE"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                valid_ends_at_targets: ["START", "MOT", "DONE"],
              }),
            ],
          }),
          tasks: [
            taskFixture("sample-prep", {
              path: "/tmp/fixture-spielgantt/workflow/sample-prep",
            }),
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["sample-prep"],
              blocks: [{ id: "report-results", kind: "task" }],
              dependencyTargets: [
                { id: "sample-prep", kind: "task" },
                { id: "START", kind: "event" },
                { id: "MOT", kind: "event" },
              ],
              endsAt: "DONE",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.ok(dependencySection, "dependency controls should render for the selected task");
    assert.ok(
      getByRole(root, "combobox", /^Task status$/),
      "task status picker should render with an accessible control",
    );
    assert.equal(queryByRole(root, "textbox", /^Task progress$/), null);
    assert.ok(
      getByRole(root, "textbox", /^Task README$/),
      "task README field should render with an accessible control",
    );
    assert.ok(
      getByRole(dependencySection!, "combobox", /^Dependency target$/),
      "dependency target picker should render with a target-agnostic accessible control",
    );
    assert.ok(
      getByRole(dependencySection!, "button", /^Add dependency target to selected task$/),
      "dependency add button should render with a target-agnostic accessible control",
    );
    assert.ok(
      getByRole(dependencySection!, "combobox", /^End at event$/),
      "ends-at event picker should render inside dependencies with an accessible control",
    );

    const dependencyOptions = Array.from(
      (getByRole(dependencySection!, "combobox", /^Dependency target$/) as HTMLSelectElement).options,
    ).map((option) => option.textContent?.trim());
    assert.deepEqual(dependencyOptions, [
      "Choose dependency target",
      "Task: sample-prep",
      "Event: START",
      "Event: MOT",
    ]);

    const endsAtSelect = getByRole(
      dependencySection!,
      "combobox",
      /^End at event$/,
    ) as HTMLSelectElement;
    assert.deepEqual(selectOptionNames(endsAtSelect), [
      "No end event set",
      "Event: START",
      "Event: MOT",
      "Event: DONE",
    ]);
  } finally {
    await cleanup();
  }
});

test("selected task inspector renders end-event choices from backend workflow validity data", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["start", "finished", "samples ready", "data ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["start", "finished", "samples ready", "data ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                dependency_references: [{ id: "data ready", kind: "event", valid: true }],
                valid_ends_at_targets: ["data ready", "analysis complete"],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["data ready"],
              dependencyTargets: [{ id: "data ready", kind: "event" }],
              readmeContent: "# Analysis\n",
              readmeVersion: "v1",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["start", "finished", "samples ready", "data ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["start", "finished", "samples ready", "data ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                dependency_references: [{ id: "data ready", kind: "event", valid: true }],
                valid_ends_at_targets: ["data ready", "analysis complete"],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["data ready"],
              dependencyTargets: [{ id: "data ready", kind: "event" }],
              readmeContent: "# Analysis\n",
              readmeVersion: "v1",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.ok(getByRole(root, "combobox", /^Task status$/));
    assert.ok(getByRole(root, "textbox", /^Task README$/));
    assert.ok(getByRole(dependencySection, "combobox", /^Dependency target$/));
    const endsAtSelect = getByRole(dependencySection, "combobox", /^End at event$/) as HTMLSelectElement;

    assert.equal(
      endsAtSelect.value,
      "",
      "a task with no persisted end event should keep the empty saved value",
    );
    assert.ok(
      getByRole(endsAtSelect, "option", /^No end event set$/),
      "the empty end-event choice should have a meaningful announced value",
    );
    assert.ok(getByRole(endsAtSelect, "option", /^Event: data ready$/));
    assert.ok(getByRole(endsAtSelect, "option", /^Event: analysis complete$/));
    assert.deepEqual(
      selectOptionNames(endsAtSelect),
      ["No end event set", "Event: data ready", "Event: analysis complete"],
      "end-event choices should keep backend valid_ends_at_targets plus a named unset value",
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector shows placement warnings inside dependency controls", async () => {
  const diagnostic = "backend says analysis-run has no upstream workflow anchor";
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["analysis complete"],
          workflow: workflowFixture({
            events: ["analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors(null, "analysis complete", [diagnostic]),
                placement_status: "incomplete",
                placement_messages: [diagnostic],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              endsAt: "analysis complete",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["analysis complete"],
          workflow: workflowFixture({
            events: ["analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors(null, "analysis complete", [diagnostic]),
                placement_status: "incomplete",
                placement_messages: [diagnostic],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              endsAt: "analysis complete",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    const warning = getByText(dependencySection, /^Undetermined$/);
    assert.match(
      dependencySection.textContent ?? "",
      /backend says analysis-run has no upstream workflow anchor/,
    );

    const dependencyTarget = getByRole(dependencySection, "combobox", /^Dependency target$/);
    const endAtEvent = getByRole(dependencySection, "combobox", /^End at event$/);
    assert.ok(warning);
    assert.ok(dependencyTarget);
    assert.ok(endAtEvent);
  } finally {
    await cleanup();
  }
});

test("selected task inspector names a missing end event from backend workflow anchors", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready"],
          workflow: workflowFixture({
            events: ["samples ready"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors("samples ready", null),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready"],
          workflow: workflowFixture({
            events: ["samples ready"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors("samples ready", null),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.ok(getByText(dependencySection, /^Undetermined$/));
    assert.match(
      dependencySection.textContent ?? "",
      /Complete this task's event-axis workflow placement/,
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector uses backend task determination status instead of deriving from anchors", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "fully_determined",
                effective_anchors: workflowAnchors(null, null),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "fully_determined",
                effective_anchors: workflowAnchors(null, null),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.equal(
      queryByText(dependencySection, /^Undetermined$/) === null,
      true,
      "backend fully_determined status should suppress undetermined placement copy even when anchors are absent",
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector consumes backend placement status instead of anchor diagnostics", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "undetermined",
                placement_ready: true,
                placement_status: "ready",
                placement_messages: [],
                effective_anchors: workflowAnchors(null, null, [
                  "legacy anchor diagnostic that should not drive inspector warnings",
                ]),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "undetermined",
                placement_ready: true,
                placement_status: "ready",
                placement_messages: [],
                effective_anchors: workflowAnchors(null, null, [
                  "legacy anchor diagnostic that should not drive inspector warnings",
                ]),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.equal(
      queryByText(dependencySection, /^Undetermined$/),
      null,
      "ready backend placement status should suppress inspector placement warnings",
    );
    assert.doesNotMatch(
      dependencySection.textContent ?? "",
      /legacy anchor diagnostic/,
      "inspector placement copy should come from backend placement_messages",
    );
  } finally {
    await cleanup();
  }
});

test("selected task inspector surfaces backend workflow diagnostics", async () => {
  const diagnostic =
    "task 'analysis-run' ends_at references missing event id 'MISSING-EVENT'";
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready"],
          workflow: workflowFixture({
            events: ["samples ready"],
            validation: { valid: false, diagnostics: [diagnostic] },
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors("samples ready", null, [diagnostic]),
                validation_diagnostics: [diagnostic],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
              endsAt: "MISSING-EVENT",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready"],
          workflow: workflowFixture({
            events: ["samples ready"],
            validation: { valid: false, diagnostics: [diagnostic] },
            tasks: [
              workflowTaskFixture("analysis-run", {
                effective_anchors: workflowAnchors("samples ready", null, [diagnostic]),
                validation_diagnostics: [diagnostic],
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
              endsAt: "MISSING-EVENT",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.ok(getByText(dependencySection, /^Placement diagnostic$/));
    assert.match(dependencySection.textContent ?? "", /MISSING-EVENT/);
    assert.match(dependencySection.textContent ?? "", /references missing event id/);
  } finally {
    await cleanup();
  }
});

test("selected task inspector shows a quiet fully determined state", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "fully_determined",
                effective_anchors: workflowAnchors("samples ready", "analysis complete"),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
              endsAt: "analysis complete",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["samples ready", "analysis complete"],
          workflow: workflowFixture({
            events: ["samples ready", "analysis complete"],
            tasks: [
              workflowTaskFixture("analysis-run", {
                determination_status: "fully_determined",
                effective_anchors: workflowAnchors("samples ready", "analysis complete"),
              }),
            ],
          }),
          tasks: [
            taskFixture("analysis-run", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run",
              dependencies: ["samples ready"],
              endsAt: "analysis complete",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    assert.equal(
      queryByText(dependencySection, /^Fully determined$/) === null,
      true,
      "fully determined tasks should not show a determination banner",
    );
  } finally {
    await cleanup();
  }
});

test("selected task dependency target picker starts with a no-op placeholder", async () => {
  const addedDependencies: Array<{ projectRoot: string; taskId: string; blockerId: string }> = [];
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["START"],
    workflow: workflowFixture({
      events: ["START"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START"],
        }),
      ],
    }),
    tasks: [
      taskFixture("sample-prep", {
        path: "/tmp/fixture-spielgantt/workflow/sample-prep",
      }),
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        dependencyTargets: [
          { id: "sample-prep", kind: "task" },
          { id: "START", kind: "event" },
        ],
      }),
    ],
  });

  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project
    },
    taskMutation: {
      addDependencyAction: async (projectRoot, taskId, blockerId) => {
        addedDependencies.push({ projectRoot, taskId, blockerId });
        return { project };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const dependencyTargetSelect = getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement;
    assert.equal(dependencyTargetSelect.value, "");
    assert.deepEqual(
      Array.from(dependencyTargetSelect.options).map((option) => option.textContent?.trim()),
      ["Choose dependency target", "Task: sample-prep", "Event: START"],
      "the dependency target picker should require an intentional choice"
    );

    getByRole(root, "button", /^Add dependency target to selected task$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();

    assert.deepEqual(
      addedDependencies,
      [],
      "pressing Add on the placeholder should not mutate task dependencies"
    );
    assert.match(
      root.textContent ?? "",
      /Choose a dependency target before adding it\./,
      "the inspector should keep the validation error visible next to the failed add workflow",
    );
    assert.doesNotMatch(root.textContent ?? "", /Saved task|Added blocker/i);
  } finally {
    await cleanup();
  }
});

test("selected task dependency target choice survives unrelated inspector saves before Add", async () => {
  const addedDependencies: Array<{ projectRoot: string; taskId: string; blockerId: string }> = [];
  const initialProject = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["start", "Protocol selected"],
    workflow: workflowFixture({
      events: ["start", "Protocol selected"],
      tasks: [
        workflowTaskFixture("Literature review", {
          valid_ends_at_targets: ["Protocol selected"],
        }),
      ],
    }),
    tasks: [
      taskFixture("Literature review", {
        path: "/tmp/fixture-spielgantt/workflow/Literature review",
        dependencyTargets: [{ id: "start", kind: "event" }],
        endsAt: "Protocol selected",
        status: "unblocked",
        readmeContent: "# Literature review\n",
        readmeVersion: "v1",
      }),
    ],
  });
  const statusEditedProject = projectFixture({
    ...initialProject,
    tasks: [
      taskFixture("Literature review", {
        path: "/tmp/fixture-spielgantt/workflow/Literature review",
        dependencyTargets: [{ id: "start", kind: "event" }],
        endsAt: "Protocol selected",
        status: "blocked",
        readmeContent: "# Literature review\n",
        readmeVersion: "v2",
      }),
    ],
  });

  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => initialProject,
      onboardProjectAction: async () => initialProject
    },
    taskMutation: {
      editTaskAction: async () => ({ project: statusEditedProject }),
      addDependencyAction: async (projectRoot, taskId, blockerId) => {
        addedDependencies.push({ projectRoot, taskId, blockerId });
        return { project: statusEditedProject };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^Literature review$/);
    await waitForHandlers();

    const dependencyTargetSelect = getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement;
    assert.deepEqual(
      Array.from(dependencyTargetSelect.options).map((option) => option.textContent?.trim()),
      ["Choose dependency target", "Event: start"],
    );
    setNativeValue(dependencyTargetSelect, "start");
    dependencyTargetSelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    assert.equal(dependencyTargetSelect.value, "start");

    const status = getByRole(root, "combobox", /^Task status$/) as HTMLSelectElement;
    setNativeValue(status, "blocked");
    status.dispatchEvent(new window.Event("change", { bubbles: true }));
    await waitForHandlers();

    assert.equal(
      (getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement).value,
      "start",
      "unrelated inspector saves should not discard the pending dependency target choice",
    );

    getByRole(root, "button", /^Add dependency target to selected task$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();

    assert.deepEqual(addedDependencies, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "Literature review",
        blockerId: "start",
      },
    ]);
  } finally {
    await cleanup();
  }
});

test("rejected task status saves revert the control and show the backend error", async () => {
  const project = workflowProject();
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project
    },
    taskMutation: {
      editTaskAction: async () => {
        throw new Error("status was rejected by validation");
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const status = getByRole(root, "combobox", /^Task status$/) as HTMLSelectElement;
    assert.equal(status.value, "unblocked");

    setNativeValue(status, "done");
    status.dispatchEvent(new window.Event("change", { bubbles: true }));
    await waitForHandlers();
    await waitForHandlers();

    assert.equal(
      (getByRole(root, "combobox", /^Task status$/) as HTMLSelectElement).value,
      "unblocked",
      "rejected task status saves should revert to the last backend-accepted value",
    );
    assert.match(root.textContent ?? "", /status was rejected by validation/);
  } finally {
    await cleanup();
  }
});

test("rejected task README saves revert to the last accepted README and show the backend error", async () => {
  const project = workflowProject();
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project
    },
    taskMutation: {
      editTaskAction: async () => {
        throw new Error("README version conflict");
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    const readme = getByRole(root, "textbox", /^Task README$/) as HTMLTextAreaElement;
    assert.equal(readme.value, "# Analysis\n");

    setNativeValue(readme, "# Rejected analysis\n");
    readme.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitForHandlers();
    assert.equal(readme.value, "# Rejected analysis\n");

    readme.dispatchEvent(new window.Event("blur", { bubbles: true }));
    readme.dispatchEvent(new window.Event("focusout", { bubbles: true }));
    await waitForHandlers();
    await waitForHandlers();

    assert.equal(
      (getByRole(root, "textbox", /^Task README$/) as HTMLTextAreaElement).value,
      "# Analysis\n",
      "rejected task README saves should revert to the last backend-accepted value",
    );
    assert.match(root.textContent ?? "", /README version conflict/);
  } finally {
    await cleanup();
  }
});

test("selected task inspector offers a plain task dependency target for an event-ending task", async () => {
  const { root, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["Workflow started", "Protocol selected"],
          tasks: [
            taskFixture("make chart", {
              path: "/tmp/fixture-spielgantt/workflow/make chart",
              dependencies: ["Workflow started"],
            }),
            taskFixture("literature-review", {
              path: "/tmp/fixture-spielgantt/workflow/literature-review",
              dependencyTargets: [
                { id: "make chart", kind: "task" },
                { id: "Workflow started", kind: "event" },
              ],
              endsAt: "Protocol selected",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["Workflow started", "Protocol selected"],
          tasks: [
            taskFixture("make chart", {
              path: "/tmp/fixture-spielgantt/workflow/make chart",
              dependencies: ["Workflow started"],
            }),
            taskFixture("literature-review", {
              path: "/tmp/fixture-spielgantt/workflow/literature-review",
              dependencyTargets: [
                { id: "make chart", kind: "task" },
                { id: "Workflow started", kind: "event" },
              ],
              endsAt: "Protocol selected",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^literature-review$/);
    await waitForHandlers();

    const dependencySection = getByRole(root, "group", /^Dependencies$/);
    const dependencyOptions = Array.from(
      (getByRole(dependencySection, "combobox", /^Dependency target$/) as HTMLSelectElement).options,
    ).map((option) => option.textContent?.trim());

    assert.deepEqual(dependencyOptions, [
      "Choose dependency target",
      "Task: make chart",
      "Event: Workflow started",
    ]);
  } finally {
    await cleanup();
  }
});

test("selected event inspector renders only event name and relationship groups from backend-provided data", async () => {
  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["Sample intake", "Samples ready"],
          eventReferences: [
            {
              id: "Sample intake",
              referencedTaskIds: ["intake-check", "sample-prep"],
              blockerTaskIds: ["intake-check", "sample-prep"],
              blockedTaskIds: [],
            },
            {
              id: "Samples ready",
              referencedTaskIds: ["sample-prep", "analysis-run"],
              blockerTaskIds: ["sample-prep"],
              blockedTaskIds: [
                "analysis-run-with-a-human-readable-path-component-that-wraps-cleanly",
              ],
            },
          ],
          tasks: [
            taskFixture("intake-check", {
              path: "/tmp/fixture-spielgantt/workflow/intake-check",
            }),
            taskFixture("sample-prep", {
              path: "/tmp/fixture-spielgantt/workflow/sample-prep",
            }),
            taskFixture("analysis-run-with-a-human-readable-path-component-that-wraps-cleanly", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run-with-a-human-readable-path-component-that-wraps-cleanly",
            }),
          ],
        }),
      onboardProjectAction: async () =>
        projectFixture({
          selectedPath: "/tmp/fixture-spielgantt/workflow",
          projectRoot: "/tmp/fixture-spielgantt/workflow",
          events: ["Sample intake", "Samples ready"],
          eventReferences: [
            {
              id: "Sample intake",
              referencedTaskIds: ["intake-check", "sample-prep"],
              blockerTaskIds: ["intake-check", "sample-prep"],
              blockedTaskIds: [],
            },
            {
              id: "Samples ready",
              referencedTaskIds: ["sample-prep", "analysis-run"],
              blockerTaskIds: ["sample-prep"],
              blockedTaskIds: [
                "analysis-run-with-a-human-readable-path-component-that-wraps-cleanly",
              ],
            },
          ],
          tasks: [
            taskFixture("intake-check", {
              path: "/tmp/fixture-spielgantt/workflow/intake-check",
            }),
            taskFixture("sample-prep", {
              path: "/tmp/fixture-spielgantt/workflow/sample-prep",
            }),
            taskFixture("analysis-run-with-a-human-readable-path-component-that-wraps-cleanly", {
              path: "/tmp/fixture-spielgantt/workflow/analysis-run-with-a-human-readable-path-component-that-wraps-cleanly",
            }),
          ],
        })
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);

    clickTreeItem(root, /^Samples ready$/);
    await waitForHandlers();

    const inspector = getByAccessibleLabel(root, "Event inspector");
    const beforeGroup = getByRole(root, "group", /^Before this event$/);
    const afterGroup = getByRole(root, "group", /^After this event$/);
    assert.match(inspector.textContent ?? "", /Samples ready/);
    assert.ok(beforeGroup, "before-event relationship group should render");
    assert.ok(afterGroup, "after-event relationship group should render");
    assert.doesNotMatch(inspector.textContent ?? "", /Milestone detail/);
    assert.doesNotMatch(inspector.textContent ?? "", /Event rail/);
    assert.match(inspector.textContent ?? "", /sample-prep/);
    assert.match(
      inspector.textContent ?? "",
      /analysis-run-with-a-human-readable-path-component-that-wraps-cleanly/,
    );
    assert.equal(queryByAccessibleLabel(root, "Task inspector"), null);
  } finally {
    await cleanup();
  }
});

test("selected task inspector actions are reachable through accessible controls", async () => {
  const editedTasks: Array<unknown> = [];
  const addedDependencies: Array<{ projectRoot: string; taskId: string; blockerId: string }> = [];
  const removedDependencies: Array<{ projectRoot: string; taskId: string; blockerId: string }> = [];
  const endsAtChanges: Array<{
    projectRoot: string;
    taskId: string;
    eventId: string | null;
    clear: boolean;
  }> = [];
  const project = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["START", "MOT", "DONE"],
    workflow: workflowFixture({
      events: ["START", "MOT", "DONE"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START", "MOT", "DONE"],
        }),
      ],
    }),
    tasks: [
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        dependencies: ["sample-prep"],
        dependencyTargets: [
          { id: "START", kind: "event" },
          { id: "MOT", kind: "event" },
        ],
        endsAt: "DONE",
        readmeVersion: "v1",
      }),
    ],
  });

  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => project,
      onboardProjectAction: async () => project
    },
    taskMutation: {
      editTaskAction: async (projectRoot, taskId, edit) => {
        editedTasks.push({ projectRoot, taskId, edit });
        return { project };
      },
      addDependencyAction: async (projectRoot, taskId, blockerId) => {
        addedDependencies.push({ projectRoot, taskId, blockerId });
        return { project };
      },
      removeDependencyAction: async (projectRoot, taskId, blockerId) => {
        removedDependencies.push({ projectRoot, taskId, blockerId });
        return { project };
      },
      setTaskEndsAtAction: async (projectRoot, taskId, eventId, clear) => {
        endsAtChanges.push({ projectRoot, taskId, eventId, clear });
        return { project };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();
    await waitForHandlers();

    const status = getByRole(root, "combobox", /^Task status$/) as HTMLSelectElement;
    setNativeValue(status, "done");
    status.dispatchEvent(new window.Event("change", { bubbles: true }));
    await waitForHandlers();

    assert.doesNotMatch(root.textContent ?? "", /Saved task 'analysis-run'\./);
    assert.equal(queryByRole(root, "textbox", /^Task progress$/), null);

    const editsBeforeReadmeInput = editedTasks.length;
    const readme = getByRole(root, "textbox", /^Task README$/) as HTMLTextAreaElement;
    setNativeValue(readme, "# Done\n");
    readme.dispatchEvent(new window.Event("input", { bubbles: true }));
    await waitForHandlers();
    assert.equal(
      editedTasks.length,
      editsBeforeReadmeInput,
      "README content should not persist before blur",
    );
    readme.dispatchEvent(new window.Event("blur", { bubbles: true }));
    readme.dispatchEvent(new window.Event("focusout", { bubbles: true }));
    await waitForHandlers();
    assert.equal(
      editedTasks.length,
      editsBeforeReadmeInput + 1,
      "README content should persist on blur",
    );

    const dependencySelect = getByRole(root, "combobox", /^Dependency target$/) as HTMLSelectElement;
    setNativeValue(dependencySelect, "START");
    dependencySelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    getByRole(root, "button", /^Add dependency target to selected task$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();

    getByRole(root, "button", /^Remove blocker sample-prep$/).dispatchEvent(
      new window.Event("click", { bubbles: true }),
    );
    await waitForHandlers();

    const endsAtSelect = getByRole(root, "combobox", /^End at event$/) as HTMLSelectElement;
    setNativeValue(endsAtSelect, "MOT");
    endsAtSelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    await waitForHandlers();

    setNativeValue(endsAtSelect, "");
    endsAtSelect.dispatchEvent(new window.Event("change", { bubbles: true }));
    await waitForHandlers();
    assert.equal(queryByRole(root, "button", /^Set task ends at$/), null);

    assert.deepEqual(editedTasks.at(-1), {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        edit: {
          status: "done",
          readmeContent: "# Done\n",
          expectedReadmeVersion: "v1",
        },
    });
    assert.deepEqual(addedDependencies, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        blockerId: "START",
      },
    ]);
    assert.deepEqual(removedDependencies, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        blockerId: "sample-prep",
      },
    ]);
    assert.deepEqual(endsAtChanges, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        eventId: "MOT",
        clear: false,
      },
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        eventId: null,
        clear: true,
      },
    ]);
  } finally {
    await cleanup();
  }
});

test("task inspector can create an event and assign it as the selected task end event", async () => {
  const createdEvents: Array<{ projectRoot: string; eventId: string }> = [];
  const endsAtChanges: Array<{
    projectRoot: string;
    taskId: string;
    eventId: string | null;
    clear: boolean;
  }> = [];
  const initialProject = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["START"],
    workflow: workflowFixture({
      events: ["START"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START"],
        }),
      ],
    }),
    tasks: [
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
      }),
    ],
  });
  const eventCreatedProject = projectFixture({
    ...initialProject,
    events: ["START", "Data archived"],
    workflow: workflowFixture({
      events: ["START", "Data archived"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START", "Data archived"],
        }),
      ],
    }),
  });
  const eventAssignedProject = projectFixture({
    ...eventCreatedProject,
    tasks: [
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
        endsAt: "Data archived",
      }),
    ],
  });

  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => initialProject,
      onboardProjectAction: async () => initialProject
    },
    taskMutation: {
      setTaskEndsAtAction: async (projectRoot, taskId, eventId, clear) => {
        endsAtChanges.push({ projectRoot, taskId, eventId, clear });
        return { project: eventAssignedProject };
      }
    },
    eventMutation: {
      createEventAction: async (projectRoot, eventId) => {
        createdEvents.push({ projectRoot, eventId });
        return { project: eventCreatedProject };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    getByRole(root, "button", /^Create event and set as end event$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();

    const eventName = getByRole(root, "textbox", /^Event name$/) as HTMLInputElement;
    setNativeValue(eventName, "Data archived");
    eventName.dispatchEvent(new window.Event("input", { bubbles: true }));
    getByRole(root, "button", /^Create Event$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();
    await waitForHandlers();

    assert.deepEqual(createdEvents, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        eventId: "Data archived",
      },
    ]);
    assert.deepEqual(endsAtChanges, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        eventId: "Data archived",
        clear: false,
      },
    ]);
    assert.equal(
      getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
      "true",
    );
    assert.equal(queryByAccessibleLabel(root, "Event inspector"), null);
    assert.ok(queryByAccessibleLabel(root, "Task inspector"));
    assert.equal(
      (getByRole(root, "combobox", /^End at event$/) as HTMLSelectElement).value,
      "Data archived",
    );
  } finally {
    await cleanup();
  }
});

test("task inspector create-event-and-set reports assignment failure without showing a false task end event", async () => {
  const createdEvents: Array<{ projectRoot: string; eventId: string }> = [];
  const endsAtChanges: Array<{
    projectRoot: string;
    taskId: string;
    eventId: string | null;
    clear: boolean;
  }> = [];
  const initialProject = projectFixture({
    selectedPath: "/tmp/fixture-spielgantt/workflow",
    projectRoot: "/tmp/fixture-spielgantt/workflow",
    events: ["START"],
    workflow: workflowFixture({
      events: ["START"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START"],
        }),
      ],
    }),
    tasks: [
      taskFixture("analysis-run", {
        path: "/tmp/fixture-spielgantt/workflow/analysis-run",
      }),
    ],
  });
  const eventCreatedProject = projectFixture({
    ...initialProject,
    events: ["START", "Data archived"],
    workflow: workflowFixture({
      events: ["START", "Data archived"],
      tasks: [
        workflowTaskFixture("analysis-run", {
          valid_ends_at_targets: ["START", "Data archived"],
        }),
      ],
    }),
  });

  const { root, window, cleanup } = await renderTestShell({
    projectLifecycle: {
      openSelectedProject: async () => initialProject,
      onboardProjectAction: async () => initialProject
    },
    taskMutation: {
      setTaskEndsAtAction: async (projectRoot, taskId, eventId, clear) => {
        endsAtChanges.push({ projectRoot, taskId, eventId, clear });
        throw new Error("event would create a cycle");
      }
    },
    eventMutation: {
      createEventAction: async (projectRoot, eventId) => {
        createdEvents.push({ projectRoot, eventId });
        return { project: eventCreatedProject };
      }
    },
    desktop: {
      pickProjectFolder: async () => "/tmp/fixture-spielgantt/workflow"
    }
  });
  try {
    await openProjectFromProjectsMenu(root);
    clickTreeItem(root, /^analysis-run$/);
    await waitForHandlers();

    getByRole(root, "button", /^Create event and set as end event$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();

    const eventName = getByRole(root, "textbox", /^Event name$/) as HTMLInputElement;
    setNativeValue(eventName, "Data archived");
    eventName.dispatchEvent(new window.Event("input", { bubbles: true }));
    getByRole(root, "button", /^Create Event$/).dispatchEvent(
      new window.MouseEvent("click", { bubbles: true, cancelable: true }),
    );
    await waitForHandlers();
    await waitForHandlers();

    assert.deepEqual(createdEvents, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        eventId: "Data archived",
      },
    ]);
    assert.deepEqual(endsAtChanges, [
      {
        projectRoot: "/tmp/fixture-spielgantt/workflow",
        taskId: "analysis-run",
        eventId: "Data archived",
        clear: false,
      },
    ]);
    assert.match(root.textContent ?? "", /Created event 'Data archived'/);
    assert.match(root.textContent ?? "", /could not set it as the task end event/);
    assert.match(root.textContent ?? "", /event would create a cycle/);
    assert.equal(
      getByRole(root, "treeitem", /^analysis-run$/).getAttribute("aria-selected"),
      "true",
    );
    assert.equal(queryByAccessibleLabel(root, "Event inspector"), null);
    assert.equal(
      (getByRole(root, "combobox", /^End at event$/) as HTMLSelectElement).value,
      "",
    );
  } finally {
    await cleanup();
  }
});
