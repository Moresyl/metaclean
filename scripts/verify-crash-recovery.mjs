import assert from "node:assert/strict";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.equal(process.env.RUNNER_ENVIRONMENT, "github-hosted", "Requires an isolated hosted runner");
const [mode, fixtureDirectory, evidenceDirectory] = process.argv.slice(2);
assert.ok(["start", "recover", "cleared"].includes(mode));
const paths = JSON.parse(await readFile(path.join(evidenceDirectory, "sources.json"), "utf8"));
let target;
for (let attempt = 0; attempt < 60; attempt++) {
  try {
    const response = await fetch("http://127.0.0.1:9222/json/list", { signal: AbortSignal.timeout(1000) });
    target = (await response.json()).find(item => item.type === "page" && /tauri\.localhost|tauri:\/\//u.test(item.url));
    if (target) break;
  } catch { /* The released WebView may still be starting. */ }
  await delay(250);
}
assert.ok(target?.webSocketDebuggerUrl, "Released WebView debug endpoint unavailable");
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  socket.addEventListener("open", resolve, { once: true });
  socket.addEventListener("error", reject, { once: true });
});
let nextId = 0;
function evaluate(expression) {
  return new Promise((resolve, reject) => {
    const id = ++nextId;
    const timeout = setTimeout(() => { socket.removeEventListener("message", listener); reject(new Error("CDP evaluation timed out")); }, 15000);
    function listener(event) {
      const message = JSON.parse(event.data);
      if (message.id !== id) return;
      clearTimeout(timeout);
      socket.removeEventListener("message", listener);
      if (message.error || message.result?.exceptionDetails) reject(new Error("Released WebView evaluation failed"));
      else resolve(message.result.result.value);
    }
    socket.addEventListener("message", listener);
    socket.send(JSON.stringify({ id, method: "Runtime.evaluate", params: { expression, returnByValue: true, awaitPromise: true } }));
  });
}
async function until(expression) {
  for (let attempt = 0; attempt < 240; attempt++) {
    if (await evaluate(expression)) return;
    await delay(250);
  }
  throw new Error(`Released UI did not reach expected state: ${expression}`);
}
try {
  await until("Boolean(document.querySelector('.app-shell'))");
  if (mode === "start") {
    await evaluate("localStorage.setItem('metaclean.locale', 'en'); localStorage.setItem('metaclean.outputMode', 'copy'); true");
    await evaluate("location.reload(); true");
    await delay(1000);
    await until("Boolean(document.querySelector('.drop-zone'))");
    // Emit the same native event used by the real drag-and-drop intake path.
    await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {target: {kind: 'Webview', label: 'main'}, event: 'tauri://drag-drop', payload: {paths: ${JSON.stringify(paths)}, position: {x: 400, y: 300}}})`);
    await until("Boolean(document.querySelector('.scan-button:not(:disabled)'))");
    await evaluate("document.querySelector('.scan-button').click(); true");
    await until("document.querySelector('.scan-button')?.textContent.includes('Confirm') && !document.querySelector('.scan-button').disabled");
    await evaluate("document.querySelector('.scan-button').click(); true");
    for (let attempt = 0; attempt < 600; attempt++) {
      const marker = await evaluate("JSON.parse(localStorage.getItem('metaclean.activeBatch') ?? 'null')");
      const outputs = (await readdir(fixtureDirectory)).filter(name => name.endsWith(".cleaned.txt"));
      if (marker && outputs.length > 0) {
        assert.equal(marker.total, paths.length);
        assert.equal(marker.mode, "copy");
        assert.ok(marker.completed < marker.total && outputs.length < paths.length, "Batch finished before interruption");
        assert.deepEqual(Object.keys(marker).sort(), ["batchId", "completed", "mode", "startedAt", "total"]);
        await writeFile(path.join(evidenceDirectory, "before-crash.json"), JSON.stringify({ marker, observedOutputs: outputs.length }, null, 2));
        console.log("Active real cleanup observed with committed output; ready for forced termination");
        break;
      }
      assert.ok(attempt < 599, "Did not observe a partially completed cleanup");
      await delay(20);
    }
  } else {
    await delay(1000);
    const state = await evaluate(`({ marker: localStorage.getItem('metaclean.activeBatch'), locale: localStorage.getItem('metaclean.locale'), visibleText: document.body.innerText.slice(0, 4000), message: /The previous cleanup may have (?:stopped after|been interrupted)/.test(document.body.innerText), safeNotice: document.body.innerText.includes('file operations are not resumed automatically'), entries: document.querySelectorAll('.file-item').length, actionDisabled: document.querySelector('.scan-button')?.disabled })`);
    await writeFile(path.join(evidenceDirectory, `${mode}.json`), JSON.stringify(state, null, 2));
    assert.equal(state.marker, null, "Restart did not clear the recovery marker");
    assert.equal(state.entries, 0, "Restart unexpectedly resumed the queue");
    assert.equal(state.actionDisabled, true);
    assert.equal(state.message, mode === "recover", "Recovery notice did not match restart phase");
    if (mode === "recover") assert.equal(state.safeNotice, true);
    console.log(`Released recovery UI passed: ${mode}`);
  }
} finally { socket.close(); }
