import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const capability = JSON.parse(await readFile(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
const policy = config?.app?.security?.csp;

assert.equal(typeof policy, "string", "production CSP must be an explicit string");
const directives = new Map(policy.split(";").map((part) => {
  const [name, ...sources] = part.trim().split(/\s+/u);
  return [name, sources];
}));

for (const directive of ["default-src", "script-src", "style-src", "font-src", "img-src", "connect-src"]) {
  assert.ok(directives.has(directive), `CSP is missing ${directive}`);
}
assert.deepEqual(directives.get("default-src"), ["'self'"]);
assert.deepEqual(directives.get("script-src"), ["'self'"]);
assert.deepEqual(directives.get("style-src"), ["'self'"]);
assert.deepEqual(directives.get("font-src"), ["'self'"]);
assert.deepEqual(directives.get("connect-src"), ["ipc:", "http://ipc.localhost"]);
for (const source of ["'self'", "asset:", "http://asset.localhost", "data:"]) {
  assert.ok(directives.get("img-src").includes(source), `img-src is missing ${source}`);
}
assert.doesNotMatch(policy, /unsafe-eval|\*|https?:\/\/(?!ipc\.localhost|asset\.localhost)/u);

const openerPermissions = capability.permissions.filter((permission) =>
  typeof permission === "string" ? permission.startsWith("opener:") : permission.identifier?.startsWith("opener:"),
);
assert.deepEqual(openerPermissions, [
  "opener:allow-reveal-item-in-dir",
  {
    identifier: "opener:allow-open-url",
    allow: [
      { url: "https://github.com/Moresyl/metaclean" },
      { url: "https://github.com/Moresyl/metaclean/issues" },
      { url: "https://github.com/Moresyl/metaclean/releases/*" },
    ],
  },
]);

for (const permission of ["core:window:allow-close", "core:window:allow-minimize", "core:window:allow-start-dragging"]) {
  assert.ok(capability.permissions.includes(permission), `desktop capability is missing ${permission}`);
}

console.log("Verified explicit local-only production CSP and scoped opener permissions.");
