import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { stableVersion } from "./stable-version.mjs";

/** A manually selected release source must be immutable and match its checkout. */
export function validateReleaseSource({ tag, version, sourceCommit = "", checkedOutCommit }) {
  if (typeof tag !== "string" || !tag.startsWith("v") || stableVersion(tag.slice(1)) !== tag.slice(1)) {
    throw new Error("Release tag must be a canonical stable version");
  }
  if (tag !== `v${version}`) throw new Error("Release tag does not match package version");
  if (typeof checkedOutCommit !== "string" || checkedOutCommit.length !== 40 || !/^[a-f0-9]{40}$/u.test(checkedOutCommit)) {
    throw new Error("Release checkout must resolve to a full commit SHA");
  }
  if (sourceCommit !== "") {
    if (typeof sourceCommit !== "string" || sourceCommit.length !== 40 || !/^[a-f0-9]{40}$/u.test(sourceCommit)) {
      throw new Error("Manual release source must be a full lowercase commit SHA");
    }
    if (sourceCommit !== checkedOutCommit) throw new Error("Release checkout differs from the selected commit");
  }
  return checkedOutCommit;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { version } = JSON.parse(await readFile("package.json", "utf8"));
  const checkedOutCommit = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  validateReleaseSource({ tag: process.env.RELEASE_TAG, version,
    sourceCommit: process.env.SOURCE_COMMIT ?? "", checkedOutCommit });
  console.log(`Verified release source ${checkedOutCommit}`);
}
