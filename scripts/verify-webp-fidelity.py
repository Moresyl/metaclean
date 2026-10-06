"""Verify WebP privacy removal, animation, alpha and compressed pixels independently."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess

from PIL import Image, ImageCms, ImageDraw, features
from PIL import __version__ as pillow_version

ROOT = Path(__file__).resolve().parent.parent
TIMESTAMP_NS = 1_600_000_000_000_000_000
SENTINEL = b"Synthetic private WebP application identity"
PRIVATE_KINDS = frozenset((b"EXIF", b"XMP ", b"prIv", b"JUMB", b"jumb", b"ZERO", b"C2PA"))
SCENARIOS = ("rgb-lossy", "rgba-lossy", "rgba-lossless", "animation-lossy",
             "animation-lossless", "animation-offset-lossless")


def encode_chunk(kind, payload):
    return kind + struct.pack("<I", len(payload)) + payload + bytes(len(payload) & 1)


def chunk_records(data):
    offset = 0
    result = []
    while offset < len(data):
        assert offset + 8 <= len(data), "Truncated WebP chunk header"
        size = struct.unpack_from("<I", data, offset + 4)[0]
        payload_end = offset + 8 + size
        end = payload_end + (size & 1)
        assert end <= len(data), "Truncated WebP chunk payload or padding"
        assert not size & 1 or data[payload_end] == 0, "Nonzero WebP padding"
        result.append((data[offset:offset + 4], data[offset + 8:payload_end], data[offset:end]))
        offset = end
    return result


def riff_records(data):
    assert len(data) >= 12 and data[:4] == b"RIFF" and data[8:12] == b"WEBP", \
        "Invalid WebP signature"
    assert struct.unpack_from("<I", data, 4)[0] + 8 == len(data), "Incorrect WebP RIFF size"
    return chunk_records(data[12:])


def make_fixture(path, scenario):
    image = Image.new("RGBA", (32, 24))
    image.putdata([(x * 7, y * 9, (x + y) * 4, (x * 5 + y * 3) % 256)
                   for y in range(24) for x in range(32)])
    if scenario == "rgb-lossy":
        image = image.convert("RGB")
    elif scenario == "animation-offset-lossless":
        image = Image.new("RGBA", (32, 24), (17, 31, 47, 255))
        ImageDraw.Draw(image).rectangle((2, 2, 10, 10), fill=(255, 127, 31, 255))
    exif = Image.Exif()
    exif[315] = SENTINEL.decode()
    options = {"lossless": "lossless" in scenario, "quality": 87, "method": 4,
               "icc_profile": ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes(),
               "exif": exif.tobytes(), "xmp": b"<xmpmeta>" + SENTINEL + b"</xmpmeta>"}
    if scenario.startswith("animation"):
        options.update(save_all=True, append_images=[image.transpose(Image.Transpose.FLIP_LEFT_RIGHT)],
                       duration=[75, 125], loop=3, background=(17, 31, 47, 128))
        if scenario == "animation-offset-lossless":
            options.update(minimize_size=False, kmin=9, kmax=17)
    image.save(path, **options)
    parts = []
    for kind, payload, raw in riff_records(path.read_bytes()):
        if kind == b"ANMF":
            assert len(payload) >= 16, "Missing WebP frame header"
            chunk_records(payload[16:])
            payload += b"".join(encode_chunk(child, SENTINEL + b" frame")
                                for child in (b"prIv", b"JUMB", b"C2PA"))
            raw = encode_chunk(kind, payload)
        parts.append(raw)
    parts.extend(encode_chunk(kind, b"" if kind == b"ZERO" else SENTINEL + b" top")
                 for kind in (b"prIv", b"JUMB", b"jumb", b"ZERO", b"C2PA"))
    body = b"WEBP" + b"".join(parts)
    path.write_bytes(b"RIFF" + struct.pack("<I", len(body)) + body)


def inspect(path):
    with Image.open(path) as image:
        profile = image.info.get("icc_profile", b"")
        result = {"size": list(image.size), "loop": image.info.get("loop"),
                  "background": image.info.get("background"),
                  "profileSha256": hashlib.sha256(profile).hexdigest(),
                  "profileBytes": len(profile), "frames": []}
        for index in range(image.n_frames):
            image.seek(index)
            image.load()
            rgba = image.convert("RGBA")
            result["frames"].append({
                "mode": image.mode, "rawPixels": hashlib.sha256(image.tobytes()).hexdigest(),
                "rgbaPixels": hashlib.sha256(rgba.tobytes()).hexdigest(),
                "alphaRange": list(rgba.getchannel("A").getextrema()),
                "duration": image.info.get("duration"),
            })
        return result


def retained_records(records, preserve_profile):
    """Compare defined media bytes, allowing only metadata flags and enclosing sizes to change."""
    retained = []
    for kind, payload, raw in records:
        if kind in PRIVATE_KINDS or kind == b"ICCP" and not preserve_profile:
            continue
        if kind == b"VP8X":
            assert len(payload) == 10, "Invalid extended WebP header"
            flags = payload[0] & ~(0x0c | (0 if preserve_profile else 0x20))
            raw = encode_chunk(kind, bytes((flags,)) + payload[1:])
        elif kind == b"ANMF":
            assert len(payload) >= 16, "Invalid animation frame header"
            children = chunk_records(payload[16:])
            kept = [child_raw for child_kind, _, child_raw in children
                    if child_kind not in PRIVATE_KINDS]
            assert kept, "Missing encoded animation frame"
            raw = encode_chunk(kind, payload[:16] + b"".join(kept))
        retained.append((kind, raw))
    return retained


def assert_no_private_chunks(records):
    for kind, payload, _ in records:
        assert kind not in PRIVATE_KINDS, "Private WebP chunk remains"
        if kind == b"ANMF":
            assert_no_private_chunks(chunk_records(payload[16:]))


def check_output(original, output, preserve_profile):
    after_records = riff_records(output)
    assert_no_private_chunks(after_records)
    assert SENTINEL not in output, "Seeded private identity remains"
    assert [(kind, raw) for kind, _, raw in after_records] \
        == retained_records(riff_records(original), preserve_profile), \
        "Compressed media, frame geometry, duration, blending or animation control changed"


def verify_case(directory, scenario, mode, preserve_profile):
    case = directory / f"{mode}-icc-{'keep' if preserve_profile else 'remove'}"
    case.mkdir()
    source = case / "sample.webp"
    shutil.copyfile(directory / "original.webp", source)
    os.utime(source, ns=(TIMESTAMP_NS, TIMESTAMP_NS))
    original = source.read_bytes()
    before_records = riff_records(original)
    assert {kind for kind, _, _ in before_records if kind in PRIVATE_KINDS} == PRIVATE_KINDS
    frames = [payload for kind, payload, _ in before_records if kind == b"ANMF"]
    for frame in frames:
        assert {kind for kind, _, _ in chunk_records(frame[16:]) if kind in PRIVATE_KINDS} \
            == {b"prIv", b"JUMB", b"C2PA"}
    before = inspect(source)
    assert before["profileBytes"] > 0, "Source must contain an actual ICC profile"
    assert len(before["frames"]) == (2 if scenario.startswith("animation") else 1)
    assert len(frames) == (2 if scenario.startswith("animation") else 0)
    if scenario not in ("rgb-lossy", "animation-offset-lossless"):
        assert before["frames"][0]["alphaRange"][0] < 255, "Fixture must have actual alpha"
    if scenario == "animation-offset-lossless":
        last = frames[-1]
        assert int.from_bytes(last[3:6], "little") * 2 > 0, "Fixture must have an actual frame offset"
        assert int.from_bytes(last[9:12], "little") + 1 < before["size"][1], \
            "Last frame must be smaller than the declared canvas"
    result = subprocess.run([
        "cargo", "test", "--locked", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_webp_sample_with_verified_output", "--", "--ignored",
    ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=600,
        env={**os.environ, "METACLEAN_WEBP_SAMPLE_PATH": str(source),
             "METACLEAN_WEBP_OUTPUT_MODE": mode,
             "METACLEAN_WEBP_PRESERVE_PROFILE": str(preserve_profile).lower()})
    (case / "native.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed; 0 failed" in result.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else case / "sample.cleaned.webp"
    backup = case / "sample.webp.bak" if mode == "replace" else source
    assert backup.read_bytes() == original, "Source or replacement backup changed"
    assert cleaned.stat().st_mtime_ns == TIMESTAMP_NS, "Filesystem modification time changed"
    output = cleaned.read_bytes()
    check_output(original, output, preserve_profile)
    after = inspect(cleaned)
    expected = {**before}
    if not preserve_profile:
        expected.update(profileSha256=hashlib.sha256(b"").hexdigest(), profileBytes=0)
    assert after == expected, "Decoded pixels, alpha, frame timing, canvas or profile policy changed"
    return {"scenario": scenario, "mode": mode, "preserveColorProfile": preserve_profile,
            "before": before, "after": after,
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(output).hexdigest(),
            "filesystemMtimePreserved": True, "sourceOrBackupUnchanged": True,
            "privateBlocksRemoved": len(PRIVATE_KINDS) + len(frames) * 3,
            "retainedMediaAndControlBytesUnchanged": True,
            "profilePolicyVerified": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    output = parser.parse_args().output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / "tools.json").write_text(json.dumps({
        "python": platform.python_version(), "pillow": pillow_version,
        "webp": features.version("webp"),
    }, indent=2), encoding="utf-8")
    results = []
    try:
        for scenario in SCENARIOS:
            directory = output / scenario
            directory.mkdir()
            make_fixture(directory / "original.webp", scenario)
            for mode in ("copy", "replace"):
                for preserve_profile in (True, False):
                    results.append(verify_case(directory, scenario, mode, preserve_profile))
                    print(f"Verified {scenario}/{mode}/icc-{preserve_profile}: "
                          "private blocks removed, decoded and compressed media unchanged", flush=True)
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
