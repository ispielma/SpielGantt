import { spawn } from "node:child_process";
import { constants } from "node:fs";
import { access, mkdir, mkdtemp, readdir, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { checkArchitecture } from "./check-architecture.mjs";

const repoRoot = path.resolve(import.meta.dirname, "..");
const defaultAppPath = path.join(
  repoRoot,
  "src-tauri",
  "target",
  "release",
  "bundle",
  "macos",
  "SpielGantt.app",
);
const brandedAppBundleName = "SpielGantt.app";
const defaultDmgDir = path.join(repoRoot, "src-tauri", "target", "release", "bundle", "dmg");

const forbiddenPathParts = new Set([
  ".cache",
  ".git",
  ".spielgantt",
  "__fixtures__",
  "fixtures",
  "node_modules",
  "target",
  "tests",
]);

const localPathPatterns = [
  /\/Users\/[^/\s<>"']+/,
  /\/private\/(?:tmp|var)\/[^\s<>"']+/,
  /[A-Za-z]:\\Users\\[^\\\s<>"']+/,
];

const defaultReleaseNotesPath = path.join(repoRoot, "docs", "release-candidate.md");

const agentTemplateFiles = [
  "AGENTS.md",
  ".agents/skills/use-spielgantt/SKILL.md",
  ".agents/skills/setup-spielgantt/SKILL.md",
  ".agents/skills/update-spielgantt/SKILL.md",
  ".agents/skills/review-spielgantt/SKILL.md",
];

function parseArgs(argv) {
  const args = {
    appPath: defaultAppPath,
    dmgPath: null,
    releaseNotesPath: defaultReleaseNotesPath,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (flag === "--app") {
      if (!value) {
        throw new Error("missing value for --app");
      }
      args.appPath = path.resolve(value);
      index += 1;
    } else if (flag === "--dmg") {
      if (!value) {
        throw new Error("missing value for --dmg");
      }
      args.dmgPath = path.resolve(value);
      index += 1;
    } else if (flag === "--release-notes") {
      if (!value) {
        throw new Error("missing value for --release-notes");
      }
      args.releaseNotesPath = path.resolve(value);
      index += 1;
    } else {
      throw new Error(`unknown argument: ${flag}`);
    }
  }

  return args;
}

async function findDefaultDmgPath() {
  if (!(await pathExists(defaultDmgDir))) {
    return null;
  }

  const entries = await readdir(defaultDmgDir, { withFileTypes: true });
  const candidates = [];
  for (const entry of entries) {
    if (!entry.isFile() || !/^SpielGantt_.*\.dmg$/.test(entry.name)) {
      continue;
    }
    const filePath = path.join(defaultDmgDir, entry.name);
    candidates.push({ filePath, mtimeMs: (await stat(filePath)).mtimeMs });
  }

  candidates.sort((left, right) => right.mtimeMs - left.mtimeMs);
  return candidates[0]?.filePath ?? null;
}

async function pathExists(filePath) {
  try {
    await access(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function isLikelyText(filePath) {
  const buffer = await readFile(filePath);
  if (buffer.includes(0)) {
    return null;
  }
  return buffer.toString("utf8");
}

async function collectFiles(root) {
  const entries = await readdir(root, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(fullPath)));
    } else if (entry.isFile()) {
      files.push(fullPath);
    }
  }

  return files;
}

async function collectExistingReleaseInputFiles() {
  const inputs = [
    "package.json",
    "package-lock.json",
    "tsconfig.json",
    "vite.config.ts",
    "index.html",
    "src",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    "src-tauri/build.rs",
    "src-tauri/Info.plist",
    "src-tauri/tauri.conf.json",
    "src-tauri/capabilities",
    "src-tauri/agent-templates",
    "src-tauri/src",
    "scripts/package-macos-dmg.mjs",
    "scripts/verify-release.mjs",
    "docs/release-candidate.md",
  ];
  const files = [];

  for (const relativePath of inputs) {
    const inputPath = path.join(repoRoot, relativePath);
    if (!(await pathExists(inputPath))) {
      continue;
    }
    const inputStat = await stat(inputPath);
    if (inputStat.isDirectory()) {
      files.push(...(await collectFiles(inputPath)));
    } else if (inputStat.isFile()) {
      files.push(inputPath);
    }
  }

  return files;
}

async function verifyDmgArtifact(dmgPath) {
  const failures = [];

  if (!dmgPath) {
    failures.push(
      `macOS DMG artifact is missing: ${path.join(defaultDmgDir, "SpielGantt_*.dmg")}`,
    );
    return failures;
  }

  if (!dmgPath.endsWith(".dmg")) {
    failures.push(`expected a macOS DMG artifact path, got ${dmgPath}`);
  }
  if (!/^SpielGantt_.*\.dmg$/.test(path.basename(dmgPath))) {
    failures.push(
      `expected branded macOS DMG artifact 'SpielGantt_*.dmg', got ${path.basename(dmgPath)}`,
    );
  }

  if (!(await pathExists(dmgPath))) {
    failures.push(`macOS DMG artifact does not exist: ${dmgPath}`);
    return failures;
  }

  const dmgStat = await stat(dmgPath);
  if (!dmgStat.isFile()) {
    failures.push(`macOS DMG artifact is not a file: ${dmgPath}`);
    return failures;
  }

  const newestInputMtime = await newestMtimeMs(await collectExistingReleaseInputFiles());
  if (dmgStat.mtimeMs + 1000 < newestInputMtime) {
    failures.push(
      `stale macOS DMG artifact: ${dmgPath} is older than release inputs; run npm run release:build`,
    );
  }

  return failures;
}

async function newestMtimeMs(files) {
  let newest = 0;
  for (const file of files) {
    const fileStat = await stat(file);
    newest = Math.max(newest, fileStat.mtimeMs);
  }
  return newest;
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd: options.cwd ?? repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      resolve({ status: null, stdout, stderr: error.message });
    });
    child.on("close", (status) => resolve({ status, stdout, stderr }));
  });
}

function parseJson(stdout, description, failures) {
  try {
    return JSON.parse(stdout);
  } catch (error) {
    failures.push(
      `${description} did not emit valid JSON: ${error instanceof Error ? error.message : String(error)}`,
    );
    return null;
  }
}

async function verifyAgentReadiness(appPath) {
  const failures = [];
  const cliPath = path.join(appPath, "Contents", "MacOS", "spielgantt");
  const resourcesPath = path.join(appPath, "Contents", "Resources", "agent-templates");

  for (const relativePath of agentTemplateFiles) {
    const templatePath = path.join(resourcesPath, relativePath);
    if (!(await pathExists(templatePath))) {
      failures.push(`missing bundled agent template: ${templatePath}`);
    }
  }

  try {
    await access(cliPath, constants.X_OK);
  } catch {
    failures.push(`agent CLI is not executable: ${cliPath}`);
    return failures;
  }

  const runtimeResult = await runCommand(cliPath, ["agent", "runtime", "--json"]);
  if (runtimeResult.status !== 0) {
    failures.push(
      `agent CLI runtime check failed with status ${runtimeResult.status}: ${runtimeResult.stderr || runtimeResult.stdout}`,
    );
    return failures;
  }

  const runtime = parseJson(runtimeResult.stdout, "agent CLI runtime check", failures);
  if (!runtime) {
    return failures;
  }

  if (runtime.executable_path !== cliPath) {
    failures.push(
      `agent CLI runtime resolved '${runtime.executable_path}' instead of bundled CLI '${cliPath}'`,
    );
  }
  if (runtime.package_context?.kind !== "macos_app_bundle") {
    failures.push(
      `agent CLI runtime did not report macOS app bundle context: ${JSON.stringify(runtime.package_context)}`,
    );
  }
  if (runtime.package_context?.package_path !== appPath) {
    failures.push(
      `agent CLI runtime resolved package path '${runtime.package_context?.package_path}' instead of '${appPath}'`,
    );
  }

  const tempRoot = await mkdtemp(path.join(tmpdir(), "spielgantt-agent-package-"));
  const projectPath = path.join(tempRoot, "project");
  await mkdir(projectPath);
  const initResult = await runCommand(cliPath, ["init", projectPath], { cwd: tempRoot });
  if (initResult.status !== 0) {
    failures.push(
      `agent CLI could not initialize a temporary project from the app bundle: ${initResult.stderr || initResult.stdout}`,
    );
    return failures;
  }

  const prepareResult = await runCommand(
    cliPath,
    ["agent", "prepare", "--json", projectPath],
    { cwd: tempRoot },
  );
  if (prepareResult.status !== 0) {
    failures.push(
      `agent CLI could not prepare a temporary project from the app bundle: ${prepareResult.stderr || prepareResult.stdout}`,
    );
    return failures;
  }

  const prepared = parseJson(prepareResult.stdout, "agent CLI prepare check", failures);
  if (!prepared) {
    return failures;
  }

  const expectedProjectFiles = [
    "AGENTS.md",
    ...agentTemplateFiles.slice(1),
    ".spielgantt/agent.json",
  ];
  for (const relativePath of expectedProjectFiles) {
    const generatedPath = path.join(projectPath, relativePath);
    if (!(await pathExists(generatedPath))) {
      failures.push(`agent CLI prepare did not generate ${relativePath}`);
    }
  }
  if (prepared.agent?.ready !== true) {
    failures.push(`agent CLI prepare did not report a ready project: ${prepareResult.stdout}`);
  }
  if (prepared.agent?.recorded_cli_path !== cliPath) {
    failures.push(
      `agent metadata recorded '${prepared.agent?.recorded_cli_path}' instead of bundled CLI '${cliPath}'`,
    );
  }

  return failures;
}

async function verifyAppBundle(appPath) {
  const failures = [];

  if (!appPath.endsWith(".app")) {
    failures.push(`expected a macOS .app bundle path, got ${appPath}`);
  }
  if (path.basename(appPath) !== brandedAppBundleName) {
    failures.push(
      `expected branded macOS app bundle '${brandedAppBundleName}', got ${path.basename(appPath)}`,
    );
  }

  if (!(await pathExists(appPath))) {
    failures.push(`app bundle does not exist: ${appPath}`);
    return failures;
  }

  const appStat = await stat(appPath);
  if (!appStat.isDirectory()) {
    failures.push(`app bundle is not a directory: ${appPath}`);
    return failures;
  }

  const newestInputMtime = await newestMtimeMs(await collectExistingReleaseInputFiles());
  if (appStat.mtimeMs + 1000 < newestInputMtime) {
    failures.push(
      `stale app bundle: ${appPath} is older than release inputs; run npm run release:build`,
    );
  }

  const requiredFiles = [
    path.join(appPath, "Contents", "Info.plist"),
    path.join(appPath, "Contents", "MacOS", "spielgantt"),
  ];

  for (const requiredFile of requiredFiles) {
    if (!(await pathExists(requiredFile))) {
      failures.push(`missing required bundle file: ${requiredFile}`);
    }
  }

  const files = await collectFiles(appPath);
  for (const file of files) {
    const relativePath = path.relative(appPath, file);
    const parts = relativePath.split(path.sep);
    const forbiddenPart = parts.find((part) => forbiddenPathParts.has(part));
    if (forbiddenPart) {
      failures.push(`release artifact includes ${forbiddenPart}: ${relativePath}`);
    }

    const text = await isLikelyText(file);
    if (!text) {
      continue;
    }

    if (text.includes(repoRoot) || localPathPatterns.some((pattern) => pattern.test(text))) {
      failures.push(`release artifact includes a local-only path: ${relativePath}`);
    }
  }

  failures.push(...(await verifyAgentReadiness(appPath)));

  return failures;
}

async function verifyKnownLimitations(releaseNotesPath) {
  const failures = [];

  if (!(await pathExists(releaseNotesPath))) {
    return [
      `known limitations document is missing: ${releaseNotesPath}`,
    ];
  }

  const releaseNotes = await readFile(releaseNotesPath, "utf8");
  const knownLimitationsHeading = /^## Known Limitations\s*$/im;
  if (!knownLimitationsHeading.test(releaseNotes)) {
    failures.push(`known limitations section is missing: ${releaseNotesPath}`);
    return failures;
  }

  const knownLimitationsSection =
    releaseNotes.split(knownLimitationsHeading)[1]?.split(/^## /m)[0] ?? "";
  if (!/^\s*-\s+\S/m.test(knownLimitationsSection)) {
    failures.push(`known limitations section has no documented limitations: ${releaseNotesPath}`);
  }

  return failures;
}

try {
  const args = parseArgs(process.argv.slice(2));
  const dmgPath = args.dmgPath ?? (await findDefaultDmgPath());
  const failures = [
    ...(await checkArchitecture(repoRoot)),
    ...(await verifyAppBundle(args.appPath)),
    ...(await verifyDmgArtifact(dmgPath)),
    ...(await verifyKnownLimitations(args.releaseNotesPath)),
  ];

  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exitCode = 1;
  } else {
    console.log(`Verified architecture guard: ${repoRoot}`);
    console.log(`Verified app bundle: ${args.appPath}`);
    console.log(`Verified macOS DMG artifact: ${dmgPath}`);
    console.log(`Verified known limitations: ${args.releaseNotesPath}`);
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
