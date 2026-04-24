import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const ignoredDirectoryNames = new Set([
  ".git",
  ".output",
  ".turbo",
  "coverage",
  "dist",
  "node_modules",
  "storybook-static",
  "target",
]);
const allowedTopLevelHiddenDirectories = new Set([".devcontainer", ".github"]);
const supportedExtensions = new Set([
  ".cjs",
  ".css",
  ".cts",
  ".js",
  ".json",
  ".json5",
  ".jsx",
  ".md",
  ".mdx",
  ".mjs",
  ".mts",
  ".scss",
  ".ts",
  ".tsx",
  ".yaml",
  ".yml",
]);
const exactIgnoredFiles = new Set(["apps/console-web/src/routeTree.gen.ts"]);
const comparePaths = (left, right) => left.localeCompare(right);
const baselineJson = JSON.parse(
  readFileSync(
    path.join(repoRoot, "scripts", "prettier-drift-baseline.json"),
    "utf8",
  ),
);
/** @type {{ drift: string[] }} */
const baseline = {
  drift: Array.isArray(baselineJson.drift)
    ? baselineJson.drift.filter((file) => typeof file === "string")
    : [],
};

function toPosixPath(filePath) {
  return filePath.split(path.sep).join(path.posix.sep);
}

function isSupportedFile(relativePath) {
  return supportedExtensions.has(path.extname(relativePath));
}

function collectFiles(rootRelativePath = "") {
  const absoluteRoot = path.join(repoRoot, rootRelativePath);
  const entries = readdirSync(absoluteRoot, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (ignoredDirectoryNames.has(entry.name)) {
        continue;
      }

      if (
        rootRelativePath === "" &&
        entry.name.startsWith(".") &&
        !allowedTopLevelHiddenDirectories.has(entry.name)
      ) {
        continue;
      }

      files.push(...collectFiles(path.join(rootRelativePath, entry.name)));
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    const relativePath = toPosixPath(path.join(rootRelativePath, entry.name));

    if (exactIgnoredFiles.has(relativePath) || !isSupportedFile(relativePath)) {
      continue;
    }

    files.push(relativePath);
  }

  return files;
}

function runPrettierListDifferent(files) {
  if (files.length === 0) {
    return [];
  }

  const batchSize = 100;
  /** @type {Set<string>} */
  const drift = new Set();

  for (let index = 0; index < files.length; index += batchSize) {
    const batch = files.slice(index, index + batchSize);
    const result = spawnSync(
      "pnpm",
      ["exec", "prettier", "--list-different", ...batch],
      {
        cwd: repoRoot,
        encoding: "utf8",
        maxBuffer: 10 * 1024 * 1024,
      },
    );

    if (![0, 1].includes(result.status ?? 1)) {
      console.error(result.stderr || result.stdout || "Prettier check failed.");
      process.exit(result.status ?? 1);
    }

    const lines = (result.stdout || "")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);

    for (const line of lines) {
      if (
        line === "Checking formatting..." ||
        line.startsWith("[warn] Code style issues found")
      ) {
        continue;
      }

      drift.add(line.replace(/^\[warn\]\s*/, ""));
    }
  }

  return [...drift].sort(comparePaths);
}

const candidateFiles = collectFiles().sort(comparePaths);
const actualDrift = runPrettierListDifferent(candidateFiles);
const baselineDrift = [...new Set(baseline.drift)].sort(comparePaths);
const baselineSet = new Set(baselineDrift);
const actualSet = new Set(actualDrift);
const regressions = actualDrift.filter((file) => !baselineSet.has(file));
const staleBaselineEntries = baselineDrift.filter(
  (file) => !actualSet.has(file),
);

if (regressions.length > 0) {
  console.error("Prettier drift baseline check failed.");
  console.error("");
  console.error(
    "New formatting drift was introduced in files outside the approved baseline:",
  );

  for (const file of regressions) {
    console.error(`- ${file}`);
  }

  console.error("");
  console.error(
    'Run "pnpm exec prettier --write <files...>" for the new files, or intentionally extend scripts/prettier-drift-baseline.json if this repository chooses to carry the debt.',
  );
  process.exit(1);
}

console.log(
  `Prettier drift baseline check passed for ${candidateFiles.length} files. ${actualDrift.length} legacy drift entries remain.`,
);

if (staleBaselineEntries.length > 0) {
  console.log("");
  console.log(
    `Baseline cleanup opportunity: ${staleBaselineEntries.length} entries are now formatted and can be removed from scripts/prettier-drift-baseline.json.`,
  );
}
