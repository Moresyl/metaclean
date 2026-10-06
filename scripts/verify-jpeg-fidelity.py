"""Verify JPEG pixels, display metadata, private segments and indexed images."""
import argparse
from fractions import Fraction
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import xml.etree.ElementTree as ET

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
    cleaned = native_cleanup(source, mode, preserve_orientation)
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


def native_cleanup(source, mode, preserve_orientation=True):
    original = source.read_bytes()
    modified = source.stat().st_mtime_ns
    result = subprocess.run([
        "cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_jpeg_sample_with_verified_output", "--", "--ignored",
    ], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=600,
        env={**os.environ, "METACLEAN_JPEG_SAMPLE_PATH": str(source),
             "METACLEAN_JPEG_OUTPUT_MODE": mode,
             "METACLEAN_JPEG_PRESERVE_ORIENTATION": str(preserve_orientation).lower()})
    (source.parent / "native.log").write_text(result.stdout + result.stderr, encoding="utf-8")
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed; 0 failed" in result.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else source.with_name(f"{source.stem}.cleaned{source.suffix}")
    backup = source.with_name(f"{source.name}.bak") if mode == "replace" else source
    assert backup.read_bytes() == original, "Source or replacement backup changed"
    assert cleaned.stat().st_mtime_ns == modified, "Filesystem modification time changed"
    return cleaned


PRIVATE_APP = b"MetaClean-synthetic-private-app-payload"
XMP_HEADER = b"http://ns.adobe.com/xap/1.0/\0"
HDR_NS = "http://ns.adobe.com/hdr-gain-map/1.0/"
CONTAINER_NS = "http://ns.google.com/photos/1.0/container/"
ITEM_NS = "http://ns.google.com/photos/1.0/container/item/"
RDF_NS = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"


def app_segment(marker, payload):
    return bytes([0xff, marker]) + (len(payload) + 2).to_bytes(2, "big") + payload


def headers(data):
    offset = 2
    while data[offset:offset + 2] != b"\xff\xda":
        assert data[offset] == 0xff and data[offset + 1] not in (0xd8, 0xd9)
        end = offset + 2 + int.from_bytes(data[offset + 2:offset + 4], "big")
        assert offset + 4 <= end <= len(data)
        yield data[offset + 1], data[offset + 4:end], offset, end
        offset = end


def pixel_snapshot(data):
    with Image.open(io.BytesIO(data)) as image:
        image.load()
        return {"size": list(image.size), "mode": image.mode,
                "rawPixels": hashlib.sha256(image.tobytes()).hexdigest(),
                "rgbPixels": hashlib.sha256(image.convert("RGB").tobytes()).hexdigest(),
                "profile": hashlib.sha256(image.info.get("icc_profile", b"")).hexdigest()}


def image_frame(index=0, mode="RGB"):
    image = Image.new("RGB", (48, 32))
    image.putdata([((x * 5 + index * 73) % 256, (y * 7 + index * 31) % 256,
                    (x * 3 + y * 2 + index * 109) % 256)
                   for y in range(32) for x in range(48)])
    return image.convert(mode)


def encoded_image(mode="RGB", frames=1):
    images = [image_frame(index, mode) for index in range(frames)]
    exif = Image.Exif()
    exif[315] = AUTHOR
    options = {"exif": exif, "quality": 91}
    if mode == "RGB":
        options["icc_profile"] = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()
    encoded = io.BytesIO()
    if frames > 2:
        # Construct the index independently: cumulative file positions are not
        # interchangeable with individual frame sizes in a multi-image table.
        images_bytes = []
        for image in images:
            buffer = io.BytesIO()
            image.save(buffer, format="JPEG", **options)
            images_bytes.append(buffer.getvalue())

        def index_payload(entries):
            directory = TiffImagePlugin.ImageFileDirectory_v2()
            for tag, kind, value in [(0xb000, 7, b"0100"), (0xb001, 4, frames),
                                     (0xb002, 7, entries)]:
                directory.tagtype[tag] = kind
                directory[tag] = value
            return b"MPF\0II\x2a\0\x08\0\0\0" + directory.tobytes(8)

        placeholder = app_segment(0xe2, index_payload(bytes(frames * 16)))
        sizes = [len(image) for image in images_bytes]
        sizes[0] += len(placeholder)
        entries = bytearray()
        start = 0
        for index, size in enumerate(sizes):
            entries.extend(struct.pack("<IIIHH", 0x030000 if index == 0 else 0,
                                       size, start - 10 if index else 0, 0, 0))
            start += size
        segment = app_segment(0xe2, index_payload(bytes(entries)))
        assert len(segment) == len(placeholder)
        images_bytes[0] = images_bytes[0][:2] + segment + images_bytes[0][2:]
        data = b"".join(images_bytes)
        with Image.open(io.BytesIO(data)) as image:
            assert image.n_frames == frames
            for index in range(frames):
                image.seek(index)
                image.load()
        return data
    images[0].save(encoded, format="MPO" if frames > 1 else "JPEG",
                   **({"save_all": True, "append_images": images[1:]} if frames > 1 else {}),
                   **options)
    return encoded.getvalue()


def indexed_frames(data):
    _, payload, marker, _ = next(row for row in headers(data)
                                 if row[0] == 0xe2 and row[1].startswith(b"MPF\0"))
    tiff = payload[4:]
    endian = "<" if tiff[:2] == b"II" else ">"
    directory = struct.unpack_from(endian + "I", tiff, 4)[0]
    count = struct.unpack_from(endian + "H", tiff, directory)[0]
    fields = {}
    for index in range(count):
        tag, kind, size, value = struct.unpack_from(endian + "HHII", tiff, directory + 2 + index * 12)
        fields[tag] = (kind, size, value)
    total = fields[0xb001][2]
    assert fields[0xb002][1] == total * 16
    entries = []
    for index in range(total):
        attributes, size, offset, dep1, dep2 = struct.unpack_from(
            endian + "IIIHH", tiff, fields[0xb002][2] + index * 16)
        start = 0 if index == 0 else marker + 8 + offset
        assert start + size <= len(data)
        frame = data[start:start + size]
        assert frame.startswith(b"\xff\xd8") and frame.endswith(b"\xff\xd9")
        entries.append({"data": frame, "start": start, "size": size,
                        "attributes": attributes, "dependencies": [dep1, dep2]})
    return entries


def hdr_packet(attributes, children=""):
    xml = (f"<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='{PRIVATE_APP.decode()}'>"
           f"<r:RDF xmlns:r='{RDF_NS}'><r:Description xmlns:h='{HDR_NS}' "
           f"xmlns:c='{CONTAINER_NS}' xmlns:i='{ITEM_NS}' xmlns:d='http://purl.org/dc/elements/1.1/' "
           f"d:creator='{AUTHOR}' {attributes}>{children}</r:Description></r:RDF></x:xmpmeta>")
    return XMP_HEADER + xml.encode()


def make_hdr_container(array):
    frames = [entry["data"] for entry in indexed_frames(encoded_image(frames=2))]
    attributes = "h:Version='1.0' h:HDRCapacityMax='4.0' h:BaseRenditionIsHDR='False' h:Gamma='1.03'"
    children = ""
    if array:
        children = "<h:GainMapMax><r:Seq><r:li>3.25</r:li><r:li>3.0000000000000001</r:li><r:li>4e0</r:li></r:Seq></h:GainMapMax>"
    else:
        attributes += " h:GainMapMax='3.25'"
    frames[1] = frames[1][:2] + app_segment(0xe1, hdr_packet(attributes, children)) + frames[1][2:]
    directory = ("<c:Directory><r:Seq><r:li r:parseType='Resource'>"
                 "<c:Item i:Mime='image/jpeg' i:Semantic='Primary'/></r:li>"
                 "<r:li r:parseType='Resource'><c:Item i:Mime='image/jpeg' i:Semantic='GainMap' "
                 f"i:Length='{len(frames[1])}' i:Label='{PRIVATE_APP.decode()}'/></r:li></r:Seq></c:Directory>")
    primary_xmp = app_segment(0xe1, hdr_packet("h:Version='1.0'", directory))
    frames[0] = bytearray(frames[0][:2] + primary_xmp + frames[0][2:])
    _, payload, marker, _ = next(row for row in headers(frames[0])
                                 if row[0] == 0xe2 and row[1].startswith(b"MPF\0"))
    tiff = payload[4:]
    endian = "<" if tiff[:2] == b"II" else ">"
    directory = struct.unpack_from(endian + "I", tiff, 4)[0]
    for index in range(struct.unpack_from(endian + "H", tiff, directory)[0]):
        tag, _, _, value = struct.unpack_from(endian + "HHII", tiff, directory + 2 + index * 12)
        if tag == 0xb002:
            table = marker + 8 + value
            break
    struct.pack_into(endian + "II", frames[0], table + 4, len(frames[0]), 0)
    struct.pack_into(endian + "II", frames[0], table + 16 + 4, len(frames[1]), len(frames[0]) - marker - 8)
    return bytes(frames[0]) + frames[1]


def hdr_rendering(data):
    entries = indexed_frames(data)
    assert len(entries) == 2
    xmls = [ET.fromstring(next(payload[len(XMP_HEADER):] for marker, payload, _, _ in headers(entry["data"])
                              if marker == 0xe1 and payload.startswith(XMP_HEADER))) for entry in entries]
    items = xmls[0].findall(f".//{{{CONTAINER_NS}}}Item")
    assert len(items) == 2 and items[1].get(f"{{{ITEM_NS}}}Semantic") == "GainMap"
    assert int(items[1].get(f"{{{ITEM_NS}}}Length")) == entries[1]["size"]
    assert entries[0]["size"] == entries[1]["start"]
    description = xmls[1].find(f".//{{{RDF_NS}}}Description")
    fields = {key.removeprefix(f"{{{HDR_NS}}}"): [value] for key, value in description.attrib.items()
              if key.startswith(f"{{{HDR_NS}}}")}
    for child in description:
        if child.tag.startswith(f"{{{HDR_NS}}}"):
            fields[child.tag.removeprefix(f"{{{HDR_NS}}}")] = [node.text for node in child.findall(f".//{{{RDF_NS}}}li")]
    assert fields["Version"] == ["1.0"] and fields["BaseRenditionIsHDR"] == ["False"]
    with Image.open(io.BytesIO(entries[0]["data"])) as base, Image.open(io.BytesIO(entries[1]["data"])) as gain:
        base_pixels = base.convert("RGB").get_flattened_data()
        gain_pixels = gain.convert("RGB").get_flattened_data()
    rendered = bytearray()
    maximum = 0.0
    for base, gain in zip(base_pixels, gain_pixels, strict=True):
        for channel in range(3):
            def value(name, default):
                values = fields.get(name, [str(default)])
                return float(values[0 if len(values) == 1 else channel])
            encoded = base[channel] / 255.0
            linear = encoded / 12.92 if encoded <= 0.04045 else ((encoded + 0.055) / 1.055) ** 2.4
            weight = (gain[channel] / 255.0) ** (1.0 / value("Gamma", 1))
            boost = value("GainMapMin", 0) * (1 - weight) + value("GainMapMax", 0) * weight
            hdr = (linear + value("OffsetSDR", 1 / 64)) * 2 ** boost - value("OffsetHDR", 1 / 64)
            maximum = max(maximum, hdr)
            rendered.extend(struct.pack("<d", hdr))
    assert maximum > 1, "Fixture has no HDR brightness above the SDR range"
    return {"fields": fields, "linearHdrPixels": hashlib.sha256(rendered).hexdigest(),
            "linearHdrMaximum": maximum}


def verify_container_case(directory, original, scenario, mode, hdr=False):
    case = directory / mode
    case.mkdir()
    source = case / "sample.jpg"
    source.write_bytes(original)
    before = indexed_frames(original) if scenario.startswith(("multi-", "hdr-")) else [{"data": original}]
    assert AUTHOR.encode() in original
    hdr_before = hdr_rendering(original) if hdr else None
    cleaned = native_cleanup(source, mode)
    output = cleaned.read_bytes()
    assert AUTHOR.encode() not in output and PRIVATE_APP not in output
    after = indexed_frames(output) if len(before) > 1 else [{"data": output}]
    assert len(before) == len(after)
    snapshots = []
    for a, b in zip(before, after, strict=True):
        assert compressed_scan(a["data"]) == compressed_scan(b["data"])
        assert pixel_snapshot(a["data"]) == pixel_snapshot(b["data"])
        if "attributes" in a:
            assert a["attributes"] == b["attributes"] and a["dependencies"] == b["dependencies"]
        with Image.open(io.BytesIO(b["data"])) as image:
            assert image.getexif().get(315) is None
        snapshots.append(pixel_snapshot(b["data"]))
    with Image.open(cleaned) as image:
        # This reader deliberately exposes only the base image for HDR XMP.
        # Both individual indexed JPEGs were decoded above; HDR rendering and
        # the resource association are independently verified below.
        reader_frames = getattr(image, "n_frames", 1)
        assert reader_frames in (1, len(after)) if hdr else reader_frames == len(after)
        for index, snapshot in enumerate(snapshots[:reader_frames]):
            image.seek(index)
            image.load()
            assert hashlib.sha256(image.tobytes()).hexdigest() == snapshot["rawPixels"]
    if len(after) > 1:
        cursor = 0
        for frame in sorted(after, key=lambda frame: frame["start"]):
            assert frame["start"] == cursor
            cursor += frame["size"]
        assert cursor == len(output)
    hdr_after = hdr_rendering(output) if hdr else None
    assert hdr_after == hdr_before
    return {"scenario": scenario, "mode": mode, "frames": snapshots, "hdr": hdr_after,
            "containerReaderFrames": reader_frames,
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(output).hexdigest()}


def verify_application_and_container_cases(output, results):
    scenarios = []
    for colour in ("RGB", "CMYK"):
        original = encoded_image(colour)
        adobe = next((payload for marker, payload, _, _ in headers(original) if marker == 0xee),
                     b"Adobe\x00\x64\x00\x00\x00\x00\x01")
        for kind, marker in (("app2-opaque", 0xe2), ("app14-opaque", 0xee), ("app14-tail", 0xee), ("adobe-control", 0xee)):
            if kind in ("app14-tail", "adobe-control"):
                rest = b"".join(original[start:end] for marker, _, start, end in headers(original) if marker != 0xee)
                payload = adobe + (PRIVATE_APP if kind == "app14-tail" else b"")
                data = b"\xff\xd8" + app_segment(0xee, payload) + rest + compressed_scan(original)
            else:
                data = original[:2] + app_segment(marker, b"private-editor\0" + PRIVATE_APP) + original[2:]
            assert pixel_snapshot(data) == pixel_snapshot(original), "Fixture injection changed pixels or ICC"
            scenarios.append((f"{colour.lower()}-{kind}", data, False))
    for count in (2, 3):
        scenarios.append((f"multi-{count}-frames", encoded_image(frames=count), False))
    for array in (False, True):
        scenarios.append((f"hdr-xmp-{'three-channel' if array else 'single-channel'}", make_hdr_container(array), True))
    for scenario, original, hdr in scenarios:
        directory = output / scenario
        directory.mkdir()
        for mode in ("copy", "replace"):
            results.append(verify_container_case(directory, original, scenario, mode, hdr))
            print(f"Verified {scenario}/{mode}")


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
        verify_application_and_container_cases(output, results)
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
