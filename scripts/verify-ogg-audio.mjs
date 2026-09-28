import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const ffmpeg = process.env.METACLEAN_FFMPEG || "ffmpeg";
const directory = await mkdtemp(path.join(tmpdir(), "metaclean-ogg-validation-"));

function run(binary, args, env = process.env) {
  const result = spawnSync(binary, args, {
    cwd: root, env, encoding: "utf8", timeout: 120_000, maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  assert.equal(result.status, 0, `${binary} failed: ${result.stderr}\n${result.stdout}`);
  return result;
}

try {
  run(ffmpeg, ["-version"]);
  for (const codec of ["opus", "vorbis"]) {
    const extension = codec === "opus" ? "opus" : "ogg";
    const gains = codec === "opus"
      ? ["R128_TRACK_GAIN=-573", "R128_ALBUM_GAIN=111"]
      : ["REPLAYGAIN_TRACK_GAIN=-5.25 dB", "REPLAYGAIN_TRACK_PEAK=0.5"];
    for (const variant of ["mono", "stereo", "surround", "long-tags", "chained"]) {
      const fixtureDirectory = path.join(directory, `${codec}-${variant}`);
      await mkdir(fixtureDirectory);
      const source = path.join(fixtureDirectory, `sample.${extension}`);
      const cleaned = path.join(fixtureDirectory, `sample.cleaned.${extension}`);
      const metadata = path.join(fixtureDirectory, "metadata.txt");
      await writeFile(metadata, [
        ";FFMETADATA1", "artist=Synthetic Artist", "title=Synthetic fixture", ...gains,
        ...(variant === "long-tags" ? [`comment=${"x".repeat(150_000)}`] : []), "",
      ].join("\n"));
      const channels = variant === "mono" ? "1" : variant === "surround" ? "6" : "2";
      const encode = (output) => run(ffmpeg, [
        "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=880:duration=2",
        "-f", "ffmetadata", "-i", metadata, "-map_metadata", "1", "-ac", channels,
        "-c:a", codec === "opus" ? "libopus" : "libvorbis", output,
      ]);
      encode(source);
      if (variant === "chained") {
        const second = path.join(fixtureDirectory, `second.${extension}`);
        encode(second);
        await writeFile(source, Buffer.concat([await readFile(source), await readFile(second)]));
      }
      const original = await readFile(source);
      const native = run("cargo", [
        "test", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_ogg_sample_without_changing_source", "--", "--ignored",
      ], { ...process.env, METACLEAN_AUDIO_SAMPLE_DIR: fixtureDirectory });
      assert.match(native.stdout, /1 passed; 0 failed/u, "External fixture test did not execute");
      assert.deepEqual(await readFile(source), original, "Source was modified");
      const output = await readFile(cleaned);
      assert.equal(output.length, original.length);
      assert.ok(!output.includes(Buffer.from("Synthetic Artist")), "Artist remains");
      assert.ok(!output.includes(Buffer.from("Synthetic fixture")), "Title remains");
      for (const gain of gains) assert.ok(output.includes(Buffer.from(gain)), `Missing gain: ${gain}`);
      const hashes = [];
      for (const file of [source, cleaned]) {
        // Arbitrary PCM bytes must never pass through UTF-8 decoding.
        const pcm = spawnSync(ffmpeg, [
          "-hide_banner", "-loglevel", "error", "-i", file, "-map", "0:a",
          "-af", "asetpts=N/SR/TB", "-f", "s16le", "pipe:1",
        ], { cwd: root, timeout: 120_000, maxBuffer: 8 * 1024 * 1024 });
        if (pcm.error) throw pcm.error;
        assert.equal(pcm.status, 0, pcm.stderr.toString());
        assert.equal(pcm.stderr.toString(), "", "Decoder emitted an error");
        assert.ok(pcm.stdout.length > 0, "Decoded audio is empty");
        hashes.push(createHash("sha256").update(pcm.stdout).digest("hex"));
      }
      assert.equal(hashes[0], hashes[1], `${codec}/${variant} PCM changed`);
      console.log(`${codec}/${variant}: identical PCM ${hashes[0]}`);
    }
  }
} finally {
  await rm(directory, { recursive: true, force: true });
}
