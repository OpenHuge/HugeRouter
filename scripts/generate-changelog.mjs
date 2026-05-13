import { execFileSync } from "node:child_process";

function runGit(args) {
  return execFileSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function safeRunGit(args) {
  try {
    return runGit(args);
  } catch {
    return "";
  }
}

const currentTag =
  process.env.GITHUB_REF_NAME ||
  safeRunGit(["describe", "--tags", "--exact-match"]);
const tags = safeRunGit(["tag", "--sort=-creatordate"])
  .split("\n")
  .map((value) => value.trim())
  .filter(Boolean);

const previousTag = currentTag
  ? (tags.find((value) => value !== currentTag) ?? null)
  : (tags[0] ?? null);

const revisionRange = previousTag ? `${previousTag}..HEAD` : "HEAD";
const commitLines = safeRunGit([
  "log",
  revisionRange,
  "--pretty=format:- %h %s (%an)",
]);

const sections = [
  `# HugeRouter Release Notes${currentTag ? ` - ${currentTag}` : ""}`,
  "",
  `Generated on ${new Date().toISOString()}.`,
  "",
];

if (previousTag) {
  sections.push(`Changes since \`${previousTag}\`:`, "");
} else {
  sections.push("Changes included in this release:", "");
}

sections.push(commitLines || "- No commits found for the selected range.", "");
process.stdout.write(sections.join("\n"));
