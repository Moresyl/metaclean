"""Verify JPEG pixels, display orientation and exact print density independently."""
import argparse
from fractions import Fraction
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

from PIL import Image, ImageCms, ImageOps, TiffImagePlugin, features
from PIL import __version__ as pillow_version

ROOT = Path(__file__).resolve().parent.parent
AUTHOR = "Synthetic private JPEG author"
SCENARIOS = {
    "inches": (2, (300, 1), (150, 1), 6, None),
    "centimeters": (3, (11811, 100), (5906, 100), 8, None),
    "aspect-ratio": (1, (2, 1), (1, 1), None, None),
    "independent-jfif": (2, (300, 1), (150, 1), 6, (72, 144)),
}


def make_fixture(path, scenario):
    unit, x, y, orientation, jfif = SCENARIOS[scenario]
    exif = Image.Exif()
    exif[282] = TiffImagePlugin.IFDRational(*x)
    exif[283] = TiffImagePlugin.IFDRational(*y)
    exif[296] = unit
    exif[315] = AUTHOR
    if orientation:
        exif[274] = orientation
    image = Image.new("RGB", (128, 96))
    image.putdata([(x * 2, y * 2, (x + y) % 256) for y in range(96) for x in range(128)])
    image.save(path, exif=exif, quality=91,
               icc_profile=ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes(),
               **({"dpi": jfif} if jfif else {}))
    seed_private_previews(path)


PREVIEW_SENTINEL = b"MetaClean-private-preview-identity"


def app0(payload):
    return b"\xff\xe0" + (len(payload) + 2).to_bytes(2, "big") + payload


def seed_private_previews(path):
    data = path.read_bytes()
    assert data[2:4] == b"\xff\xe0", "Fixture must begin with JFIF"
    end = 4 + int.from_bytes(data[4:6], "big")
    header = data[6:end]
    assert len(header) == 14 and header[:5] == b"JFIF\0"
    thumbnail = Image.new("RGB", (2, 1), (39, 72, 118))
    encoded = io.BytesIO()
    thumbnail.save(encoded, format="JPEG", comment=PREVIEW_SENTINEL)
    previews = app0(header[:12] + b"\x01\x01\x27\x48\x76" + PREVIEW_SENTINEL)
    previews += app0(b"JFXX\0\x10" + encoded.getvalue())
    previews += app0(b"JFXX\0\x11\x01\x01" + bytes(768) + b"\x00")
    previews += app0(b"JFXX\0\x13\x02\x01" + thumbnail.tobytes())
    previews += app0(b"private-editor\0" + PREVIEW_SENTINEL)
    path.write_bytes(data[:2] + previews + data[end:])


def compressed_scan(data):
    offset = 2
    while offset < len(data):
        assert data[offset] == 0xff, "Invalid fixture JPEG marker"
        if data[offset + 1] == 0xda:
            return data[offset:]
        offset += 2 + int.from_bytes(data[offset + 2:offset + 4], "big")
    raise AssertionError("Fixture lacks its compressed scan")


def inspect(path, preview):
    with Image.open(path) as image:
        image.load()
        exif = image.getexif()
        displayed = ImageOps.exif_transpose(image).convert("RGB")
        displayed.save(preview)
        return {
            "size": list(image.size),
            "pixels": hashlib.sha256(image.convert("RGB").tobytes()).hexdigest(),
            "displaySize": list(displayed.size),
            "displayPixels": hashlib.sha256(displayed.tobytes()).hexdigest(),
            "density": {str(key): [exif[key].numerator, exif[key].denominator] for key in (282, 283)},
            "unit": exif.get(296), "orientation": exif.get(274),
            "jfifUnit": image.info.get("jfif_unit"), "jfifDensity": image.info.get("jfif_density"),
            "profile": hashlib.sha256(image.info.get("icc_profile", b"")).hexdigest(),
            "profileBytes": len(image.info.get("icc_profile", b"")),
            "exifTags": sorted(exif.keys()), "author": exif.get(315),
        }


def verify_case(directory, scenario, mode, preserve_orientation):
    case = directory / f"{mode}-orientation-{preserve_orientation}"
    case.mkdir()
    source = case / "sample.jpg"
    shutil.copyfile(directory / "original.jpg", source)
    original = source.read_bytes()
    before = inspect(source, case / "before.png")
    unit, x, y, orientation, jfif = SCENARIOS[scenario]
    assert {key: Fraction(*value) for key, value in before["density"].items()} == {
        "282": Fraction(*x), "283": Fraction(*y),
    }
    assert before["unit"] == unit and before["orientation"] == orientation
    assert before["author"] == AUTHOR and AUTHOR.encode() in original
    assert before["profileBytes"] > 0, "Fixture lacks its color profile"
    if jfif:
        assert before["jfifUnit"] == 1 and tuple(before["jfifDensity"]) == jfif
    result = subprocess.run([
        "cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_jpeg_sample_with_verified_output", "--", "--ignored",
    ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=600,
        env={**os.environ, "METACLEAN_JPEG_SAMPLE_PATH": str(source),
             "METACLEAN_JPEG_OUTPUT_MODE": mode,
             "METACLEAN_JPEG_PRESERVE_ORIENTATION": str(preserve_orientation).lower()})
    (case / "native.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed; 0 failed" in result.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else case / "sample.cleaned.jpg"
    backup = case / "sample.jpg.bak" if mode == "replace" else source
    assert backup.read_bytes() == original, "Source or replacement backup changed"
    assert AUTHOR.encode() not in cleaned.read_bytes(), "Private author bytes remain"
    cleaned_bytes = cleaned.read_bytes()
    assert PREVIEW_SENTINEL not in cleaned_bytes and b"JFXX\0" not in cleaned_bytes
    assert compressed_scan(cleaned_bytes) == compressed_scan(original), "Compressed image bytes changed"
    after = inspect(cleaned, case / "after.png")
    for key in ("size", "pixels", "density", "unit", "jfifUnit", "jfifDensity", "profile", "profileBytes"):
        assert after[key] == before[key], f"{scenario}/{mode}: {key} changed"
    assert after["author"] is None and set(after["exifTags"]) <= {274, 282, 283, 296}
    assert after["orientation"] == (orientation if preserve_orientation else None)
    assert after["displayPixels"] == before["displayPixels" if preserve_orientation else "pixels"]
    assert after["displaySize"] == before["displaySize" if preserve_orientation else "size"]
    return {"scenario": scenario, "mode": mode, "preserveOrientation": preserve_orientation,
            "before": before, "after": after,
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(cleaned.read_bytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / "tools.json").write_text(json.dumps({
        "python": platform.python_version(), "pillow": pillow_version,
        "jpeg": features.version("jpg"),
    }, indent=2), encoding="utf-8")
    results = []
    try:
        for scenario in SCENARIOS:
            directory = output / scenario
            directory.mkdir()
            make_fixture(directory / "original.jpg", scenario)
            for mode in ("copy", "replace"):
                for preserve_orientation in (True, False):
                    results.append(verify_case(directory, scenario, mode, preserve_orientation))
                    print(f"Verified {scenario}/{mode}/orientation={preserve_orientation}")
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
