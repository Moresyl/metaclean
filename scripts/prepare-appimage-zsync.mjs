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

export async function placeAppImageZsync(bundleDirectory, generatedDirectory = process.cwd()) {
  const files = await walk(bundleDirectory);
  const appImages = files.filter((file) => file.toLowerCase().endsWith(".appimage"));
  if (appImages.length !== 1) throw new Error(`Expected exactly one AppImage, found ${appImages.length}`);

  const appImagePath = appImages[0];
  const zsyncName = `${path.basename(appImagePath)}.zsync`;
  const generatedPath = path.join(generatedDirectory, zsyncName);
  const targetPath = `${appImagePath}.zsync`;
  if (generatedPath === targetPath) return targetPath;

  let generatedStat;
  try {
    generatedStat = await stat(generatedPath);
  } catch (error) {
    if (error?.code === "ENOENT") throw new Error(`Missing generated AppImage delta metadata: ${zsyncName}`);
    throw error;
  }
  if (!generatedStat.isFile() || generatedStat.size === 0) {
    throw new Error(`Generated AppImage delta metadata is empty or not a file: ${zsyncName}`);
  }

  await mkdir(path.dirname(targetPath), { recursive: true });
  await rename(generatedPath, targetPath);
  return targetPath;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory, generatedDirectory] = process.argv.slice(2);
  if (!bundleDirectory) {
    throw new Error("Usage: prepare-appimage-zsync.mjs <bundle-directory> [generated-directory]");
  }
  const targetPath = await placeAppImageZsync(bundleDirectory, generatedDirectory ?? process.cwd());
  console.log(`Placed AppImage delta metadata: ${targetPath}`);
}
