"""Synthetic negative tests; no genuine private paths or credentials in fixtures."""
import importlib.util
from pathlib import Path
import io
import json
import subprocess
import sys
sys.dont_write_bytecode = True
import tempfile
import unittest
import zipfile
import struct

SCRIPT = Path(__file__).with_name('release-privacy.py')
spec = importlib.util.spec_from_file_location('privacy', SCRIPT)
privacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(privacy)


class PrivacyTests(unittest.TestCase):
    def test_path_encodings(self):
        for text in (r'Q:\Users\Synthetic\crate.rs', 'Z:/unrelated/private/build.c',
                     r'\\machine\share\file', '/home/synthetic/code.rs',
                     r'R:\\escaped\\directory\\file.rs'):
            for encoding in ('utf-8', 'utf-16le'):
                with self.subTest(encoding=encoding):
                    self.assertTrue(privacy.scan_bytes(b'x' + text.encode(encoding)))

    def test_credentials(self):
        for text in ('ghp_' + 'X' * 36, 'github_pat_' + 'X' * 50,
                     'AKIA' + 'X' * 16, '-----BEGIN RSA PRIVATE KEY-----',
                     'client_secret=' + 'X' * 32):
            self.assertTrue(privacy.scan_bytes(text.encode()))

    def test_relative_remaps_and_public_urls(self):
        self.assertFalse(privacy.scan_bytes(b'build/cargo/src/lib.rs https://github.com/org/repo GPL-3.0'))

    def test_prefix_not_limited_to_windows(self):
        self.assertTrue(privacy.scan_bytes(b'/custom-cache/build.rs', ['/custom-cache']))

    def test_public_toolchain_and_binary_prefix_classification(self):
        public = b'/rustc/' + b'a' * 40 + b'/library/core/src/lib.rs'
        self.assertEqual(privacy.classify(public, privacy.scan_bytes(public)[0]),
                         'public-rust-compiler-source-namespace')
        for local in (b'/rustc/private/library/core.rs', b'C:\\x', b'C:/private/file'):
            self.assertIsNone(privacy.classify(local, privacy.scan_bytes(local)[0]))
        root = b'C:\\\x00'
        self.assertEqual(privacy.classify(root, privacy.scan_bytes(root)[0]),
                         'bare-drive-prefix-without-path-component')

    def test_code_coincidence_does_not_exempt_real_paths(self):
        for payload, executable, expected in (
                (b'E:\\$\xe0', True, 'nonterminated-single-character-prefix-in-pe-instruction'),
                (b'C:\\x\0', True, None), (b'C:\\private\\file\0', True, None),
                (b'E:\\$\xe0', False, None)):
            data = bytearray(512)
            data[:2] = b'MZ'
            struct.pack_into('<I', data, 60, 64)
            data[64:68] = b'PE\0\0'
            struct.pack_into('<H', data, 70, 1)
            struct.pack_into('<II', data, 104, 128, 256)
            struct.pack_into('<I', data, 124, 0x20000000 if executable else 0)
            data[256:256 + len(payload)] = payload
            finding = next(f for f in privacy.scan_bytes(data) if f['offset'] == 256)
            self.assertEqual(privacy.classify(data, finding), expected)

    def test_cli_failure_redacts_and_checks_nested_docx(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            nested = io.BytesIO()
            with zipfile.ZipFile(nested, 'w') as archive:
                archive.writestr('word/document.xml', r'V:\SyntheticPrivate\file')
            source = root / 'input.zip'
            with zipfile.ZipFile(source, 'w') as archive:
                archive.writestr('license.docx', nested.getvalue())
            report = root / 'report.json'
            result = subprocess.run([sys.executable, str(SCRIPT), str(source), '--report', str(report)], capture_output=True)
            self.assertEqual(result.returncode, 1)
            self.assertNotIn('SyntheticPrivate', report.read_text())
            self.assertEqual(json.loads(report.read_text())['status'], 'FAIL')

    def test_unsafe_zip_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            source = Path(temp) / 'unsafe.zip'
            with zipfile.ZipFile(source, 'w') as archive:
                archive.writestr('../save.json', 'innocuous')
            with self.assertRaises(ValueError):
                privacy.scan_file(source)


if __name__ == '__main__':
    unittest.main()
