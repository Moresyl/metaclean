"""Check that the Office fidelity gate rejects corrupted qualification outputs."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
import sys
from xml.etree import ElementTree as ET
from zipfile import ZipFile

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("office_fidelity", Path(__file__).with_name("verify-office-fidelity.py"))
fidelity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fidelity)


class PackageEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.original = self.directory / "original.docx"
        self.output = self.directory / "output.docx"
        self.identities = fidelity.make_fixture(self.original, "docx", "core", "fragment")
        with ZipFile(self.original) as archive:
            self.entries = {name: archive.read(name) for name in archive.namelist()}
        property_names, _, _, _ = self.identities
        removed = {name for name in self.entries if name.startswith("customXml/") and name not in property_names}
        for name in removed:
            del self.entries[name]
        for name in property_names:
            root = ET.fromstring(self.entries[name])
            root.clear()
            self.entries[name] = ET.tostring(root)
        for name in list(self.entries):
            if name != "[Content_Types].xml" and not name.endswith(".rels"):
                continue
            root = ET.fromstring(self.entries[name])
            for node in list(root):
                target = node.attrib.get("PartName", "").lstrip("/") if name == "[Content_Types].xml" else \
                    fidelity.resolve_target(name, node.attrib["Target"])
                if target in removed:
                    root.remove(node)
            self.entries[name] = ET.tostring(root)
        # Unchanged reference parts must remain byte-identical, including aliases.
        with ZipFile(self.original) as archive:
            for name in self.entries:
                if name.endswith(".rels") and not any(
                        fidelity.resolve_target(name, node.attrib["Target"]) in removed
                        for node in ET.fromstring(archive.read(name))):
                    self.entries[name] = archive.read(name)
        fidelity.write_package(self.output, self.entries)

    def verify(self):
        return fidelity.check_package(self.original, self.output, *self.identities)

    def test_accepts_exact_surviving_content_and_pruned_records(self):
        self.assertEqual(len(self.verify()), 3)

    def test_rejects_changed_visible_body_even_when_xml_remains_valid(self):
        self.entries["word/document.xml"] = self.entries["word/document.xml"].replace(b"Visible table", b"Altered table")
        fidelity.write_package(self.output, self.entries)
        with self.assertRaisesRegex(AssertionError, "Visible or unrelated payload changed"):
            self.verify()

    def test_rejects_private_value_left_in_property_root(self):
        selected = self.identities[1]
        root = ET.fromstring(self.entries[selected])
        ET.SubElement(root, "{http://purl.org/dc/elements/1.1/}creator").text = "Synthetic private author"
        self.entries[selected] = ET.tostring(root)
        fidelity.write_package(self.output, self.entries)
        with self.assertRaisesRegex(AssertionError, "Private properties remain"):
            self.verify()

    def test_rejects_deleted_unrelated_relationship_after_expected_pruning(self):
        name = "word/_rels/document.xml.rels"
        root = ET.fromstring(self.entries[name])
        root.remove(next(node for node in root if node.attrib["Target"] == "styles.xml"))
        self.entries[name] = ET.tostring(root)
        fidelity.write_package(self.output, self.entries)
        with self.assertRaisesRegex(AssertionError, "Relationship or manifest records changed"):
            self.verify()

    def test_rejects_private_comment_even_outside_empty_property_root(self):
        selected = self.identities[1]
        self.entries[selected] += b"<!-- Synthetic private author -->"
        fidelity.write_package(self.output, self.entries)
        with self.assertRaisesRegex(AssertionError, "Private property markup remains"):
            self.verify()

    def test_rejects_removed_unrelated_part(self):
        del self.entries["word/styles.xml"]
        fidelity.write_package(self.output, self.entries)
        with self.assertRaisesRegex(AssertionError, "Unexpected package part changes"):
            self.verify()

    def test_refuses_external_reference_in_internal_uri_resolver(self):
        with self.assertRaisesRegex(AssertionError, "Unexpected external target"):
            fidelity.resolve_target("_rels/.rels", "https://example.invalid/part.xml")


if __name__ == "__main__":
    unittest.main()
