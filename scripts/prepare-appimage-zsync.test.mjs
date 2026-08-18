import assert from "node:assert/strict";
import { access, mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { placeAppImageZsync } from "./prepare-appimage-zsync.mjs";

const appImageName = "MetaClean_1.2.3_amd64.AppImage";

test("moves appimagetool's working-directory zsync output beside the AppImage", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-place-zsync-"));
  try {
    const bundle = path.join(root, "bundle", "appimage");
    const generated = path.join(root, "generated");
    await mkdir(path.join(bundle, "MetaClean.AppDir"), { recursive: true });
    await mkdir(generated);
    await writeFile(path.join(bundle, appImageName), "appimage");
    await writeFile(path.join(bundle, "MetaClean.AppDir", "ignored.AppImage"), "internal");
    await writeFile(path.join(generated, `${appImageName}.zsync`), "zsync");

    const target = await placeAppImageZsync(path.join(root, "bundle"), generated);
    assert.equal(target, path.join(bundle, `${appImageName}.zsync`));
    await access(target);
    await assert.rejects(access(path.join(generated, `${appImageName}.zsync`)));
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("rejects missing, empty and ambiguous AppImage delta output", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-place-zsync-invalid-"));
  try {
    await assert.rejects(placeAppImageZsync(root, root), /found 0/u);
    await writeFile(path.join(root, appImageName), "appimage");
    await assert.rejects(placeAppImageZsync(root, path.join(root, "generated")), /Missing generated/u);
    await mkdir(path.join(root, "generated"));
    await writeFile(path.join(root, "generated", `${appImageName}.zsync`), "");
    await assert.rejects(placeAppImageZsync(root, path.join(root, "generated")), /empty or not a file/u);
    await writeFile(path.join(root, "Other.AppImage"), "other");
    await assert.rejects(placeAppImageZsync(root, path.join(root, "generated")), /found 2/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("accepts one exact match across Tauri's possible working directories and rejects duplicates", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "metaclean-place-zsync-candidates-"));
  try {
    const bundle = path.join(root, "bundle");
    const first = path.join(root, "repository");
    const second = path.join(first, "src-tauri");
    await mkdir(bundle);
    await mkdir(second, { recursive: true });
    await writeFile(path.join(bundle, appImageName), "appimage");
    await writeFile(path.join(second, `${appImageName}.zsync`), "zsync");
    assert.equal(await placeAppImageZsync(bundle, [first, second]), path.join(bundle, `${appImageName}.zsync`));

    await rm(path.join(bundle, `${appImageName}.zsync`));
    await writeFile(path.join(first, `${appImageName}.zsync`), "first");
    await writeFile(path.join(second, `${appImageName}.zsync`), "second");
    await assert.rejects(placeAppImageZsync(bundle, [first, second]), /exactly one generated/u);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
