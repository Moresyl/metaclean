import assert from "node:assert/strict";
import { createWriteStream } from "node:fs";
import { lstat, mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import archiver from "archiver";
import extract from "extract-zip";

const root = await mkdtemp(join(tmpdir(), "metaclean-extract-zip-"));

async function writeArchive(archivePath, populate) {
  await new Promise((resolve, reject) => {
    const output = createWriteStream(archivePath);
    const archive = archiver("zip");
    output.on("close", resolve);
    output.on("error", reject);
    archive.on("error", reject);
    archive.pipe(output);
    populate(archive);
    void archive.finalize();
  });
}

try {
  const archivePath = join(root, "symlink.zip");
  await writeArchive(archivePath, (archive) => {
    archive.symlink("escape-link", "../escape-target");
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

  const safeArchive = join(root, "safe.zip");
  await writeArchive(safeArchive, (archive) => {
    archive.append("safe content", { name: "nested/file.txt" });
  });
  const safeDestination = join(root, "safe-destination");
  await extract(safeArchive, { dir: safeDestination });
  assert.equal(
    await readFile(join(safeDestination, "nested", "file.txt"), "utf8"),
    "safe content",
  );
} finally {
  await rm(root, { recursive: true, force: true });
}
