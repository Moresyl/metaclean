"""Verify PNG timestamp removal, decoded pixels and untouched image chunks independently."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import zlib

from PIL import Image, ImageCms
from PIL import __version__ as pillow_version

ROOT = Path(__file__).resolve().parent.parent
SIGNATURE = b"\x89PNG\r\n\x1a\n"
TIMESTAMP_NS = 1_600_000_000_000_000_000


def chunks(data):
    assert data[:8] == SIGNATURE, "Invalid PNG signature"
    offset = 8
    result = []
    while offset < len(data):
        assert offset + 12 <= len(data), "Truncated chunk header"
        length = struct.unpack_from(">I", data, offset)[0]
        end = offset + length + 12
        assert end <= len(data), "Truncated chunk data"
        kind = data[offset + 4:offset + 8]
        assert zlib.crc32(data[offset + 4:end - 4]) == struct.unpack_from(">I", data, end - 4)[0]
        result.append((kind, data[offset:end]))
        offset = end
    assert result[-1][0] == b"IEND", "Missing final IEND"
    return result


def make_fixture(path, scenario):
    image = Image.new("RGBA", (32, 24))
    image.putdata([(x * 7, y * 9, (x + y) * 4, (x * 5 + y * 3) % 256)
                   for y in range(24) for x in range(32)])
    options = {"dpi": (300, 150)}
    if scenario == "gray16":
        image = Image.new("I;16", (32, 24))
        image.putdata([(x * 1001 + y * 997) % 65536 for y in range(24) for x in range(32)])
    else:
        options["icc_profile"] = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
    if scenario == "palette":
        image = image.quantize(colors=32)
    if scenario == "animation":
        options.update(save_all=True, append_images=[image.transpose(Image.Transpose.FLIP_LEFT_RIGHT)],
                       duration=[75, 125], loop=3, disposal=[0, 0], blend=[0, 0])
    image.save(path, **options)
    time = b"tIME" + struct.pack(">H5B", 2026, 10, 1, 12, 34, 56)
    time_chunk = struct.pack(">I", 7) + time + struct.pack(">I", zlib.crc32(time))
    parts = chunks(path.read_bytes())
    index = len(parts) - 1 if scenario in ("gray16", "animation") else 1
    parts.insert(index, (b"tIME", time_chunk))
    path.write_bytes(SIGNATURE + b"".join(raw for _, raw in parts))


def inspect(path):
    with Image.open(path) as image:
        result = {"dpi": image.info.get("dpi"), "loop": image.info.get("loop"),
                  "profileSha256": hashlib.sha256(image.info.get("icc_profile", b"")).hexdigest(),
                  "frames": []}
        for index in range(image.n_frames):
            image.seek(index)
            image.load()
            result["frames"].append({
                "mode": image.mode, "size": list(image.size),
                "rawPixels": hashlib.sha256(image.tobytes()).hexdigest(),
                "rgbaPixels": hashlib.sha256(image.convert("RGBA").tobytes()).hexdigest(),
                "duration": image.info.get("duration"),
                "disposal": getattr(image, "disposal_op", None),
                "blend": getattr(image, "blend_op", None),
            })
        return result


def verify_case(directory, scenario, mode):
    case = directory / mode
    case.mkdir()
    source = case / "sample.png"
    shutil.copyfile(directory / "original.png", source)
    os.utime(source, ns=(TIMESTAMP_NS, TIMESTAMP_NS))
    original = source.read_bytes()
    before_chunks = chunks(original)
    assert sum(kind == b"tIME" for kind, _ in before_chunks) == 1
    before = inspect(source)
    assert len(before["frames"]) == (2 if scenario == "animation" else 1)
    result = subprocess.run([
        "cargo", "test", "--locked", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_png_sample_with_verified_output", "--", "--ignored",
    ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=600,
        env={**os.environ, "METACLEAN_PNG_SAMPLE_PATH": str(source),
             "METACLEAN_PNG_OUTPUT_MODE": mode})
    (case / "native.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed; 0 failed" in result.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else case / "sample.cleaned.png"
    backup = case / "sample.png.bak" if mode == "replace" else source
    assert backup.read_bytes() == original, "Source or replacement backup changed"
    assert cleaned.stat().st_mtime_ns == TIMESTAMP_NS, "Filesystem modification time changed"
    after_chunks = chunks(cleaned.read_bytes())
    assert after_chunks == [(kind, raw) for kind, raw in before_chunks if kind != b"tIME"], \
        "Chunks other than the private timestamp changed"
    after = inspect(cleaned)
    assert before == after, "Pixels, animation, resolution or color profile changed"
    return {"scenario": scenario, "mode": mode, "before": before, "after": after,
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(cleaned.read_bytes()).hexdigest(),
            "filesystemMtimePreserved": True, "onlyTimeChunkRemoved": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / "tools.json").write_text(json.dumps({
        "python": platform.python_version(), "pillow": pillow_version,
    }, indent=2), encoding="utf-8")
    results = []
    try:
        for scenario in ("rgba", "palette", "gray16", "animation"):
            directory = output / scenario
            directory.mkdir()
            make_fixture(directory / "original.png", scenario)
            for mode in ("copy", "replace"):
                results.append(verify_case(directory, scenario, mode))
                print(f"Verified {scenario}/{mode}: timestamp removed, pixels and other chunks unchanged")
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
