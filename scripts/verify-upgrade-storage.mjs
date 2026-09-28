import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";

assert.equal(process.env.RUNNER_ENVIRONMENT, "github-hosted", "Requires an isolated hosted runner");
const mode = process.argv[2];
assert.ok(["seed", "verify"].includes(mode));
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
function evaluate(expression) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timeout = setTimeout(() => { socket.removeEventListener("message", listener); reject(new Error("CDP evaluation timed out")); }, 10000);
    function listener(event) {
      const message = JSON.parse(event.data);
      if (message.id !== id) return;
      clearTimeout(timeout);
      socket.removeEventListener("message", listener);
      if (message.error || message.result?.exceptionDetails) reject(new Error("WebView evaluation failed"));
      else resolve(message.result.result.value);
    }
    socket.addEventListener("message", listener);
    socket.send(JSON.stringify({ id, method: "Runtime.evaluate", params: { expression, returnByValue: true } }));
  });
}
try {
  if (mode === "seed") {
    await evaluate(`(() => { for (const [key,value] of Object.entries(${JSON.stringify(expected)})) localStorage.setItem(key,value); return true; })()`);
    await evaluate("location.reload(); true");
    await delay(2000);
  }
  const snapshot = await evaluate(`(() => ({ values: Object.fromEntries(${JSON.stringify(Object.keys(expected))}.map(key => [key, localStorage.getItem(key)])), theme: document.documentElement.dataset.theme, title: document.title }))()`);
  assert.deepEqual(snapshot.values, expected, "Persisted preferences/history changed across installation");
  assert.equal(snapshot.theme, "dark", "Application did not apply the persisted theme");
  assert.equal(snapshot.title, "MetaClean");
  console.log(`WebView ${mode}: preferences and synthetic history/fingerprints preserved; dark theme applied`);
} finally { socket.close(); }
