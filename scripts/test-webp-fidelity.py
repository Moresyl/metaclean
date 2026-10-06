"""Check that the WebP fidelity gate rejects damaged or privacy-bearing outputs."""
import importlib.util
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("webp_fidelity", Path(__file__).with_name("verify-webp-fidelity.py"))
fidelity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fidelity)


class WebpEvidenceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        source = Path(temporary.name) / "source.webp"
        fidelity.make_fixture(source, "animation-lossy")
        self.original = source.read_bytes()
        self.parts = fidelity.retained_records(fidelity.riff_records(self.original), True)

    def output(self):
        body = b"WEBP" + b"".join(raw for _, raw in self.parts)
        return b"RIFF" + struct.pack("<I", len(body)) + body

    def change_frame(self, transform):
        index = next(index for index, (kind, _) in enumerate(self.parts) if kind == b"ANMF")
        kind, raw = self.parts[index]
        self.parts[index] = (kind, fidelity.encode_chunk(kind, transform(raw[8:])))

    def test_accepts_only_the_requested_profile_policy(self):
        fidelity.check_output(self.original, self.output(), True)
        with self.assertRaisesRegex(AssertionError, "Compressed media"):
            fidelity.check_output(self.original, self.output(), False)
        self.parts = fidelity.retained_records(fidelity.riff_records(self.original), False)
        fidelity.check_output(self.original, self.output(), False)
        with self.assertRaisesRegex(AssertionError, "Compressed media"):
            fidelity.check_output(self.original, self.output(), True)

    def test_rejects_frame_private_data_even_without_the_seeded_identity(self):
        self.change_frame(lambda payload: payload + fidelity.encode_chunk(b"prIv", b"unrelated identity"))
        with self.assertRaisesRegex(AssertionError, "Private WebP chunk remains"):
            fidelity.check_output(self.original, self.output(), True)

    def test_rejects_changed_frame_duration(self):
        self.change_frame(lambda payload: payload[:12] + bytes((payload[12] ^ 1,)) + payload[13:])
        with self.assertRaisesRegex(AssertionError, "Compressed media"):
            fidelity.check_output(self.original, self.output(), True)

    def test_rejects_changed_alpha_or_compressed_pixels(self):
        self.change_frame(lambda payload: payload[:24] + bytes((payload[24] ^ 1,)) + payload[25:])
        with self.assertRaisesRegex(AssertionError, "Compressed media"):
            fidelity.check_output(self.original, self.output(), True)

    def test_rejects_missing_secondary_frame(self):
        removed = False
        kept = []
        for part in reversed(self.parts):
            if not removed and part[0] == b"ANMF":
                removed = True
            else:
                kept.append(part)
        self.parts = list(reversed(kept))
        with self.assertRaisesRegex(AssertionError, "Compressed media"):
            fidelity.check_output(self.original, self.output(), True)

    def test_rejects_wrong_riff_size_and_truncated_payload(self):
        output = self.output()
        with self.assertRaisesRegex(AssertionError, "RIFF size"):
            fidelity.check_output(self.original, output[:-1], True)
        body = output[8:-1]
        with self.assertRaisesRegex(AssertionError, "Truncated WebP"):
            fidelity.check_output(self.original, b"RIFF" + struct.pack("<I", len(body)) + body, True)

    def test_rejects_nonzero_chunk_padding(self):
        odd_chunk = fidelity.encode_chunk(b"prIv", b"x")[:-1] + b"\x01"
        self.parts.append((b"prIv", odd_chunk))
        with self.assertRaisesRegex(AssertionError, "Nonzero WebP padding"):
            fidelity.check_output(self.original, self.output(), True)


if __name__ == "__main__":
    unittest.main()
