# MetaClean support and privacy policy

MetaClean is a privacy cleaner, not a forensic metadata viewer. It only accepts
a format when it can identify the container, remove the supported privacy
traces without re-encoding the payload, and inspect the candidate bytes again
before any output path, backup or write is created.

## Why metadata values are not displayed

Scan reports intentionally expose only the trace category and count. They do
not return or render raw GPS coordinates, author names, device identifiers,
comments, document history or other metadata values. This keeps sensitive
content out of the React state tree, history records, screenshots, clipboard
and accessibility APIs. File contents and metadata values are never persisted
in MetaClean history.

The tradeoff is deliberate: MetaClean is less suitable than a forensic viewer
for inspecting or copying individual tag values. The application instead shows
the file, detected format, actionable categories and counts, explicit errors,
source/output sizes and output location. Cleanup is verified against the exact
in-memory candidate before it can be written.

## Explicitly refused format families

The following extensions are not aliases for an existing cleaner. They remain
outside the intake allowlist and fail closed if supplied directly.

| Family | Extensions | Safety decision |
| --- | --- | --- |
| TIFF | `.tif`, `.tiff` | Refused. Multi-IFD pointer graphs, embedded previews and MakerNote-relative offsets require a dedicated parser and rewriter; applying the JPEG EXIF strategy would risk image or camera-data corruption. |
| HEIF/AVIF | `.heic`, `.heif`, `.avif` | Refused. These ISO BMFF still-image brands use a `meta` item graph (`iloc`, `iinf`, `iref` and `iprp`) rather than the media-atom layout handled by MetaClean. The video cleaner explicitly rejects image brands. |
| BMP | `.bmp` | Refused. V5 headers can reference embedded or linked color profiles. MetaClean has no BMP-specific profile/offset rewriter and will not silently discard color behavior. |
| Camera RAW | `.arw`, `.cr2`, `.cr3`, `.dng`, `.nef`, `.nrw`, `.orf`, `.pef`, `.raf`, `.rw2`, `.srw` | Refused individually. These formats mix proprietary MakerNotes, preview images and offset-sensitive TIFF or ISO BMFF structures; a generic metadata rewrite is not considered safe. |
| AVI | `.avi` | Refused. AVI uses RIFF, but its `LIST`/`INFO`, `IDIT` and embedded metadata layout is not interchangeable with the format-specific WAV cleaner. |
| Matroska/WebM | `.mkv`, `.webm` | Refused. EBML unknown-size elements, segment information, tags, attachments, chapters and CRC elements require a dedicated structural rewriter. |
| ASF/WMV | `.asf`, `.wmv` | Refused. Header extensions, content descriptions, metadata objects and indexes require an ASF-specific parser and verifier. |

A future format may move into the supported list only with malformed-input
tests, container-boundary validation, payload-preservation evidence and a
post-clean inspection that fails closed on residual traces. Merely recognizing
an extension or deleting a count of bytes is insufficient.

## Current supported scope

The authoritative allowlist contains 47 extensions and is shared by the Rust
engine, frontend classification and Windows shell integration. It covers JPEG,
PNG, WebP and GIF; MP3, WAV and FLAC; 17 validated ISO BMFF/QuickTime media
extensions; DOCX, XLSX, PPTX and ODT; PDF; and 16 UTF-8 text/markup extensions.
CI rejects any mismatch between those manifests.
