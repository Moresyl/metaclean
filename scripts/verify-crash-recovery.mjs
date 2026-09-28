import assert from "node:assert/strict";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

assert.equal(process.env.RUNNER_ENVIRONMENT, "github-hosted", "Requires an isolated hosted runner");
const [mode, fixtureDirectory, evidenceDirectory] = process.argv.slice(2);
assert.ok(["start", "complete", "recover", "cleared"].includes(mode));
const paths = JSON.parse(await readFile(path.join(evidenceDirectory, "sources.json"), "utf8"));
let target;
let lastTargets = [];
let lastConnectionError;
for (let attempt = 0; attempt < 120; attempt++) {
  try {
    const response = await fetch("http://127.0.0.1:9222/json/list", { signal: AbortSignal.timeout(1000) });
    lastTargets = await response.json();
    target = lastTargets.find(item => item.type === "page" && /tauri\.localhost|tauri:\/\//u.test(item.url));
    if (target) break;
  } catch (error) { lastConnectionError = String(error).slice(0, 1000); }
  await delay(250);
}
await writeFile(path.join(evidenceDirectory, `${mode}-connection.json`), JSON.stringify({ targets: lastTargets.map(({ type, url }) => ({ type, url })), error: lastConnectionError }, null, 2));
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
  if (mode === "start" || mode === "complete") {
    await evaluate("localStorage.setItem('metaclean.locale', 'en'); localStorage.setItem('metaclean.outputMode', 'copy'); true");
    await evaluate("location.reload(); true");
    await delay(1000);
    await until("Boolean(document.querySelector('.drop-zone'))");
    // Observe the native hover response before sending the one-shot drop.
    for (let attempt = 0; attempt < 240; attempt++) {
      await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {target: {kind: 'Webview', label: 'main'}, event: 'tauri://drag-enter', payload: {paths: [], position: {x: 400, y: 300}}})`);
      if (await evaluate("document.querySelector('.drop-zone')?.classList.contains('border-brand')")) break;
      assert.ok(attempt < 239, "Native drag listener did not become ready");
      await delay(250);
    }
    // Emit the same native event used by the real drag-and-drop intake path.
    await evaluate(`window.__TAURI_INTERNALS__.invoke('plugin:event|emit_to', {target: {kind: 'Webview', label: 'main'}, event: 'tauri://drag-drop', payload: {paths: ${JSON.stringify(paths)}, position: {x: 400, y: 300}}})`);
    await until("Boolean(document.querySelector('.scan-button:not(:disabled)'))");
    const scanStarted = performance.now();
    await evaluate("document.querySelector('.scan-button').click(); true");
    await until("document.querySelector('.scan-button')?.textContent.includes('Confirm') && !document.querySelector('.scan-button').disabled");
    const scanMs = performance.now() - scanStarted;
    const cleanStarted = performance.now();
    await evaluate("document.querySelector('.scan-button').click(); true");
    if (mode === "complete") {
      for (let attempt = 0; attempt < 600; attempt++) {
        const outputs = (await readdir(fixtureDirectory)).filter(name => name.endsWith(".cleaned.txt"));
        if (outputs.length === paths.length && await evaluate("localStorage.getItem('metaclean.activeBatch') === null")) break;
        assert.ok(attempt < 599, "Cleanup did not complete within the observation window");
        await delay(100);
      }
      const results = await evaluate(`JSON.parse(localStorage.getItem('metaclean.history') ?? '[]').flatMap(entry => entry.results).filter(result => ${JSON.stringify(paths)}.includes(result.sourcePath))`);
      assert.equal(results.length, paths.length);
      assert.ok(results.every(result => result.success && result.integrity));
      await writeFile(path.join(evidenceDirectory, "completed.json"), JSON.stringify({ scanMs, cleanMs: performance.now() - cleanStarted, results }, null, 2));
      console.log("Released desktop cleanup completed with audit fingerprints");
    } else {
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
