import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

assert.equal(process.env.RUNNER_ENVIRONMENT, "github-hosted", "Requires an isolated hosted runner");
const mode = process.argv[2];
assert.ok(["seed", "verify", "update"].includes(mode));
const expected = {
  "metaclean.locale": "en",
  "metaclean.theme": "dark",
  "metaclean.outputMode": "copy",
  "metaclean.history": JSON.stringify([{
    id: "upgrade-synthetic-history", createdAt: "2026-09-28T00:00:00.000Z", mode: "copy",
    results: [{ sourcePath: "upgrade-synthetic.jpg", removed: [], success: true,
      integrity: { sourceSha256: "a".repeat(64), outputSha256: "b".repeat(64) } }],
  }]),
};
let target;
let diagnostic;
for (let attempt = 0; attempt < 30; attempt++) {
  try {
    const response = await fetch("http://127.0.0.1:9222/json/list", { signal: AbortSignal.timeout(1000) });
    const targets = await response.json();
    diagnostic = targets.map(({ type, url }) => ({ type, url }));
    target = targets.find((item) => item.type === "page" && /tauri\.localhost|tauri:\/\//u.test(item.url));
    if (target) break;
  } catch (error) { diagnostic = error.message; }
  await delay(500);
}
assert.ok(target?.webSocketDebuggerUrl, `Released application did not expose its isolated WebView: ${JSON.stringify(diagnostic)}`);
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let nextId = 0;
function evaluate(expression, phase = "evaluation") {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timeout = setTimeout(() => { socket.removeEventListener("message", listener); reject(new Error("CDP evaluation timed out")); }, 10000);
    function listener(event) {
      const message = JSON.parse(event.data);
      if (message.id !== id) return;
      clearTimeout(timeout);
      socket.removeEventListener("message", listener);
      if (message.error || message.result?.exceptionDetails) {
        const detail = message.error?.message ?? message.result.exceptionDetails.exception?.description ?? message.result.exceptionDetails.text;
        reject(new Error(`WebView ${phase} failed: ${detail}`));
      }
      else resolve(message.result.result.value);
    }
    socket.addEventListener("message", listener);
    socket.send(JSON.stringify({ id, method: "Runtime.evaluate", params: { expression, returnByValue: true } }));
  });
}
try {
  if (mode === "seed") {
    await evaluate(`(() => { for (const [key,value] of Object.entries(${JSON.stringify(expected)})) localStorage.setItem(key,value); return true; })()`, "seed storage");
    await evaluate("location.reload(); true", "reload after seed");
    await delay(2000);
  }
  const snapshot = await evaluate(`(() => ({ values: Object.fromEntries(${JSON.stringify(Object.keys(expected))}.map(key => [key, localStorage.getItem(key)])), theme: document.documentElement.dataset.theme, title: document.title }))()`, "read persisted state");
  assert.deepEqual(snapshot.values, expected, "Persisted preferences/history changed across installation");
  assert.equal(snapshot.theme, "dark", "Application did not apply the persisted theme");
  assert.equal(snapshot.title, "MetaClean");
  console.log(`WebView ${mode}: preferences and synthetic history/fingerprints preserved; dark theme applied`);
  if (mode === "update") {
    const version = process.argv[3];
    assert.match(version, /^\d+\.\d+\.\d+$/u);
    let ready = false;
    for (let attempt = 0; attempt < 60; attempt++) {
      ready = await evaluate(`(() => { const title = document.getElementById('update-dialog-title'); return title?.textContent === ${JSON.stringify(`v${version}`)} && [...document.querySelectorAll('[role="dialog"] button')].some(button => button.textContent.trim() === 'Install update' && !button.disabled); })()`);
      if (ready) break;
      await delay(500);
    }
    assert.ok(ready, "Expected signed update was not offered by the real application");
    assert.equal(await evaluate("(() => { const button = [...document.querySelectorAll('[role=\"dialog\"] button')].find(button => button.textContent.trim() === 'Install update' && !button.disabled); if (!button) return false; button.click(); return true; })()"), true);
    console.log(`Clicked the released application's Install update button for ${version}`);
  }
} finally { socket.close(); }
