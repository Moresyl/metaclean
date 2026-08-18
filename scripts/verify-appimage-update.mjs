import { execFile } from "node:child_process";
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import { pathToFileURL } from "node:url";

const execFileAsync = promisify(execFile);
const UPDATE_INFORMATION_PATTERN = /^gh-releases-zsync\|[A-Za-z0-9_.-]+\|[A-Za-z0-9_.-]+\|latest\|[^|]+\.AppImage\.zsync$/u;

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory() && !entry.name.toLowerCase().endsWith(".appdir")) files.push(...await walk(fullPath));
    else if (entry.isFile()) files.push(fullPath);
  }
  return files;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

export function validateUpdateInformation(output, expectedUpdateInformation) {
  if (!UPDATE_INFORMATION_PATTERN.test(expectedUpdateInformation)) {
    throw new Error("Invalid AppImage GitHub Releases update information");
  }
  if (!output.includes(expectedUpdateInformation)) {
    throw new Error("AppImage .upd_info does not contain the expected GitHub Releases update information");
  }
}

export function validateZsyncContent(content, appImageName) {
  const escapedName = escapeRegex(appImageName);
  if (!new RegExp(`^Filename:\\s*${escapedName}\\s*$`, "mu").test(content)) {
    throw new Error(`zsync metadata does not target ${appImageName}`);
  }
  if (!new RegExp(`^URL:\\s*(?:.*/)?${escapedName}\\s*$`, "mu").test(content)) {
    throw new Error(`zsync metadata does not download ${appImageName}`);
  }
}

async function inspectUpdateSection(appImagePath) {
  const { stdout } = await execFileAsync("readelf", ["--string-dump=.upd_info", appImagePath], {
    encoding: "utf8",
    maxBuffer: 1024 * 1024
  });
  return stdout;
}

export async function verifyAppImageUpdate(bundleDirectory, expectedUpdateInformation, inspect = inspectUpdateSection) {
  const files = await walk(bundleDirectory);
  const appImages = files.filter((file) => file.toLowerCase().endsWith(".appimage"));
  if (appImages.length !== 1) throw new Error(`Expected exactly one AppImage, found ${appImages.length}`);

  const appImagePath = appImages[0];
  const zsyncPath = `${appImagePath}.zsync`;
  if (!files.includes(zsyncPath)) throw new Error(`Missing AppImage delta metadata: ${path.basename(zsyncPath)}`);
  if ((await stat(appImagePath)).size === 0 || (await stat(zsyncPath)).size === 0) {
    throw new Error("AppImage and zsync artifacts must be non-empty");
  }

  validateUpdateInformation(await inspect(appImagePath), expectedUpdateInformation);
  validateZsyncContent(await readFile(zsyncPath, "utf8"), path.basename(appImagePath));
  return [path.basename(appImagePath), path.basename(zsyncPath)];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory, expectedUpdateInformation] = process.argv.slice(2);
  if (!bundleDirectory || !expectedUpdateInformation) {
    throw new Error("Usage: verify-appimage-update.mjs <bundle-directory> <expected-update-information>");
  }
  const artifacts = await verifyAppImageUpdate(bundleDirectory, expectedUpdateInformation);
  console.log(`Verified AppImage delta updates: ${artifacts.join(", ")}`);
}
