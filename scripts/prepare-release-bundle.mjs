import { lstat, rm } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

export async function prepareReleaseBundle(bundleDirectory, workspace = process.cwd()) {
  // Only generated release bundles may be removed; retain compiled dependencies.
  const relative = bundleDirectory.replaceAll("\\", "/");
  if (!/^src-tauri\/target\/(?:[a-z0-9_]+(?:-[a-z0-9]+)+\/)?release\/bundle$/u.test(relative)) {
    throw new Error("Expected a generated src-tauri/target release bundle directory");
  }
  let current = path.resolve(workspace);
  for (const segment of relative.split("/")) {
    current = path.join(current, segment);
    try {
      if ((await lstat(current)).isSymbolicLink()) throw new Error("Release bundle path must not contain symbolic links");
    } catch (error) {
      if (error?.code === "ENOENT") break;
      throw error;
    }
  }
  await rm(path.resolve(workspace, relative), { recursive: true, force: true });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory] = process.argv.slice(2);
  if (!bundleDirectory) throw new Error("Usage: prepare-release-bundle.mjs <bundle-directory>");
  await prepareReleaseBundle(bundleDirectory);
  console.log("Removed previous release bundles; compilation cache retained.");
}
