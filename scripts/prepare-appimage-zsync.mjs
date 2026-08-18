import { mkdir, readdir, rename, stat } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory() && !entry.name.toLowerCase().endsWith(".appdir")) files.push(...await walk(fullPath));
    else if (entry.isFile()) files.push(fullPath);
  }
  return files;
}

export async function placeAppImageZsync(
  bundleDirectory,
  generatedDirectories = [process.cwd(), path.join(process.cwd(), "src-tauri")]
) {
  const files = await walk(bundleDirectory);
  const appImages = files.filter((file) => file.toLowerCase().endsWith(".appimage"));
  if (appImages.length !== 1) throw new Error(`Expected exactly one AppImage, found ${appImages.length}`);

  const appImagePath = appImages[0];
  const zsyncName = `${path.basename(appImagePath)}.zsync`;
  const targetPath = `${appImagePath}.zsync`;
  try {
    const targetStat = await stat(targetPath);
    if (!targetStat.isFile() || targetStat.size === 0) {
      throw new Error(`Generated AppImage delta metadata is empty or not a file: ${zsyncName}`);
    }
    return targetPath;
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }

  const directories = Array.isArray(generatedDirectories) ? generatedDirectories : [generatedDirectories];
  const candidates = [];
  for (const directory of directories) {
    const candidatePath = path.join(directory, zsyncName);
    if (candidatePath === targetPath) continue;
    try {
      candidates.push({ path: candidatePath, details: await stat(candidatePath) });
    } catch (error) {
      if (error?.code !== "ENOENT") throw error;
    }
  }
  if (candidates.length === 0) throw new Error(`Missing generated AppImage delta metadata: ${zsyncName}`);
  if (candidates.length !== 1) throw new Error(`Expected exactly one generated ${zsyncName}, found ${candidates.length}`);
  if (!candidates[0].details.isFile() || candidates[0].details.size === 0) {
    throw new Error(`Generated AppImage delta metadata is empty or not a file: ${zsyncName}`);
  }

  await mkdir(path.dirname(targetPath), { recursive: true });
  await rename(candidates[0].path, targetPath);
  return targetPath;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory, ...generatedDirectories] = process.argv.slice(2);
  if (!bundleDirectory) {
    throw new Error("Usage: prepare-appimage-zsync.mjs <bundle-directory> [generated-directory]");
  }
  const targetPath = await placeAppImageZsync(
    bundleDirectory,
    generatedDirectories.length > 0 ? generatedDirectories : undefined
  );
  console.log(`Placed AppImage delta metadata: ${targetPath}`);
}
