"""Compare native PDF cleanup with independent parsers and page renderers."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess

from PIL import Image
from pypdf import PdfReader
from reportlab.lib.colors import HexColor
from reportlab.lib.pagesizes import letter
from reportlab.pdfgen import canvas

ROOT = Path(__file__).resolve().parent.parent


def run(arguments, log=None, **kwargs):
    result = subprocess.run(
        arguments, cwd=ROOT, capture_output=True, text=True,
        encoding="utf-8", errors="replace", timeout=600, **kwargs,
    )
    if log:
        log.write_text(result.stdout + result.stderr, encoding="utf-8")
    if result.returncode:
        raise RuntimeError(f"Command failed: {arguments[0]}\n{result.stdout}\n{result.stderr}")
    return result


def make_fixture(path, kind):
    document = canvas.Canvas(str(path), pagesize=letter, pageCompression=1)
    document.setTitle("Synthetic private document title")
    document.setAuthor("Synthetic private author")
    document.setSubject("Independent fidelity fixture")
    document.setFillColor(HexColor("#17263c"))
    document.setFont("Helvetica-Bold", 24)
    document.drawString(54, 728, "Document fidelity fixture")
    document.setFont("Helvetica", 12)
    document.drawString(54, 700, f"Scenario: {kind}. Visible content must remain unchanged.")
    if kind == "form":
        document.drawString(54, 650, "Reviewer")
        document.acroForm.textfield(
            name="reviewer_name", value="Sample reviewer", x=54, y=608,
            width=250, height=28, borderWidth=1, forceBorder=True,
        )
        document.acroForm.checkbox(name="approved", checked=True, x=54, y=562)
        document.drawString(84, 566, "Approval retained")
        document.showPage()
        document.setFont("Helvetica", 12)
        document.drawString(54, 728, "Second page: interactive reference field")
        document.acroForm.textfield(
            name="reference_code", value="MC-2026", x=54, y=660,
            width=250, height=28, borderWidth=1, forceBorder=True,
        )
    elif kind == "image":
        picture = Image.new("RGB", (160, 96))
        picture.putdata([(x % 256, y * 2, (x + y) % 256) for y in range(96) for x in range(160)])
        exif = Image.Exif()
        exif[315] = "Synthetic private image author"
        exif[270] = "Synthetic private image description"
        jpeg = path.with_suffix(".jpg")
        picture.save(jpeg, quality=95, exif=exif)
        # Keep the author's default ASCII85 + DCT filters: qualification must
        # exercise ordinary generated PDFs, not only hand-simplified streams.
        document.drawImage(str(jpeg), 54, 340, width=480, height=288)
        document.saveState()
        document.setFillColor(HexColor("#126edb"))
        document.setFillAlpha(0.4)
        document.rect(90, 310, 220, 110, fill=1, stroke=0)
        document.restoreState()
    else:
        document.setFont("Helvetica", 12)
        for row in range(8):
            y = 646 - row * 38
            document.drawString(66, y, f"Line {row + 1}: searchable text, punctuation (A/B), 0123456789.")
            document.line(54, y - 12, 552, y - 12)
        document.showPage()
        document.setPageRotation(90)
        document.setFont("Helvetica-Bold", 20)
        document.drawString(54, 540, "Rotated second page")
        document.setFont("Helvetica", 12)
        document.drawString(54, 506, "Geometry and text order must survive metadata cleanup.")
        document.setStrokeColor(HexColor("#126edb"))
        document.circle(220, 320, 90, stroke=1, fill=0)
    document.save()


def structure(path):
    reader = PdfReader(path, strict=True)
    fields = reader.get_fields() or {}
    return {
        "pages": [{
            "mediaBox": list(page.mediabox), "cropBox": list(page.cropbox),
            "rotation": page.rotation, "text": page.extract_text(),
            "widgets": [{key: str(annotation.get_object().get(key, ""))
                         for key in ("/T", "/FT", "/V", "/DV", "/AS", "/Rect", "/Ff")}
                        for annotation in page.get("/Annots", [])
                        if annotation.get_object().get("/Subtype") == "/Widget"],
        } for page in reader.pages],
        "fields": {name: {key: str(field.get(key, "")) for key in ("/FT", "/V")}
                   for name, field in fields.items()},
    }


def render(path, prefix, renderer):
    if renderer == "poppler":
        result = run(["pdftoppm", "-r", "144", "-png", str(path), str(prefix)])
        if result.stderr.strip():
            raise RuntimeError(f"Renderer diagnostics: {result.stderr}")
        return sorted(prefix.parent.glob(f"{prefix.name}-*.png"))
    import fitz
    paths = []
    with fitz.open(path) as document:
        for index, page in enumerate(document):
            output = prefix.with_name(f"{prefix.name}-{index + 1}.png")
            page.get_pixmap(matrix=fitz.Matrix(2, 2), alpha=False).save(output)
            paths.append(output)
    return paths


def embedded_images(reader):
    # Read the decoded XObject bytes directly. pypdf's page.images convenience
    # conversion re-encodes JPEGs and can discard EXIF before we inspect it.
    return [Image.open(io.BytesIO(value.get_object().get_data()))
            for value in reader.pages[0]["/Resources"]["/XObject"].values()
            if value.get_object().get("/Subtype") == "/Image"]


def verify_case(directory, kind, mode, renderer):
    fixture = directory / "original.pdf"
    case = directory / mode
    case.mkdir()
    source = case / "sample.pdf"
    shutil.copyfile(fixture, source)
    original = fixture.read_bytes()
    expected = structure(fixture)
    assert len(expected["pages"]) >= 1
    if kind == "form":
        assert set(expected["fields"]) == {"reviewer_name", "approved", "reference_code"}
    native = run([
        "cargo", "test", "--manifest-path", "src-tauri/Cargo.toml", "--lib",
        "cleans_external_pdf_sample_with_verified_output", "--", "--ignored",
    ], log=case / "native.log",
        env={**os.environ, "METACLEAN_PDF_SAMPLE_DIR": str(case), "METACLEAN_PDF_OUTPUT_MODE": mode})
    assert "1 passed; 0 failed" in native.stdout, "Native fixture test did not execute"
    cleaned = source if mode == "replace" else case / "sample.cleaned.pdf"
    preserved = case / "sample.pdf.bak" if mode == "replace" else source
    assert preserved.read_bytes() == original, "Source or replacement backup changed"
    before_pages = render(fixture, case / "before", renderer)
    after_pages = render(cleaned, case / "after", renderer)
    assert len(before_pages) == len(after_pages) == len(expected["pages"])
    page_hashes = []
    for before, after in zip(before_pages, after_pages):
        with Image.open(before) as a, Image.open(after) as b:
            assert a.size == b.size and a.convert("RGB").tobytes() == b.convert("RGB").tobytes(), "Rendered page changed"
            page_hashes.append(hashlib.sha256(a.convert("RGB").tobytes()).hexdigest())
    actual = structure(cleaned)
    assert actual == expected, f"Logical document changed: {kind}/{mode}\nExpected: {expected}\nActual: {actual}"
    reader = PdfReader(cleaned, strict=True)
    assert not reader.metadata, "Document metadata remains"
    assert "/ID" not in reader.trailer, "Document identifier remains"
    if kind == "image":
        original_images = embedded_images(PdfReader(fixture))
        cleaned_images = embedded_images(reader)
        assert len(original_images) == len(cleaned_images) == 1
        before, after = original_images[0], cleaned_images[0]
        assert before.getexif().get(315) == "Synthetic private image author"
        assert not after.getexif(), "Embedded JPEG metadata remains"
        assert before.size == after.size and before.tobytes() == after.tobytes(), "JPEG pixels changed"
    return {"scenario": kind, "mode": mode, "renderer": renderer,
            "pages": len(before_pages), "fields": expected["fields"],
            "sourceSha256": hashlib.sha256(original).hexdigest(),
            "outputSha256": hashlib.sha256(cleaned.read_bytes()).hexdigest(),
            "renderedPageSha256": page_hashes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--renderer", choices=("mupdf", "poppler"), required=True)
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    results = []
    try:
        for kind in ("vector", "form", "image"):
            directory = output / kind
            directory.mkdir()
            make_fixture(directory / "original.pdf", kind)
            for mode in ("copy", "replace"):
                results.append(verify_case(directory, kind, mode, arguments.renderer))
                print(f"Verified {kind}/{mode}: identical pages, text, fields and source/backup")
    finally:
        (output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
