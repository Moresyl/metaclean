import assert from "node:assert/strict";
import test from "node:test";
import { validateReleaseSource } from "./verify-release-source.mjs";

const commit = "1234567890abcdef1234567890abcdef12345678";
const selection = { tag: "v0.12.0", version: "0.12.0", checkedOutCommit: commit };

test("accepts an existing tag checkout and an exact manual commit", () => {
  assert.equal(validateReleaseSource(selection), commit);
  assert.equal(validateReleaseSource({ ...selection, sourceCommit: commit }), commit);
});

test("rejects moving refs and malformed manual sources", () => {
  for (const sourceCommit of ["master", "v0.12.0", commit.slice(0, 7), commit.toUpperCase(), ` ${commit}`, `${commit}\n`, null, 123]) {
    assert.throws(() => validateReleaseSource({ ...selection, sourceCommit }), /full lowercase commit SHA/u);
  }
});

test("rejects mismatched source, version and checkout identities", () => {
  assert.throws(() => validateReleaseSource({ ...selection, sourceCommit: "a".repeat(40) }), /differs/u);
  assert.throws(() => validateReleaseSource({ ...selection, version: "0.11.10" }), /package version/u);
  for (const checkedOutCommit of ["", "HEAD", "a".repeat(39), null]) {
    assert.throws(() => validateReleaseSource({ ...selection, checkedOutCommit }), /full commit SHA/u);
  }
});

test("rejects ambiguous tags and unstable versions", () => {
  for (const tag of ["0.12.0", "v00.12.0", "v0.12.0-beta.1", "v0.12.0 ", "v0.12.0\n", undefined]) {
    assert.throws(() => validateReleaseSource({ ...selection, tag }), /canonical stable version/u);
  }
});
