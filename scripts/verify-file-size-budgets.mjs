import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

const sourceRoots = [
  "apps",
  "packages",
  "services",
  "crates",
  "infra",
  "scripts",
];
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
const jsLikeExtensions = new Set([
  ".cjs",
  ".cts",
  ".js",
  ".jsx",
  ".mjs",
  ".mts",
  ".ts",
  ".tsx",
]);
const shellLikeExtensions = new Set([".ps1", ".sh"]);

const exactBudgetOverrides = new Map([
  [
    "apps/console-web/src/features/control-plane/service.ts",
    {
      maxLines: 2100,
      reason:
        "Legacy control-plane service aggregator. Keep it from growing while it is split by feature.",
    },
  ],
  [
    "apps/console-web/src/routes/app.providers.tsx",
    {
      maxLines: 800,
      reason:
        "Legacy provider management screen. New provider flows should move into focused subcomponents.",
    },
  ],
  [
    "apps/console-web/src/routes/app.routes.tsx",
    {
      maxLines: 700,
      reason:
        "Legacy routing console screen. Additional workflow surface should be split before expansion.",
    },
  ],
  [
    "apps/console-web/src/test/console-routes.test.tsx",
    {
      maxLines: 900,
      reason:
        "Large integration suite with many route scenarios. Keep it stable until scenarios are partitioned.",
    },
  ],
  [
    "packages/ts-api-client/src/index.ts",
    {
      maxLines: 900,
      reason:
        "API surface aggregator. Hold the line until domain modules are extracted.",
    },
  ],
  [
    "packages/ts-shared-schema/src/index.ts",
    {
      maxLines: 1000,
      reason:
        "Shared schema barrel plus schema declarations. Prevent further accretion while it is modularized.",
    },
  ],
  [
    "crates/core-domain/src/lib.rs",
    {
      maxLines: 1600,
      reason:
        "Core domain types still live in one module. New domain areas should land in dedicated submodules.",
    },
  ],
  [
    "crates/protocol-ir/src/lib.rs",
    {
      maxLines: 1000,
      reason:
        "Protocol IR root now composes focused contract, artifact, docs, and example modules. Keep new contract families out of the root.",
    },
  ],
  [
    "infra/scripts/smoke.sh",
    {
      maxLines: 700,
      reason:
        "Existing end-to-end smoke script already bundles many checks. Additional phases should move to helpers.",
    },
  ],
  [
    "services/control-plane-api/src/lib.rs",
    {
      maxLines: 5600,
      reason:
        "Control-plane API wiring is still oversized. New endpoint groups must move into focused handler modules.",
    },
  ],
  [
    "services/control-plane-api/src/store.rs",
    {
      maxLines: 5750,
      reason:
        "Store implementation is a known monolith. Persistence and domain-specific store code should move into focused modules.",
    },
  ],
  [
    "services/gateway-api/src/lib.rs",
    {
      maxLines: 3350,
      reason:
        "Gateway API wiring is still centralized. New protocols and handlers should be factored into modules.",
    },
  ],
]);

function toPosixPath(filePath) {
  return filePath.split(path.sep).join(path.posix.sep);
}

function isIgnoredFile(relativePath) {
  return (
    relativePath.endsWith("/routeTree.gen.ts") ||
    relativePath === "routeTree.gen.ts"
  );
}

function isJsLikeFile(relativePath) {
  return jsLikeExtensions.has(path.extname(relativePath));
}

function isRustFile(relativePath) {
  return path.extname(relativePath) === ".rs";
}

function isShellFile(relativePath) {
  return shellLikeExtensions.has(path.extname(relativePath));
}

function isTrackedFile(relativePath) {
  return (
    isJsLikeFile(relativePath) ||
    isRustFile(relativePath) ||
    isShellFile(relativePath)
  );
}

function isTestFile(relativePath) {
  return (
    /(\/|^)[^/]+\.(test|spec)\.[cm]?[jt]sx?$/.test(relativePath) ||
    /(^|\/)src\/test\/.+\.[cm]?[jt]sx?$/.test(relativePath)
  );
}

function isRouteScreen(relativePath) {
  return /^apps\/[^/]+\/src\/routes\/.+\.tsx$/.test(relativePath);
}

function countLines(absolutePath) {
  const content = readFileSync(absolutePath, "utf8");

  if (content.length === 0) {
    return 0;
  }

  const newlineMatches = content.match(/\r?\n/g);
  const newlineCount = newlineMatches ? newlineMatches.length : 0;

  return newlineCount + (content.endsWith("\n") ? 0 : 1);
}

function collectFiles(rootRelativePath) {
  const absoluteRoot = path.join(repoRoot, rootRelativePath);
  const entries = readdirSync(absoluteRoot, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    if (ignoredDirectoryNames.has(entry.name)) {
      continue;
    }

    const absolutePath = path.join(absoluteRoot, entry.name);
    const relativePath = toPosixPath(path.relative(repoRoot, absolutePath));

    if (entry.isDirectory()) {
      files.push(...collectFiles(relativePath));
      continue;
    }

    if (
      !entry.isFile() ||
      isIgnoredFile(relativePath) ||
      !isTrackedFile(relativePath)
    ) {
      continue;
    }

    files.push(relativePath);
  }

  return files;
}

function getBudget(relativePath) {
  const exactOverride = exactBudgetOverrides.get(relativePath);

  if (exactOverride) {
    return {
      category: "legacy-override",
      maxLines: exactOverride.maxLines,
      reason: exactOverride.reason,
    };
  }

  if (isTestFile(relativePath)) {
    return {
      category: "test",
      maxLines: 900,
      reason:
        "Tests may be larger, but they still need readable scenario boundaries.",
    };
  }

  if (isRouteScreen(relativePath)) {
    return {
      category: "route-screen",
      maxLines: 600,
      reason:
        "Route entry files should compose smaller view and workflow modules.",
    };
  }

  if (isJsLikeFile(relativePath)) {
    return {
      category: "js-source",
      maxLines: 500,
      reason:
        "Application and library source files should stay focused enough to review and refactor.",
    };
  }

  if (isRustFile(relativePath)) {
    return {
      category: "rust-source",
      maxLines: 800,
      reason:
        "Rust modules larger than this usually hide multiple responsibilities.",
    };
  }

  if (isShellFile(relativePath)) {
    return {
      category: "ops-script",
      maxLines: 300,
      reason:
        "Operational scripts should remain short enough to audit and debug safely.",
    };
  }

  return null;
}

const files = sourceRoots
  .flatMap((root) => collectFiles(root))
  .sort((left, right) => left.localeCompare(right));
const violations = [];

for (const relativePath of files) {
  const budget = getBudget(relativePath);

  if (!budget) {
    continue;
  }

  const absolutePath = path.join(repoRoot, relativePath);
  const lineCount = countLines(absolutePath);

  if (lineCount > budget.maxLines) {
    violations.push({
      relativePath,
      lineCount,
      ...budget,
    });
  }
}

if (violations.length > 0) {
  console.error("File size budget check failed.");
  console.error("");

  for (const violation of violations) {
    console.error(
      `- ${violation.relativePath}: ${violation.lineCount} lines exceeds ${violation.maxLines} (${violation.category})`,
    );
    console.error(`  ${violation.reason}`);
  }

  console.error("");
  console.error(
    "Refactor the file into smaller modules or raise its explicit budget with a documented reason in scripts/verify-file-size-budgets.mjs.",
  );
  process.exit(1);
}

console.log(
  `File size budget check passed for ${files.length} tracked files. ${exactBudgetOverrides.size} legacy overrides remain under watch.`,
);
