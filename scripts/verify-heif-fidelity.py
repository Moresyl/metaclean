"""Qualify native HEIF/AVIF cleanup against independent image decoders."""
import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import shutil
import subprocess

from PIL import Image, ImageCms, ImageOps, features
from PIL import __version__ as pillow_version
import pillow_heif

ROOT = Path(__file__).resolve().parent.parent
AUTHOR = "Synthetic private image author"
XMP = b'<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" dc:creator="Synthetic private image author"/></rdf:RDF></x:xmpmeta>'


def make_fixture(path, scenario):
    profile = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
    exif = Image.Exif()
    exif[315] = AUTHOR
    exif[270] = "Synthetic private description"
    exif[274] = 6
    frames = []
    for index in range(2 if scenario == "heic-sequence" else 1):
        picture = Image.new("RGBA" if scenario == "avif-alpha" else "RGB", (128, 96))
        pixels = [(x * 2, y * 2, (x + y + index * 79) % 256,
                   (x * 2 if scenario == "avif-alpha" else 255))
                  for y in range(96) for x in range(128)]
        picture.putdata(pixels if picture.mode == "RGBA" else [pixel[:3] for pixel in pixels])
        picture.info.update(exif=exif.tobytes(), xmp=XMP, icc_profile=profile)
        frames.append(picture)
    frames[0].save(path, format="AVIF" if scenario == "avif-alpha" else "HEIF",
                   quality=90, save_all=True, append_images=frames[1:],
                   exif=exif.tobytes(), xmp=XMP, icc_profile=profile)


def inspect(path, previews):
    previews.mkdir()
    frames = []
    with Image.open(path) as picture:
        for index in range(picture.n_frames):
            picture.seek(index)
            picture.load()
            pixels = picture.convert("RGBA")
            displayed = ImageOps.exif_transpose(picture).convert("RGBA")
            displayed.save(previews / f"frame-{index + 1}.png")
            profile = picture.info.get("icc_profile") or b""
            xmp = picture.info.get("xmp") or b""
            frames.append({
                "size": list(picture.size),
                "pixels": hashlib.sha256(pixels.tobytes()).hexdigest(),
                "profile": hashlib.sha256(profile).hexdigest(),
                "profileBytes": len(profile),
                "displaySize": list(displayed.size),
                "displayPixels": hashlib.sha256(displayed.tobytes()).hexdigest(),
                "alphaRange": list(pixels.getchannel("A").getextrema()),
                "exifTags": sorted(picture.getexif().keys()),
                "author": picture.getexif().get(315),
                "description": picture.getexif().get(270),
                "xmp": bool(xmp), "xmpAuthor": AUTHOR.encode() in xmp,
            })
    return frames


def verify_case(directory, scenario, mode):
    fixture = directory / ("original.avif" if scenario == "avif-alpha" else "original.heic")
    case = directory / mode
    case.mkdir()
    source = case / f"sample{fixture.suffix}"
    shutil.copyfile(fixture, source)
    original = fixture.read_bytes()
    before = inspect(source, case / "before")
    assert len(before) == (2 if scenario == "heic-sequence" else 1)
    assert all(frame["author"] == AUTHOR and frame["xmpAuthor"] and frame["description"]
               and frame["profileBytes"] for frame in before)
    if scenario == "avif-alpha":
        assert before[0]["alphaRange"][0] < before[0]["alphaRange"][1], "Fixture has no varying alpha"
        assert before[0]["displaySize"] == [96, 128], "Fixture is not rotated"
    if scenario == "heic-sequence":
        assert before[0]["pixels"] != before[1]["pixels"], "Fixture frames are not distinct"
    result = subprocess.run([
        "cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_heif_sample_with_verified_output", "--", "--ignored",
    ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace",
        timeout=600, env={**os.environ, "METACLEAN_HEIF_SAMPLE_PATH": str(source),
                          "METACLEAN_HEIF_OUTPUT_MODE": mode})
    (case / "native.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed; 0 failed" in result.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else source.with_name(f"sample.cleaned{source.suffix}")
    backup = source.with_name(f"{source.name}.bak") if mode == "replace" else source
    assert backup.read_bytes() == original, "Source or replacement backup changed"
    cleaned_bytes = cleaned.read_bytes()
    for marker in (AUTHOR.encode(), b"Synthetic private description"):
        assert marker in original, "Fixture lacks the expected private byte marker"
        assert marker not in cleaned_bytes, "Private bytes remain outside decoded metadata"
    after = inspect(cleaned, case / "after")
    assert len(after) == len(before), "Frame count changed"
    for expected, actual in zip(before, after):
        assert actual["author"] is None and not actual["xmp"], "Private metadata remains"
        assert set(actual["exifTags"]) <= {274}, "Non-orientation EXIF tags remain"
        for key in ("size", "pixels", "profile", "profileBytes", "displaySize", "displayPixels", "alphaRange"):
            assert actual[key] == expected[key], f"{scenario}/{mode}: {key} changed"
    return {"scenario": scenario, "mode": mode, "frames": after,
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(cleaned_bytes).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    assert features.check("avif"), "Pillow requires independent AVIF codec support"
    pillow_heif.register_heif_opener()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    (output / "tools.json").write_text(json.dumps({
        "python": platform.python_version(), "pillow": pillow_version,
        "pillowHeif": pillow_heif.__version__, "heif": pillow_heif.libheif_info(),
        "avif": features.version("avif"),
    }, indent=2), encoding="utf-8")
    results = []
    try:
        for scenario in ("heic-rgb", "heic-sequence", "avif-alpha"):
            directory = output / scenario
            directory.mkdir()
            make_fixture(directory / ("original.avif" if scenario == "avif-alpha" else "original.heic"), scenario)
            for mode in ("copy", "replace"):
                results.append(verify_case(directory, scenario, mode))
                print(f"Verified {scenario}/{mode}: pixels, profiles, frames and source/backup unchanged")
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
