import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { prepareReleaseBundle } from "./prepare-release-bundle.mjs";
import { collectReleaseAssets } from "./collect-release-assets.mjs";

test("release workflow clears restored bundles before packaging", async () => {
  const workflow = await readFile(new URL("../.github/workflows/release.yml", import.meta.url), "utf8");
  const prepare = workflow.indexOf('node scripts/prepare-release-bundle.mjs "${{ matrix.bundlePath }}"');
  assert.ok(prepare > workflow.lastIndexOf("uses: actions/cache@v5"));
  assert.ok(prepare < workflow.indexOf("- name: Build release packages"));
});

test("cached old packages cannot contaminate a new native or cross-target release", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-bundle-"));
  try {
    for (const target of ["", "i686-pc-windows-msvc/", "aarch64-apple-darwin/"]) {
      const relative = `src-tauri/target/${target}release/bundle`;
      const bundle = path.join(root, relative);
      const dependencies = path.join(bundle, "..", "deps");
      await mkdir(dependencies, { recursive: true });
      await writeFile(path.join(dependencies, "compiled.rlib"), "keep");
      await mkdir(bundle, { recursive: true });
      await writeFile(path.join(bundle, "old.exe"), "old");
      await writeFile(path.join(bundle, "new.exe"), "new");
      await assert.rejects(collectReleaseAssets(bundle, path.join(root, "out"), ["exe"]), /found 2/u);
      await prepareReleaseBundle(relative, root);
      await prepareReleaseBundle(relative, root);
      assert.equal(await readFile(path.join(dependencies, "compiled.rlib"), "utf8"), "keep");
      await mkdir(bundle, { recursive: true });
      await writeFile(path.join(bundle, "new.exe"), "new");
      assert.deepEqual(await collectReleaseAssets(bundle, path.join(root, "out"), ["exe"]), ["new.exe"]);
    }
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("rejects paths outside generated release bundles and linked ancestors", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-bundle-safe-"));
  try {
    for (const invalid of [".", "src-tauri/target", "src-tauri/target/../release/bundle", "/tmp/release/bundle"]) {
      await assert.rejects(prepareReleaseBundle(invalid, root), /Expected a generated/u);
    }
    await mkdir(path.join(root, "external"));
    await writeFile(path.join(root, "external", "keep"), "keep");
    await mkdir(path.join(root, "src-tauri"));
    await symlink(path.join(root, "external"), path.join(root, "src-tauri", "target"), "junction");
    await assert.rejects(prepareReleaseBundle("src-tauri/target/release/bundle", root), /symbolic links/u);
    assert.equal(await readFile(path.join(root, "external", "keep"), "utf8"), "keep");
  } finally { await rm(root, { recursive: true, force: true }); }
});
