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

## The bar a format has to clear

A format enters the allowlist only with malformed-input tests, container
boundary validation, payload-preservation evidence and a post-clean inspection
that fails closed on residual traces. Recognizing an extension or deleting a
count of bytes is not enough.

The hard cases all fail the same way: every one of them keeps absolute byte
offsets somewhere — a strip pointer, a cue index, an item extent — and a
metadata block that is simply deleted drags every offset behind it out of
alignment. That is why so many tools either refuse these containers or
re-encode them. MetaClean does neither. Each of the families below is cleaned
by a strategy that removes the metadata **without moving a single byte of the
file**, so the offsets that were true before the clean are still true after it.

| Family | Extensions | How the offsets are kept honest |
| --- | --- | --- |
| TIFF | `.tif`, `.tiff` | Directory entries are deleted by compacting the IFD in place and rewriting the next-directory pointer behind them, so a directory only ever shrinks and the file never shifts. Sub-IFD, Exif, GPS and Interop pointer graphs are walked to a bounded depth; a pointer removed at one level has the directory it named cleaned too, rather than being orphaned. |
| Camera RAW | `.cr2`, `.crw`, `.nef`, `.nrw`, `.arw`, `.srf`, `.sr2`, `.orf`, `.rw2`, `.rwl`, `.dng`, `.pef`, `.srw`, `.3fr`, `.erf`, `.mef`, `.mos`, `.iiq`, `.kdc`, `.dcr`, `.k25` | A raw negative from every major vendor is a TIFF wearing a private magic word, so the same in-place walker covers them. Because nothing moves, strip, tile, MakerNote-relative and embedded-preview offsets stay valid — which is precisely the reason it is safe to strip a negative rather than refuse it. |
| Fujifilm RAF | `.raf` | A RAF is not a TIFF but a fixed header naming three byte ranges, one of which is a JPEG preview carrying a complete EXIF block — GPS, serial number, timestamps, the lot. Refusing the format would leave the worst of the leak in place. The preview is cleaned as an ordinary JPEG, written back at its original offset and zero-padded to its original extent, so only the length field changes and the sensor data is never read or rewritten. |
| HEIF/AVIF | `.heic`, `.heif`, `.heics`, `.heifs`, `.hif`, `.avif`, `.avifs` | Cleaned at item granularity rather than box granularity. In a HEIF the `meta` box is not a place to hang a title — it holds `iinf`, `iloc` and `iprp`, the table that says where the picture is, and deleting it deletes the image. So the item table is read, the items whose payload is an EXIF block, an XMP packet or a C2PA manifest are zeroed only after their unique, non-overlapping extents have been proved to lie inside local `mdat` or `idat` storage, and their extent length is set to zero. Missing, duplicate, external, derived or overlapping private-item locations fail closed. Every other item's offset survives untouched. |
| JPEG XL | `.jxl` | A naked `FF 0A` codestream cannot carry container metadata and is preserved byte-for-byte. In a box container, EXIF, XML/XMP, JUMBF/C2PA, Brotli-wrapped forms and JPEG reconstruction data are renamed to an equal-sized `free` box and zero-filled. The `jxlc` codestream or ordered `jxlp` fragments never move; malformed box lengths and fragment sequences fail closed. |
| Canon CR3 | `.cr3` | An ISO base media file like a HEIC, cleaned by the same item walker, plus the four bare TIFF directories — IFD0, Exif, MakerNote and GPS — that Canon parks inside a private box. |
| AVI | `.avi` | Nothing inside the container is deleted. Some encoders write `idx1` offsets relative to `movi` and some relative to the file, so removing a chunk ahead of the media desynchronises half the players in existence. A private chunk is instead renamed to `JUNK` — RIFF's own padding tag, which every parser is required to skip — and its payload is zeroed. Chained OpenDML `AVIX` RIFF segments are walked and preserved rather than mistaken for trailing data. The file keeps its media offsets and the index keeps its meaning; only bytes after the final complete RIFF form are discarded. |
| Matroska/WebM | `.mkv`, `.mka`, `.mks`, `.mk3d`, `.webm` | EBML defines `Void` for exactly this situation, and permits a length field written wider than strictly necessary. A tag block is retired by stamping the one-byte `Void` identifier over its own and widening the length to absorb the difference, so the element occupies the same bytes it always did and the cue index stays true. Strings the specification insists on keeping are zero-filled instead, which EBML reads back as empty. |
| ASF/WMV/WMA | `.asf`, `.wmv`, `.wma` | ASF defines a padding object so a writer can reserve space it does not intend to use, and every reader must skip it. A content-description or metadata object is retired by stamping the padding GUID over its own and blanking the body. The header's object count stays honest, no length changes, and the media object never moves. |
| BMP | `.bmp`, `.dib` | BMP has no metadata standard, which is exactly why it leaks: the two reserved header words are scratch space editors write ID numbers into, a V5 header can carry an embedded ICC profile, and because the format never declares the file ends at the last pixel, tools staple EXIF or XMP onto the tail. All three are handled; an embedded profile is released back to `LCS_sRGB` rather than left dangling. |

Every strategy above is covered by tests that assert the picture and the media
payload survive byte-identical, that the index, item table or object count still
resolves afterwards, that tags which merely look like metadata are left alone,
and that malformed input fails closed.

## Deep embedded payloads

Container-level success is not sufficient when a document can carry another
privacy-bearing file inside it. MetaClean therefore continues inspection into
two bounded embedded surfaces:

- PDF JPEG image XObjects are inspected and rebuilt with private APP/COM blocks
  removed while orientation and ICC data follow the user's fidelity settings.
- Image data URIs in HTML, XHTML, SVG and Markdown are decoded and run through
  the same native JPEG, PNG, WebP, JPEG XL, GIF, BMP and HEIF cleaners. Nested SVG is
  supported to four levels. Every decoded payload is capped at 16 MiB, while
  the shared 256 MiB input limit bounds the containing source file; valid
  payloads after the first 100 are still inspected instead of being silently
  skipped.

Unknown or oversized embedded data is left unchanged rather than sent through
a generic decoder. WAV C2PA chunks and ID3v2/C2PA prefixes before FLAC streams
follow the same rule: only container structures the cleaner can bound and
re-inspect are retired.

The optional JSON audit export records categories, counts, paths and outcomes
but never raw metadata values. The native writer accepts JSON only, caps the
report at 10 MiB and commits it atomically.

## Still refused

These fail closed. MetaClean does not accept them, and does not pretend to.

| Out of scope | Why |
| --- | --- |
| Legacy binary Office (`.doc`, `.xls`, `.ppt`) | OLE compound documents interleave metadata with content in a structure whose in-place removal cannot be verified to the standard above. |
| Statistical text watermarks | Detecting them requires a model, and removing them requires rewriting the author's prose. MetaClean edits bytes, not meaning — and would have to send text off the device to do otherwise. |
| Pixel-domain watermarks | Removal means re-encoding the image, which contradicts the payload-preservation guarantee that makes the rest of this policy safe. |
| Unknown binary formats | An unrecognized container is refused rather than run through a generic rewrite. |

## Current supported scope

The authoritative allowlist contains 105 extensions and is shared by the Rust
engine, frontend classification and Windows shell integration. It covers 18
still-image extensions across JPEG, PNG, WebP, JPEG XL, GIF, BMP, TIFF, HEIF and AVIF;
23 camera raw formats; 13 audio extensions across MP3, WAV, FLAC, AIFF, WMA and the
MPEG-4 audio brands; 19 video containers across MP4/QuickTime, AVI, ASF/WMV and
Matroska/WebM; DOCX, XLSX, PPTX, 11 OpenDocument formats and EPUB; PDF; and 16
UTF-8 text and markup extensions. CI rejects any mismatch between those
manifests.
