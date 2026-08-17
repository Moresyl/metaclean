import assert from "node:assert/strict";
import { createWriteStream } from "node:fs";
import { lstat, mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import archiver from "archiver";
import extract from "extract-zip";

const root = await mkdtemp(join(tmpdir(), "metaclean-extract-zip-"));

try {
  const archivePath = join(root, "symlink.zip");
  await new Promise((resolve, reject) => {
    const output = createWriteStream(archivePath);
    const archive = archiver("zip");
    output.on("close", resolve);
    output.on("error", reject);
    archive.on("error", reject);
    archive.pipe(output);
    archive.symlink("escape-link", "../escape-target");
    void archive.finalize();
  });

  const destination = join(root, "destination");
  await mkdir(destination);
  await assert.rejects(
    extract(archivePath, { dir: destination }),
    /Out of bound symlink target/,
  );
  const linkExists = await lstat(join(destination, "escape-link")).then(
    () => true,
    () => false,
  );
  assert.equal(linkExists, false, "unsafe archive symlink must not be created");
} finally {
  await rm(root, { recursive: true, force: true });
}
