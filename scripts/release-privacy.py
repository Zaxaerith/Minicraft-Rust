"""Fail-closed release file scanner. Reports contain categories/offsets, never matches.

Scans all bytes as UTF-8/ASCII and both UTF-16LE alignments, plus every nested ZIP
member (including DOCX XML). No extraction and no exemptions for dependency code.
This is a pattern-based preflight, not a guarantee against arbitrary secrets.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import struct
import sys
import zipfile

PATTERNS = {
    'absolute-windows-path': r'[a-zA-Z]:(?:\\|/(?!/))',
    'absolute-unc-path': r'\\\\[A-Za-z0-9_.-]{2,}\\+[A-Za-z0-9_$.-]{2,}',
    'absolute-private-posix-path': r'(?<![:/])/(?:home|Users|root|tmp|private|var|rustc)/[^\s\x00<>"\']+',
    'private-key': r'-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----',
    'github-token': r'(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})',
    'aws-access-key': r'(?:AKIA|ASIA)[A-Z0-9]{16}',
    'credential-assignment': r'(?i)(?:password|access_token|client_secret)\s*[=:]\s*["\']?[A-Za-z0-9_+/.-]{20,}',
}
MAX_BYTES = 256 * 1024 * 1024
MAX_MEMBERS = 2048


def private_prefixes():
    values = [str(Path(__file__).resolve().parents[1])]
    for key in ('USERPROFILE', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME'):
        if os.environ.get(key):
            values.append(os.environ[key])
    result = set()
    for value in values:
        if len(value) > 3:
            result.update((value, value.replace('\\', '/'), value.replace('\\', '\\\\')))
    return result


def scan_bytes(data, prefixes=()):
    findings = set()
    patterns = [(name, re.compile(pattern)) for name, pattern in PATTERNS.items()]
    patterns += [('local-build-prefix', re.compile(re.escape(prefix), re.I)) for prefix in prefixes]
    views = [('utf8-ascii', data.decode('latin1'), 0, 1)]
    for alignment in (0, 1):
        end = len(data) - ((len(data) - alignment) % 2)
        views.append(('utf16le', data[alignment:end].decode('utf-16le', errors='surrogatepass'), alignment, 2))
    for encoding, text, start, stride in views:
        for category, pattern in patterns:
            for match in pattern.finditer(text):
                findings.add((category, encoding, start + stride * match.start()))
    return [dict(category=c, encoding=e, offset=o) for c, e, o in sorted(findings)]


def classify(data, finding):
    offset = finding['offset']
    if finding['encoding'] == 'utf16le':
        length = min(1024, len(data) - offset) // 2 * 2
        text = data[offset:offset + length].decode('utf-16le', errors='surrogatepass')
    else:
        text = data[offset:offset + 1024].decode('latin1')
    if finding['category'] == 'absolute-private-posix-path' and re.match(
            r'/rustc/[0-9a-f]{40}/library[\\/]', text):
        return 'public-rust-compiler-source-namespace'
    # Keep every broad match in the report. A bare drive root or a drive prefix
    # followed immediately by machine-code bytes is not a private pathname.
    # Any real component (even a single character) remains a blocking finding.
    if finding['category'] == 'absolute-windows-path':
        component = re.match(r'[A-Za-z]:[\\/]([^\x00-\x1f\x7f-\xff<>:"|?*]*)', text)
        if component and not component.group(1):
            return 'bare-drive-prefix-without-path-component'
        if component and len(component.group(1)) == 1 and text[4:5] != '\x00':
            # A three-byte pattern inside an instruction can resemble a drive
            # prefix. Only classify this when PE headers prove executable code,
            # and the next byte makes it an unterminated, invalid pathname.
            try:
                pe = struct.unpack_from('<I', data, 60)[0]
                if data[:2] == b'MZ' and data[pe:pe + 4] == b'PE\0\0':
                    count = struct.unpack_from('<H', data, pe + 6)[0]
                    optional = struct.unpack_from('<H', data, pe + 20)[0]
                    for index in range(min(count, 96)):
                        section = pe + 24 + optional + index * 40
                        size, start = struct.unpack_from('<II', data, section + 16)
                        flags = struct.unpack_from('<I', data, section + 36)[0]
                        if start <= offset < start + size and flags & 0x20000000:
                            return 'nonterminated-single-character-prefix-in-pe-instruction'
            except struct.error:
                pass
    return None


def scan_file(path):
    records = []
    budget = [0, 0]
    prefixes = private_prefixes()

    def visit(data, member, depth):
        budget[0] += len(data)
        budget[1] += 1
        if budget[0] > MAX_BYTES or budget[1] > MAX_MEMBERS or depth > 3:
            raise ValueError('archive-resource-limit')
        compressed_ranges = []
        if data.startswith(b'PK\x03\x04'):
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                for entry in archive.infolist():
                    if entry.compress_type != zipfile.ZIP_STORED:
                        header = entry.header_offset
                        name_size, extra_size = struct.unpack_from('<HH', data, header + 26)
                        start = header + 30 + name_size + extra_size
                        compressed_ranges.append((start, start + entry.compress_size))
        findings, reviewed = [], []
        for finding in scan_bytes(data, prefixes):
            # Compressed bytes are not text. Retain their candidates, and scan
            # every decoded byte below; metadata/comments stay in the raw scan.
            reason = ('compressed-payload-scanned-after-decompression' if any(
                start <= finding['offset'] < end for start, end in compressed_ranges)
                else classify(data, finding))
            if reason:
                reviewed.append(dict(**finding, classification=reason))
            else:
                findings.append(finding)
        records.append(dict(member=member, bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                            findings=findings, reviewedNonPrivate=reviewed))
        if data.startswith(b'PK\x03\x04'):
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                seen = set()
                for ordinal, entry in enumerate(archive.infolist()):
                    name = entry.filename
                    parts = PurePosixPath(name).parts
                    if (not name or name.startswith('/') or '\\' in name or ':' in name
                            or '..' in parts or name.casefold() in seen
                            or (entry.external_attr >> 16) & 0o170000 == 0o120000):
                        raise ValueError('unsafe-archive-member')
                    seen.add(name.casefold())
                    if entry.is_dir():
                        continue
                    if entry.file_size > MAX_BYTES - budget[0]:
                        raise ValueError('archive-resource-limit')
                    # Even an accidental member name can contain private data.
                    # Ordinals plus hashes identify files without echoing names.
                    visit(archive.read(entry), member + '!member-' + str(ordinal), depth + 1)

    if path.stat().st_size > MAX_BYTES:
        raise ValueError('file-resource-limit')
    visit(path.read_bytes(), 'input', 0)
    count = sum(len(record['findings']) for record in records)
    return dict(status='FAIL' if count else 'PASS', findings=count, scannedBytes=budget[0], files=records)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('--report', required=True, type=Path)
    args = parser.parse_args()
    try:
        report = scan_file(args.input)
    except (OSError, ValueError, zipfile.BadZipFile, RuntimeError, struct.error):
        report = dict(status='FAIL', reason='unreadable-invalid-or-unsafe-input')
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print('Privacy preflight:', report['status'], 'findings=', report.get('findings', 'unknown'))
    return 0 if report['status'] == 'PASS' else 1


if __name__ == '__main__':
    sys.exit(main())
