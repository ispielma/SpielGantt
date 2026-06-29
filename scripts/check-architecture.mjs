import { constants } from "node:fs";
import { access, readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = path.resolve(path.dirname(scriptPath), "..");

function lineBudget(path, maxLines, reason, owner, followUp) {
  return { path, maxLines, reason, owner, followUp };
}

export const lineBudgets = [
  lineBudget(
    "src/project-actions.ts",
    220,
    "project action identity, labels, availability, and command mapping should stay in one focused frontend model",
    "project action model",
    "split native-menu or React menu adaptation into adapters before adding new project action policy",
  ),
  lineBudget(
    "src/task-overlay-workflows.ts",
    340,
    "task overlay submission behavior should stay in task workflow ownership instead of returning to the shell",
    "task overlay workflows",
    "split create/adopt, rename/delete, or dependency edit workflows before adding more task dialog branches",
  ),
  lineBudget(
    "src/event-overlay-workflows.ts",
    340,
    "event overlay submission behavior should stay in event workflow ownership instead of returning to the shell",
    "event overlay workflows",
    "split create, rename, or delete event workflow helpers before adding more event dialog branches",
  ),
  lineBudget(
    "src/main.ts",
    40,
    "frontend shell startup should stay as a lightweight composition root",
    "frontend startup",
    "keep startup limited to bootstrapping and move new runtime behavior into shell modules",
  ),
  lineBudget(
    "src/react-app-shell.tsx",
    300,
    "the React application shell should stay focused on composition, mounting, and layout hosts",
    "React app composition",
    "extract new workflow state into shell controller or focused React panels before adding more shell layout",
  ),
  lineBudget(
    "src/shell-controller.ts",
    1600,
    "project orchestration should delegate package actions, overlay workflows, project session, sidebar navigation, and workspace selection to focused owners",
    "frontend shell orchestration",
    "move any new project-session, menu, dialog, selection, or persistence behavior into the existing owner modules before adding shell branching",
  ),
  lineBudget(
    "src/shell-project-session.ts",
    640,
    "active project state, refresh behavior, remembered projects, and GUI-owned project removal should stay in the session owner",
    "project session owner",
    "split remembered-project persistence or project-open lifecycle adapters if session behavior grows further",
  ),
  lineBudget(
    "src/sidebar-tree-model.ts",
    680,
    "sidebar navigation state and accessible tree commands should stay in the navigator model",
    "sidebar navigator model",
    "split context-menu intent mapping, keyboard navigation, or missing-project shaping before adding more tree policy",
  ),
  lineBudget(
    "src/sidebar-tree-mantine-adapter.ts",
    120,
    "Mantine-specific tree accessibility repair should stay isolated from the navigation model",
    "sidebar Mantine adapter",
    "move any broader navigation policy back into sidebar-tree-model instead of growing adapter logic",
  ),
  lineBudget(
    "src/sidebar-tree.tsx",
    950,
    "the sidebar tree should stay focused on React rendering and project navigation affordances",
    "sidebar tree rendering",
    "extract menu actions, tree model shaping, or section controls before adding sidebar behavior",
  ),
  lineBudget(
    "src/shell-dialogs.tsx",
    760,
    "dialog components should stay split by focused task and event workflows",
    "shell dialog composition",
    "create focused project, task, event, or alignment dialog modules instead of adding broad dialog code",
  ),
  lineBudget(
    "src/task-inspector-panel.tsx",
    360,
    "the task inspector should stay focused on presentation instead of regrowing package logic",
    "task inspector presentation",
    "move reusable inspector sections or controls into focused presentation modules",
  ),
  lineBudget(
    "src/timeline.ts",
    260,
    "timeline view-model building should stay focused on visual layout inputs",
    "timeline view model",
    "split visual placement, rail sizing, or label planning into timeline-owned helpers",
  ),
  lineBudget(
    "src/timeline-panel.tsx",
    260,
    "the timeline React view should stay focused on rendering the visual event axis",
    "timeline React rendering",
    "move non-rendering timeline calculations into visual layout modules",
  ),
  lineBudget(
    "src/work-area-panel.tsx",
    230,
    "the React workspace bridge should stay small and focused on event wiring",
    "workspace panel bridge",
    "extract inspector or timeline dispatch wiring before adding more workspace branching",
  ),
  lineBudget(
    "src/workspace-selection-model.ts",
    300,
    "workspace selection and inspector derivation should stay in a frontend model separate from rendering",
    "workspace selection model",
    "split timeline selection projection or inspector command derivation before adding more selection states",
  ),
  lineBudget(
    "src/shell-overlays.tsx",
    220,
    "overlay event dispatch should stay focused and avoid regrowing ad hoc shell bindings",
    "overlay dispatcher",
    "move workflow-specific overlay transitions into their dialog or menu owners",
  ),
  lineBudget(
    "src-tauri/src/lib.rs",
    2300,
    "Rust command wiring should stay split from graph, snapshot, diagnostics, task, event, and Application adapter modules",
    "Rust crate composition",
    "keep new command behavior in domain modules and expose only thin composition from lib",
  ),
  lineBudget(
    "src-tauri/src/tauri_commands.rs",
    520,
    "Tauri command wrappers should stay thin over shared Rust/core capabilities",
    "Tauri command adapters",
    "move domain behavior into CLI/core modules and leave wrappers as request translation",
  ),
  lineBudget(
    "src-tauri/src/task.rs",
    2250,
    "broad task orchestration should delegate focused mutation groups such as relative insertion to owner modules",
    "task core orchestration",
    "extract mutation groups behind task package index or domain-specific task modules",
  ),
  lineBudget(
    "src-tauri/src/semantic_projection.rs",
    140,
    "shared semantic projections should centralize snapshot, relationship, and workflow loading without absorbing command behavior",
    "semantic projection loader",
    "split projection caching or projection-specific error translation before adding command concerns",
  ),
  lineBudget(
    "src-tauri/src/task_package_index.rs",
    500,
    "task package indexing should centralize project root, metadata, and task loading without absorbing mutation workflows",
    "task package index",
    "keep mutation workflows out of the index and split any new indexing concern by package responsibility",
  ),
  lineBudget(
    "src-tauri/src/task_metadata_mutations.rs",
    760,
    "task metadata mutation workflows should use the package index without regrowing broad task orchestration",
    "task metadata mutations",
    "split unrelated metadata mutation flows by command family before adding new behavior",
  ),
  lineBudget(
    "src-tauri/src/relative_task_insert.rs",
    430,
    "relative insert-before/insert-after mutation ownership should stay isolated from the broad task module",
    "relative task insertion",
    "extract ordering validation or filesystem move helpers if relative insertion grows further",
  ),
  lineBudget(
    "src-tauri/src/event.rs",
    800,
    "event behavior should stay focused",
    "event core behavior",
    "split event mutation, listing, or reference validation into focused modules before adding event features",
  ),
  lineBudget(
    "src-tauri/src/project_graph.rs",
    1100,
    "dependency and event-axis semantics should stay centralized without growing into unrelated behavior",
    "project graph semantics",
    "split graph diagnostics or workflow projection helpers while keeping semantics in Rust/core",
  ),
  lineBudget(
    "src-tauri/src/app_facade.rs",
    900,
    "Application facade should stay explicit over package capabilities and app payload assembly",
    "Application API facade",
    "route new payload or lifecycle behavior through project_payload, project_lifecycle, or action adapters",
  ),
  lineBudget(
    "src-tauri/src/cli/agent.rs",
    420,
    "agent CLI commands should stay grouped by agent-facing package preparation and runtime diagnostics",
    "agent CLI command family",
    "split output rendering or preparation helpers before adding unrelated agent commands",
  ),
  lineBudget(
    "src-tauri/src/cli/event.rs",
    180,
    "event CLI commands should stay as thin adapters over Rust/core event behavior",
    "event CLI command family",
    "move event mutation or reference semantics into core event modules before adding CLI branching",
  ),
  lineBudget(
    "src-tauri/src/cli/mod.rs",
    160,
    "CLI module composition should stay limited to command family routing and shared CLI utilities",
    "CLI composition",
    "split shared output helpers or command-family modules before adding new routing concerns",
  ),
  lineBudget(
    "src-tauri/src/cli/package.rs",
    280,
    "package CLI commands should stay as package-level JSON and human-output adapters",
    "package CLI command family",
    "move package loading or validation semantics into Rust/core modules before adding CLI logic",
  ),
  lineBudget(
    "src-tauri/src/cli/project.rs",
    120,
    "project CLI commands should stay as thin adapters over project package behavior",
    "project CLI command family",
    "move project mutation semantics into Rust/core project modules before adding CLI branching",
  ),
  lineBudget(
    "src-tauri/src/cli/task.rs",
    900,
    "task CLI commands should stay as thin adapters over task package behavior and JSON contracts",
    "task CLI command family",
    "split task command output rendering or subcommand adapters before adding more task command behavior",
  ),
  lineBudget(
    "src-tauri/src/project_payload.rs",
    600,
    "Open-project payload construction should stay focused on serializable Rust-owned project data",
    "Project payload builder",
    "extract reusable serialization or README/task/event payload helpers before adding more payload fields",
  ),
  lineBudget(
    "src-tauri/src/project_lifecycle.rs",
    600,
    "Project lifecycle adapters should stay focused on open, refresh, onboarding, and agent preparation",
    "Project lifecycle",
    "separate onboarding, refresh, and watcher lifecycle flows if more lifecycle behavior is added",
  ),
  lineBudget(
    "src-tauri/src/task_actions.rs",
    720,
    "Application task action adapters should translate Tauri requests to Rust/core without regrowing broad project payload code",
    "Application task action adapters",
    "move shared task-action payload refresh or mutation helpers into focused Rust/core modules",
  ),
  lineBudget(
    "src-tauri/src/task_edit_action.rs",
    300,
    "task edit application behavior should own README/version rollback without absorbing unrelated task actions",
    "task edit action",
    "split reusable rewrite transactions or readme/version concerns if edit behavior grows further",
  ),
  lineBudget(
    "tests/frontend-smoke.test.ts",
    380,
    "frontend smoke tests should stay at durable interaction seams instead of pinning slice implementation details",
    "frontend smoke coverage",
    "move feature assertions into behavior-specific frontend tests and keep smoke coverage to boot/navigation",
  ),
  lineBudget(
    "tests/support/frontend-shell.ts",
    350,
    "frontend test support should stay as shared fixtures instead of becoming another test suite",
    "frontend test harness",
    "move accessible queries, fixtures, or adapters into focused support modules before adding helpers",
  ),
];

const forbiddenFrontendPatterns = [
  { label: "buildEventGraph", pattern: /\bbuildEventGraph\b/ },
  { label: "topologicalSort", pattern: /\btopologicalSort\b/ },
  { label: "detectEventCycle", pattern: /\bdetectEventCycle\b/ },
  { label: "eventGraph", pattern: /\beventGraph\b/ },
  { label: "validDependencyTargets", pattern: /\bvalidDependencyTargets\b/ },
  { label: "validEndsAtTargets", pattern: /\bvalidEndsAtTargets\b/ },
  { label: "classifyWorkflowDiagnostics", pattern: /\bclassifyWorkflowDiagnostics\b/ },
];

const frontendVisualLayoutFiles = new Set(["src/timeline.ts", "src/timeline-placement.ts"]);

const rustAppPayloadFiles = new Set([
  "src-tauri/src/app_facade.rs",
  "src-tauri/src/project_actions.rs",
  "src-tauri/src/project_lifecycle.rs",
  "src-tauri/src/project_payload.rs",
  "src-tauri/src/task_actions.rs",
  "src-tauri/src/task_edit_action.rs",
]);

const forbiddenRustAppPayloadVisualFields = [
  "row",
  "lane",
  "grid_column",
  "grid_row",
  "pixel_x",
  "pixel_y",
  "css",
  "css_class",
  "css_style",
  "connector",
  "connector_path",
  "connector_points",
  "label_placement",
  "label_offset",
];

const tauriCommandParity = new Map([
  ["spielgantt_health", "CLI JSON parity: spielgantt agent runtime --json"],
  [
    "spielgantt_open_project",
    "CLI JSON parity: agent snapshot --json, validate --json, task relationships --json, task workflow --json",
  ],
  [
    "spielgantt_refresh_project",
    "CLI JSON parity: agent snapshot --json, validate --json, task relationships --json, task workflow --json",
  ],
  ["spielgantt_edit_project_readme", "CLI JSON parity: spielgantt project update-readme --json"],
  ["spielgantt_dependency_relationships", "CLI JSON parity: spielgantt task relationships --json"],
  ["spielgantt_onboard_project", "CLI JSON parity: spielgantt agent prepare --json"],
  ["spielgantt_create_project", "CLI parity: spielgantt init; JSON follow-up via agent snapshot --json"],
  ["spielgantt_prepare_agent_scaffolding", "CLI JSON parity: spielgantt agent prepare --json"],
  ["spielgantt_create_task", "CLI parity: spielgantt task create; JSON follow-up via task list --json"],
  ["spielgantt_insert_task_before", "CLI JSON parity: spielgantt task insert-before --json"],
  ["spielgantt_insert_task_after", "CLI JSON parity: spielgantt task insert-after --json"],
  ["spielgantt_create_event", "CLI parity: spielgantt event create; JSON follow-up via event list --json"],
  ["spielgantt_delete_event", "CLI parity: spielgantt event delete; JSON follow-up via task relationships --json"],
  ["spielgantt_adopt_task", "CLI parity: spielgantt task adopt; JSON follow-up via task list --json"],
  ["spielgantt_list_adoptable_task_folders", "CLI JSON parity: spielgantt task adoptable-folders --json"],
  ["spielgantt_preview_task_normalization", "documented exception: normalize preview is human CLI output"],
  [
    "spielgantt_preview_task_folder_alignment",
    "documented exception: normalize preview is human CLI output",
  ],
  ["spielgantt_apply_task_folder_alignment", "CLI parity: spielgantt normalize --apply"],
  ["spielgantt_apply_task_normalization", "CLI parity: spielgantt normalize --apply"],
  ["spielgantt_edit_task", "CLI parity: spielgantt task update; JSON follow-up via task show --json"],
  ["spielgantt_add_task_dependency", "CLI parity: spielgantt task depend; JSON follow-up via task relationships --json"],
  ["spielgantt_rename_task", "CLI parity: spielgantt task rename; JSON follow-up via task list --json"],
  ["spielgantt_delete_task", "CLI parity: spielgantt task delete --json"],
  ["spielgantt_rename_event", "CLI parity: spielgantt event rename; JSON follow-up via event list --json"],
  [
    "spielgantt_remove_task_dependency",
    "CLI parity: spielgantt task dependency remove; JSON follow-up via task relationships --json",
  ],
  ["spielgantt_set_task_ends_at", "CLI parity: spielgantt task ends-at; JSON follow-up via task workflow --json"],
]);

function parseArgs(argv) {
  const args = { root: repoRoot };

  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (flag === "--root") {
      if (!value) {
        throw new Error("missing value for --root");
      }
      args.root = path.resolve(value);
      index += 1;
    } else {
      throw new Error(`unknown argument: ${flag}`);
    }
  }

  return args;
}

async function pathExists(filePath) {
  try {
    await access(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

function countLines(source) {
  if (source.length === 0) {
    return 0;
  }
  return source.endsWith("\n") ? source.split("\n").length - 1 : source.split("\n").length;
}

async function collectTypescriptFiles(root) {
  if (!(await pathExists(root))) {
    return [];
  }

  const entries = await readdir(root, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectTypescriptFiles(fullPath)));
    } else if (
      entry.isFile() &&
      (entry.name.endsWith(".ts") || entry.name.endsWith(".tsx"))
    ) {
      files.push(fullPath);
    }
  }

  return files;
}

async function collectSourceFiles(root, extensions) {
  if (!(await pathExists(root))) {
    return [];
  }

  const entries = await readdir(root, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectSourceFiles(fullPath, extensions)));
    } else if (entry.isFile() && extensions.some((extension) => entry.name.endsWith(extension))) {
      files.push(fullPath);
    }
  }

  return files;
}

async function checkLineBudgets(root) {
  const failures = [];

  for (const budget of lineBudgets) {
    const filePath = path.join(root, budget.path);
    if (!(await pathExists(filePath))) {
      failures.push(`architecture budget target is missing: ${budget.path}`);
      continue;
    }

    const source = await readFile(filePath, "utf8");
    const lineCount = countLines(source);
    if (lineCount > budget.maxLines) {
      failures.push(
        `${budget.path} has ${lineCount} lines, budget is ${budget.maxLines}: ${budget.reason}`,
      );
    }
  }

  return failures;
}

async function checkLineBudgetWarnings(root) {
  const warnings = [];

  for (const budget of lineBudgets) {
    const filePath = path.join(root, budget.path);
    if (!(await pathExists(filePath))) {
      continue;
    }

    const source = await readFile(filePath, "utf8");
    const lineCount = countLines(source);
    const warningThreshold = Math.ceil(budget.maxLines * 0.8);
    if (lineCount >= warningThreshold && lineCount <= budget.maxLines) {
      warnings.push(
        `${budget.path} has ${lineCount} lines, warning threshold is ${warningThreshold}, budget is ${budget.maxLines}: ${budget.reason}; owner: ${budget.owner}; follow-up: ${budget.followUp}`,
      );
    }
  }

  return warnings;
}

async function checkFrontendSemanticAlgorithmOwnership(root) {
  const failures = [];
  const frontendFiles = await collectTypescriptFiles(path.join(root, "src"));

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const lines = source.split("\n");
    for (const forbidden of forbiddenFrontendPatterns) {
      const lineIndex = lines.findIndex((line) => forbidden.pattern.test(line));
      if (lineIndex !== -1) {
        failures.push(
          `frontend dependency/event semantic algorithms must come from Rust contracts; found ${forbidden.label} in ${path.relative(root, filePath)}:${lineIndex + 1}`,
        );
      }
    }
  }

  return failures;
}

async function checkFrontendSemanticOwnership(root) {
  const failures = [];
  const frontendFiles = await collectTypescriptFiles(path.join(root, "src"));

  for (const filePath of frontendFiles) {
    const relativePath = path.relative(root, filePath);
    if (frontendVisualLayoutFiles.has(relativePath)) {
      continue;
    }

    const source = await readFile(filePath, "utf8");
    if (/\bdependency_references\b[\s\S]*\bends_at_reference\b/.test(source)) {
      failures.push(
        `frontend event-reference semantics must come from Rust contracts; found workflow reference scan in ${relativePath}`,
      );
    }
    if (/\btasks\b[\s\S]*\.filter\([\s\S]*\bdependencies\b[\s\S]*\.includes\(/.test(source)) {
      failures.push(
        `frontend dependency semantics must come from Rust contracts; found reverse dependency scan in ${relativePath}`,
      );
    }
    if (/\b(unresolved_references|invalid_references|validation_diagnostics)\b[\s\S]*\.(some|filter|find)\(/.test(source)) {
      failures.push(
        `frontend workflow validation semantics must come from Rust contracts; found diagnostic classification in ${relativePath}`,
      );
    }
  }

  return failures;
}

async function checkFrontendTaskContractFields(root) {
  const shellTypesPath = path.join(root, "src", "shell-types.ts");
  if (!(await pathExists(shellTypesPath))) {
    return [];
  }

  const source = await readFile(shellTypesPath, "utf8");
  const requiredProjectFields = ["events", "eventReferences", "workflow"];
  const requiredTaskFields = [
    "blocks",
    "dependencyReferences",
    "dependencyTargets",
    "readmeContent",
    "readmeVersion",
  ];
  const optionalProjectFields = requiredProjectFields.filter((field) =>
    new RegExp(`\\b${field}\\?\\s*:`).test(source),
  );
  const optionalFields = requiredTaskFields.filter((field) =>
    new RegExp(`\\b${field}\\?\\s*:`).test(source),
  );
  const failures = [];

  if (optionalProjectFields.length > 0) {
    failures.push(
      `frontend project contract fields guaranteed by Rust must be required in src/shell-types.ts: ${optionalProjectFields.join(", ")}`,
    );
  }

  if (optionalFields.length > 0) {
    failures.push(
      `frontend task contract fields guaranteed by Rust must be required in src/shell-types.ts: ${optionalFields.join(", ")}`,
    );
  }

  return failures;
}

async function checkLegacyShellRendering(root) {
  const failures = [];
  const frontendFiles = await collectSourceFiles(path.join(root, "src"), [".ts", ".tsx"]);
  const activeShellStringRenderingPattern =
    /\b(?:innerHTML|outerHTML)\s*=\s*renderToStaticMarkup\(|\binsertAdjacentHTML\([\s\S]*?renderToStaticMarkup\(/;
  const shellMarkupAssignments = [];
  const renderToStaticMarkupFunctionPattern =
    /export\s+function\s+([A-Za-z0-9_]+)\s*\([^)]*\)\s*\{[\s\S]*?renderToStaticMarkup\(/g;

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);

    for (const match of source.matchAll(renderToStaticMarkupFunctionPattern)) {
      shellMarkupAssignments.push({
        filePath: relativePath,
        functionName: match[1],
      });
    }
  }

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    const referencesAssignedShellMarkup = shellMarkupAssignments.filter(({ functionName }) =>
      new RegExp(`\\b(?:innerHTML|outerHTML)\\s*=\\s*${functionName}\\(|\\binsertAdjacentHTML\\([\\s\\S]*?${functionName}\\(`).test(
        source,
      ),
    );
    if (
      /\b(renderShellHtml|renderShellMarkup)\b/.test(source) ||
      activeShellStringRenderingPattern.test(source) ||
      referencesAssignedShellMarkup.length > 0
    ) {
      failures.push(
        `legacy shell string rendering must stay removed from the active frontend; found legacy shell string rendering in ${relativePath}`,
      );
    }

    for (const assignment of referencesAssignedShellMarkup) {
      failures.push(
        `legacy shell string rendering must stay removed from the active frontend; found legacy shell string rendering in ${assignment.filePath}`,
      );
    }
  }

  return failures;
}

async function checkBroadPostRenderDomBinding(root) {
  const failures = [];
  const frontendFiles = await collectSourceFiles(path.join(root, "src"), [".ts", ".tsx"]);
  const broadBindingPattern =
    /\bquerySelectorAll(?:<[^>]+>)?\([\s\S]*?\)\.forEach\([\s\S]*?\baddEventListener\(/;
  const queriedNodeListBindingPattern =
    /\bquerySelectorAll(?:<[^>]+>)?\([\s\S]*?\)[\s\S]*?\baddEventListener\(/;

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    if (broadBindingPattern.test(source) || queriedNodeListBindingPattern.test(source)) {
      failures.push(
        `broad post-render DOM binding must stay removed from the active frontend; found broad post-render DOM binding in ${relativePath}`,
      );
    }
  }

  return failures;
}

async function checkProductionStaticMarkupRenderHelpers(root) {
  const failures = [];
  const frontendFiles = await collectSourceFiles(path.join(root, "src"), [".ts", ".tsx"]);

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    if (/\brenderToStaticMarkup\b/.test(source)) {
      failures.push(
        `production static markup render helpers must stay out of active frontend source; found renderToStaticMarkup in ${relativePath}`,
      );
    }
  }

  return failures;
}

async function checkYamlMetadataWrites(root) {
  const failures = [];
  const productionRoots = [path.join(root, "src"), path.join(root, "src-tauri", "src")];
  const productionFiles = (
    await Promise.all(
      productionRoots.map((sourceRoot) => collectSourceFiles(sourceRoot, [".ts", ".rs"])),
    )
  ).flat();

  for (const filePath of productionFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    if (
      /\.spielgantt[\s\S]*\.ya?ml\b/.test(source) ||
      /\.ya?ml\b[\s\S]*(write|serialize|to_string|dump)/i.test(source)
    ) {
      failures.push(
        `production code must not perform YAML metadata writes; found YAML metadata writes in ${relativePath}`,
      );
    }
  }

  return failures;
}

async function checkRustFrontendSessionState(root) {
  const failures = [];
  const rustFiles = await collectSourceFiles(path.join(root, "src-tauri", "src"), [".rs"]);
  const forbiddenPatterns = [
    {
      label: "Tauri window event or bounds state",
      pattern: /\b(WindowEvent|outer_position|outer_size|set_position|set_size)\b/,
    },
    {
      label: "Tauri plugin-store frontend/session persistence",
      pattern: /\btauri_plugin_store\b|\bStoreExt\b/,
    },
    {
      label: "remembered-project frontend session commands",
      pattern: /\bspielgantt_(list|upsert|remove)_remembered_project(s)?\b/,
    },
    {
      label: "project-watch frontend refresh commands",
      pattern: /\bspielgantt_(un)?watch_project\b/,
    },
    {
      label: "OS folder opener frontend commands",
      pattern: /\bspielgantt_open_(project|task)_folder\b/,
    },
  ];

  for (const filePath of rustFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    for (const forbidden of forbiddenPatterns) {
      if (forbidden.pattern.test(source)) {
        failures.push(
          `Frontend session state must stay out of Rust backend; found ${forbidden.label} in ${relativePath}`,
        );
      }
    }
  }

  return failures;
}

async function checkRustAppPayloadVisualFields(root) {
  const failures = [];

  for (const relativePath of rustAppPayloadFiles) {
    const filePath = path.join(root, relativePath);
    if (!(await pathExists(filePath))) {
      continue;
    }

    const source = await readFile(filePath, "utf8");
    const lines = source.split("\n");
    for (const field of forbiddenRustAppPayloadVisualFields) {
      const fieldPattern = new RegExp(`\\b(?:pub\\s+)?${field}\\s*:`);
      const lineIndex = lines.findIndex((line) => fieldPattern.test(line));
      if (lineIndex !== -1) {
        failures.push(
          `Rust app payloads must not expose visual layout fields; found ${field} in ${relativePath}:${lineIndex + 1}`,
        );
      }
    }
  }

  return failures;
}

async function checkFrontendTestDurability(root) {
  const failures = [];
  const frontendTests = await collectSourceFiles(path.join(root, "tests"), [".ts", ".tsx"]);
  const mantineInternalClassPattern = /\.mantine-[A-Za-z0-9-]+\b/;

  for (const filePath of frontendTests) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    const match = source.match(mantineInternalClassPattern);
    if (match) {
      failures.push(
        `frontend tests must not pin Mantine internal CSS classes; found ${match[0]} in ${relativePath}`,
      );
    }
  }

  return failures;
}

async function checkTauriCommandParity(root) {
  const rustSources = await collectSourceFiles(path.join(root, "src-tauri", "src"), [".rs"]);
  const commandPattern = /#\[tauri::command\]\s*(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z0-9_]+)/g;
  const failures = [];

  for (const sourcePath of rustSources) {
    const source = await readFile(sourcePath, "utf8");
    for (const match of source.matchAll(commandPattern)) {
      const commandName = match[1];
      if (!tauriCommandParity.has(commandName)) {
        failures.push(
          `Tauri command parity is undocumented for ${commandName}; add CLI JSON parity or a documented frontend-only exception`,
        );
      }
    }
  }

  return failures;
}

async function checkTrivialCompatibilityShims(root) {
  const failures = [];
  const frontendFiles = (
    await Promise.all([
      collectSourceFiles(path.join(root, "src"), [".ts", ".tsx"]),
      collectSourceFiles(path.join(root, "tests"), [".ts", ".tsx"]),
    ])
  ).flat();
  const rustFiles = await collectSourceFiles(path.join(root, "src-tauri", "src"), [".rs"]);

  // These checks target only old transition shims removed during MVP cleanup.
  // Allowed boundaries:
  // - Tauri command wrappers remain thin ABI adapters over Rust/core contracts.
  // - Frontend invoke wrappers may centralize Tauri command names and payloads.
  // - Persisted metadata aliases, such as serde aliases for legacy data values,
  //   are durable data compatibility rather than old-call-site support.
  // - Public module re-exports may compose stable module APIs when they are not
  //   resurrecting removed alias names such as app_api or shared.
  const frontendPatterns = [
    {
      label: "LegacyShellDeps",
      pattern: /\bLegacyShellDeps\b/,
    },
    {
      label: "legacy flat frontend shell dependency mapper",
      pattern: /\b(?:mapLegacyShellDeps|legacyShellDeps|buildLegacyShellDeps)\b/,
    },
    {
      label: "watchProjectChanges",
      pattern: /\bwatchProjectChanges\b/,
    },
    {
      label: "shared",
      pattern: /\bimport\s+(?:type\s+)?(?:[^'"]+\s+from\s+)?["']shared(?:\/[^"']*)?["']|export\s+[^'"]+\s+from\s+["']shared(?:\/[^"']*)?["']/,
    },
    {
      label: "AppFacade",
      pattern: /\b(?:export\s+)?(?:const|let|var)\s+AppFacade\s*=\s*\{/,
    },
  ];
  const rustPatterns = [
    {
      label: "app_api",
      pattern: /\bapp_api\b/,
    },
    {
      label: "AppFacade",
      pattern: /\b(?:pub\s+)?struct\s+AppFacade\s*(?:;|\{\s*\})/,
    },
  ];

  for (const filePath of frontendFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    const lines = source.split("\n");
    for (const legacy of frontendPatterns) {
      const lineIndex = lines.findIndex((line) => legacy.pattern.test(line));
      if (lineIndex !== -1) {
        failures.push(
          `trivial compatibility shims must stay removed; found ${legacy.label} in ${relativePath}:${lineIndex + 1}`,
        );
      }
    }
  }

  for (const filePath of rustFiles) {
    const source = await readFile(filePath, "utf8");
    const relativePath = path.relative(root, filePath);
    const lines = source.split("\n");
    for (const legacy of rustPatterns) {
      const lineIndex = lines.findIndex((line) => legacy.pattern.test(line));
      if (lineIndex !== -1) {
        failures.push(
          `trivial compatibility shims must stay removed; found ${legacy.label} in ${relativePath}:${lineIndex + 1}`,
        );
      }
    }
  }

  for (const configPath of ["tsconfig.json", "tsconfig.node.json", "vite.config.ts"]) {
    const filePath = path.join(root, configPath);
    if (!(await pathExists(filePath))) {
      continue;
    }

    const source = await readFile(filePath, "utf8");
    const lines = source.split("\n");
    const lineIndex = lines.findIndex((line) => /["']shared(?:\/\*)?["']\s*:/.test(line));
    if (lineIndex !== -1) {
      failures.push(
        `trivial compatibility shims must stay removed; found shared alias in ${configPath}:${lineIndex + 1}`,
      );
    }
  }

  return failures;
}

export async function checkArchitecture(root = repoRoot) {
  return [
    ...(await checkLineBudgets(root)),
    ...(await checkFrontendSemanticAlgorithmOwnership(root)),
    ...(await checkFrontendSemanticOwnership(root)),
    ...(await checkFrontendTaskContractFields(root)),
    ...(await checkLegacyShellRendering(root)),
    ...(await checkBroadPostRenderDomBinding(root)),
    ...(await checkProductionStaticMarkupRenderHelpers(root)),
    ...(await checkYamlMetadataWrites(root)),
    ...(await checkRustFrontendSessionState(root)),
    ...(await checkRustAppPayloadVisualFields(root)),
    ...(await checkFrontendTestDurability(root)),
    ...(await checkTauriCommandParity(root)),
    ...(await checkTrivialCompatibilityShims(root)),
  ];
}

export async function checkArchitectureWarnings(root = repoRoot) {
  return checkLineBudgetWarnings(root);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const failures = await checkArchitecture(args.root);
  const warnings = await checkArchitectureWarnings(args.root);

  if (warnings.length > 0) {
    console.log(`Architecture guard yellow-zone line budget warnings:\n${warnings.join("\n")}`);
  }

  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exitCode = 1;
  } else {
    console.log(`Architecture guard passed: ${args.root}`);
  }
}

if (process.argv[1] === scriptPath) {
  try {
    await main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
