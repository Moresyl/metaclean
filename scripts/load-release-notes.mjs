import { appendFile, readFile } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const REQUIRED_HEADINGS = ["新功能", "变更与安全", "修复与打磨", "安装", "English summary"];

export function validateReleaseNotes(tag, body) {
  if (!/^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(tag)) throw new Error(`Invalid release tag: ${tag}`);
  if (!body.startsWith(`# MetaClean ${tag}\n`)) throw new Error(`Release notes must start with # MetaClean ${tag}`);
  for (const heading of REQUIRED_HEADINGS) {
    const marker = `### ${heading}\n`;
    const start = body.indexOf(marker);
    const contentStart = start < 0 ? -1 : start + marker.length;
    const nextHeading = contentStart < 0 ? -1 : body.indexOf("\n### ", contentStart);
    const section = contentStart < 0 ? "" : body.slice(contentStart, nextHeading < 0 ? undefined : nextHeading);
    if (!/^\s*-\s+\S/mu.test(section)) throw new Error(`Release notes section is missing concrete bullets: ${heading}`);
  }
  if (/仅见|see changelog|changelog only/iu.test(body)) throw new Error("Release notes cannot delegate all detail to the changelog");
  return body.trimEnd();
}

export async function loadReleaseNotes(tag, root = path.resolve(import.meta.dirname, "..")) {
  const packageMetadata = JSON.parse(await readFile(path.join(root, "package.json"), "utf8"));
  if (tag !== `v${packageMetadata.version}`) {
    throw new Error(`Release tag ${tag} does not match package version v${packageMetadata.version}`);
  }
  const filePath = path.join(root, "release-notes", `${tag}.md`);
  return validateReleaseNotes(tag, await readFile(filePath, "utf8"));
}

async function writeGitHubOutput(body, outputPath) {
  const delimiter = `METACLEAN_RELEASE_${Date.now()}`;
  await appendFile(outputPath, `body<<${delimiter}\n${body}\n${delimiter}\n`, "utf8");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const checkIndex = process.argv.indexOf("--check");
  const tag = checkIndex >= 0 ? process.argv[checkIndex + 1] : process.env.RELEASE_TAG;
  if (!tag) throw new Error("RELEASE_TAG is required");
  const body = await loadReleaseNotes(tag);
  if (checkIndex >= 0) {
    console.log(`Validated release notes for ${tag}.`);
  } else {
    if (!process.env.GITHUB_OUTPUT) throw new Error("GITHUB_OUTPUT is required in release mode");
    await writeGitHubOutput(body, process.env.GITHUB_OUTPUT);
  }
}
