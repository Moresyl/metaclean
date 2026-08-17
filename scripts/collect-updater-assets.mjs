import { copyFile, mkdir, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const PLATFORM_SPECS = {
  "windows-x86_64": { packageSuffix: ".exe", signatureSuffix: ".exe.sig" },
  "windows-i686": { packageSuffix: ".exe", signatureSuffix: ".exe.sig" },
  "darwin-aarch64": { packageSuffix: ".app.tar.gz", signatureSuffix: ".app.tar.gz.sig", outputArch: "aarch64" },
  "darwin-x86_64": { packageSuffix: ".app.tar.gz", signatureSuffix: ".app.tar.gz.sig", outputArch: "x64" },
  "linux-x86_64": { packageSuffix: ".AppImage", signatureSuffix: ".AppImage.sig" }
};

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory() && !/\.(?:app|appdir)$/iu.test(entry.name)) files.push(...await walk(fullPath));
    else if (entry.isFile()) files.push(fullPath);
  }
  return files;
}

function exactSuffix(file, suffix) {
  return file.toLowerCase().endsWith(suffix.toLowerCase());
}

export async function collectUpdaterAssets(bundleDirectory, outputDirectory, platform, version) {
  const spec = PLATFORM_SPECS[platform];
  if (!spec) throw new Error(`Unsupported updater platform: ${platform}`);
  const normalizedVersion = version.replace(/^v/u, "");
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(normalizedVersion)) throw new Error(`Invalid updater version: ${version}`);

  const files = await walk(bundleDirectory);
  const signatures = files.filter((file) => exactSuffix(file, spec.signatureSuffix));
  if (signatures.length !== 1) {
    throw new Error(`Expected exactly one ${spec.signatureSuffix} updater signature, found ${signatures.length}`);
  }
  const signature = signatures[0];
  const updaterPackage = signature.slice(0, -4);
  if (!exactSuffix(updaterPackage, spec.packageSuffix)) throw new Error(`Updater package has an unexpected name: ${updaterPackage}`);
  if (!files.includes(updaterPackage)) throw new Error(`Updater package is missing for signature: ${signature}`);
  for (const file of [updaterPackage, signature]) {
    if ((await stat(file)).size === 0) throw new Error(`Updater asset is empty: ${file}`);
  }

  const packageName = spec.outputArch
    ? `MetaClean_${normalizedVersion}_${spec.outputArch}.app.tar.gz`
    : path.basename(updaterPackage);
  const signatureName = `${packageName}.sig`;
  await mkdir(outputDirectory, { recursive: true });
  await copyFile(updaterPackage, path.join(outputDirectory, packageName));
  await copyFile(signature, path.join(outputDirectory, signatureName));
  return [packageName, signatureName];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [bundleDirectory, outputDirectory, platform, version] = process.argv.slice(2);
  if (!bundleDirectory || !outputDirectory || !platform || !version) {
    throw new Error("Usage: collect-updater-assets.mjs <bundle-directory> <output-directory> <platform> <version>");
  }
  const names = await collectUpdaterAssets(bundleDirectory, outputDirectory, platform, version);
  console.log(`Collected updater assets for ${platform}: ${names.join(", ")}`);
}
