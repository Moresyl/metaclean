import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { loadReleaseNotes, validateReleaseNotes } from "./load-release-notes.mjs";

test("loads the concrete bilingual notes for a published version", async () => {
  const notes = await loadReleaseNotes("v0.3.0");
  assert.match(notes, /### 新功能/u);
  assert.match(notes, /### English summary/u);
  assert.equal(validateReleaseNotes("v0.3.0", notes.replace(/\n/gu, "\r\n")), notes);
});

test("rejects missing sections, generic bodies and invalid tags", () => {
  assert.throws(() => validateReleaseNotes("latest", ""), /Invalid release tag/u);
  assert.throws(() => validateReleaseNotes("v1.0.0", "# MetaClean v1.0.0\n\n### 新功能\n\n- item"), /missing concrete bullets/u);
  const generic = "# MetaClean v1.0.0\n\n### 新功能\n- 仅见 changelog\n### 变更与安全\n- item\n### 修复与打磨\n- item\n### 安装\n- item\n### English summary\n- item";
  assert.throws(() => validateReleaseNotes("v1.0.0", generic), /cannot delegate/u);
});

test("release workflow consumes validated notes and finalizes checksums", async () => {
  const workflow = await readFile(new URL("../.github/workflows/release.yml", import.meta.url), "utf8");
  const ciWorkflow = await readFile(new URL("../.github/workflows/ci.yml", import.meta.url), "utf8");
  assert.doesNotMatch(workflow, /releaseBody:/u);
  assert.match(workflow, /^  finalize:/mu);
  assert.match(workflow, /generate-checksums\.mjs/u);
  assert.match(workflow, /smoke-windows-installer\.ps1/u);
  assert.match(workflow, /package-windows-portable\.ps1/u);
  assert.match(workflow, /smoke-windows-portable\.ps1/u);
  assert.match(workflow, /i686-pc-windows-msvc/u);
  assert.match(workflow, /args: --bundles "nsis,msi"/u);
  assert.match(workflow, /smoke-macos-dmg\.sh/u);
  assert.match(workflow, /smoke-linux-deb\.sh/u);
  assert.match(workflow, /gh release create/u);
  for (const setup of [workflow, ciWorkflow]) {
    assert.doesNotMatch(setup, /pnpm\/action-setup|dtolnay\/rust-toolchain|Swatinem\/rust-cache/u);
    assert.match(setup, /corepack prepare pnpm@10\.32\.1 --activate/u);
    assert.match(setup, /actions\/cache@v5/u);
  }
  assert.match(workflow, /rustup target add \$\{\{ matrix\.target \}\}/u);
  assert.doesNotMatch(workflow, /\$RUNNER_TEMP/u);
  assert.equal((workflow.match(/ref: \$\{\{ env\.RELEASE_TAG \}\}/gu) ?? []).length, 2);
});
