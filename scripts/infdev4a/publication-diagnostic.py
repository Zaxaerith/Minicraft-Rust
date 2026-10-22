"""Bounded, ordinary-user MoveFileW experiments; never retry or remove evidence.

Each case gets a new owned directory beside the real storage Smoke. This is a
Win32 API trace, NOT ProcMon: kernel stacks/filter attribution remain UNKNOWN.
No ACL changes, elevated token, service/driver installation or settings changes.
"""
import argparse
import atexit
import ctypes as c
from ctypes import wintypes as w
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def api(dll, name, result, *args):
    function = getattr(dll, name)
    function.restype, function.argtypes = result, args
    return function


kernel = c.WinDLL('kernel32', use_last_error=True)
advapi = c.WinDLL('advapi32', use_last_error=True)
create = api(kernel, 'CreateFileW', w.HANDLE, w.LPCWSTR, w.DWORD, w.DWORD,
             c.c_void_p, w.DWORD, w.DWORD, w.HANDLE)
close = api(kernel, 'CloseHandle', w.BOOL, w.HANDLE)
move = api(kernel, 'MoveFileW', w.BOOL, w.LPCWSTR, w.LPCWSTR)
flush = api(kernel, 'FlushFileBuffers', w.BOOL, w.HANDLE)
final_name = api(kernel, 'GetFinalPathNameByHandleW', w.DWORD, w.HANDLE, w.LPWSTR, w.DWORD, w.DWORD)
volume_path = api(kernel, 'GetVolumePathNameW', w.BOOL, w.LPCWSTR, w.LPWSTR, w.DWORD)
volume_info = api(kernel, 'GetVolumeInformationW', w.BOOL, w.LPCWSTR, w.LPWSTR, w.DWORD,
                  c.POINTER(w.DWORD), c.POINTER(w.DWORD), c.POINTER(w.DWORD), w.LPWSTR, w.DWORD)
get_attributes = api(kernel, 'GetFileAttributesW', w.DWORD, w.LPCWSTR)
get_security = api(advapi, 'GetNamedSecurityInfoW', w.DWORD, w.LPCWSTR, w.DWORD, w.DWORD,
                   c.c_void_p, c.c_void_p, c.c_void_p, c.c_void_p, c.POINTER(c.c_void_p))
free = api(kernel, 'LocalFree', c.c_void_p, c.c_void_p)
get_current = api(kernel, 'GetCurrentProcess', w.HANDLE)
open_token = api(advapi, 'OpenProcessToken', w.BOOL, w.HANDLE, w.DWORD, c.POINTER(w.HANDLE))
duplicate_token = api(advapi, 'DuplicateToken', w.BOOL, w.HANDLE, c.c_int, c.POINTER(w.HANDLE))


class Mapping(c.Structure):
    _fields_ = [(name, w.DWORD) for name in ('read', 'write', 'execute', 'all')]


access_check = api(advapi, 'AccessCheck', w.BOOL, c.c_void_p, w.HANDLE, w.DWORD,
                   c.POINTER(Mapping), c.c_void_p, c.POINTER(w.DWORD), c.POINTER(w.DWORD), c.POINTER(w.BOOL))
INVALID = c.c_void_p(-1).value
ROOT = Path(__file__).resolve().parents[2]


def stamp():
    return dict(utc=datetime.now(timezone.utc).isoformat(), pid=os.getpid(), process=Path(sys.executable).name)


def masked(path):
    value = str(path)
    for prefix, label in ((str(ROOT), '<project>'), (str(Path.home()), '<home>')):
        value = value.replace(prefix, label)
    return value


def opened(path, share, directory, events, access=0x80000000):
    handle = create(str(path), access, share, None, 3, 0x02000000 if directory else 0, None)
    error = c.get_last_error() if handle == INVALID else 0
    events.append(dict(**stamp(), operation='CreateFileW', path=masked(path), access=access,
                       share=share, directory=directory, handle=handle, win32=error))
    if handle == INVALID:
        raise OSError(error, 'diagnostic CreateFileW failed')
    return handle


def closed(handle, events):
    ok = bool(close(handle))
    events.append(dict(**stamp(), operation='CloseHandle', handle=handle, success=ok,
                       win32=0 if ok else c.get_last_error()))
    if not ok:
        raise RuntimeError('diagnostic CloseHandle failed')


def rights(path, requested):
    descriptor = c.c_void_p()
    error = get_security(str(path), 1, 7, None, None, None, None, c.byref(descriptor))
    if error:
        return dict(query_win32=error)
    token, impersonation = w.HANDLE(), w.HANDLE()
    try:
        if not open_token(get_current(), 10, c.byref(token)) or not duplicate_token(token, 2, c.byref(impersonation)):
            return dict(token_win32=c.get_last_error())
        privileges, size = c.create_string_buffer(4096), w.DWORD(4096)
        granted, allowed = w.DWORD(), w.BOOL()
        mapping = Mapping(0x120089, 0x120116, 0x1200a0, 0x1f01ff)
        ok = bool(access_check(descriptor, impersonation, requested, c.byref(mapping),
                               privileges, c.byref(size), c.byref(granted), c.byref(allowed)))
        return dict(api_succeeded=ok, requested=requested, allowed=bool(allowed.value),
                    granted=granted.value, win32=0 if ok else c.get_last_error(),
                    limit='DACL/token check only; not a sharing or filesystem-filter test')
    finally:
        if impersonation: close(impersonation)
        if token: close(token)
        free(descriptor)


def path_facts(path, events):
    absolute = Path(os.path.abspath(path))
    attributes = get_attributes(str(absolute))
    result = dict(supplied=masked(path), absolute=masked(absolute),
                  utf16_units=len(str(path).encode('utf-16le')) // 2,
                  attributes=None if attributes == 0xffffffff else attributes,
                  attribute_win32=c.get_last_error() if attributes == 0xffffffff else 0)
    parent = absolute if absolute.exists() else absolute.parent
    volume = c.create_unicode_buffer(32768)
    if volume_path(str(parent), volume, len(volume)):
        serial, maximum, flags = w.DWORD(), w.DWORD(), w.DWORD()
        filesystem = c.create_unicode_buffer(64)
        ok = volume_info(volume.value, None, 0, c.byref(serial), c.byref(maximum), c.byref(flags), filesystem, len(filesystem))
        result['volume'] = dict(root=volume.value, filesystem=filesystem.value, query_ok=bool(ok),
                                serial_hash=hashlib.sha256(str(serial.value).encode()).hexdigest())
    else:
        result['volume_win32'] = c.get_last_error()
    result['ancestor_reparse_points'] = [masked(p) for p in [parent, *parent.parents]
                                         if get_attributes(str(p)) != 0xffffffff and get_attributes(str(p)) & 0x400]
    if absolute.exists():
        handle = opened(absolute, 7, absolute.is_dir(), events, access=0)
        try:
            buffer = c.create_unicode_buffer(32768)
            count = final_name(handle, buffer, len(buffer), 0)
            result['handle_normalized'] = masked(buffer.value) if 0 < count < len(buffer) else None
        finally:
            closed(handle, events)
        result['delete_access'] = rights(absolute, 0x10000)
    result['parent_delete_child_access'] = rights(absolute.parent, 0x40)
    return result


def owners(files):
    """Restart Manager is a best-effort file owner snapshot, never absence proof."""
    manager = c.WinDLL('rstrtmgr', use_last_error=True)

    class Process(c.Structure):
        _fields_ = [('pid', w.DWORD), ('start', w.FILETIME)]

    class Info(c.Structure):
        _fields_ = [('process', Process), ('app', w.WCHAR * 256), ('service', w.WCHAR * 64),
                    ('type', w.DWORD), ('status', w.DWORD), ('session', w.DWORD), ('restartable', w.BOOL)]

    start = api(manager, 'RmStartSession', w.DWORD, c.POINTER(w.DWORD), w.DWORD, w.LPWSTR)
    register = api(manager, 'RmRegisterResources', w.DWORD, w.DWORD, w.UINT, c.POINTER(w.LPCWSTR),
                    w.UINT, c.c_void_p, w.UINT, c.c_void_p)
    query = api(manager, 'RmGetList', w.DWORD, w.DWORD, c.POINTER(w.UINT), c.POINTER(w.UINT), c.POINTER(Info), c.POINTER(w.DWORD))
    end = api(manager, 'RmEndSession', w.DWORD, w.DWORD)
    session, key = w.DWORD(), c.create_unicode_buffer(33)
    error = start(c.byref(session), 0, key)
    if error: return dict(start_win32=error)
    try:
        names = (w.LPCWSTR * len(files))(*map(str, files))
        error = register(session, len(files), names, 0, None, 0, None)
        if error: return dict(register_win32=error)
        needed, count, reason = w.UINT(), w.UINT(), w.DWORD()
        error = query(session, c.byref(needed), c.byref(count), None, c.byref(reason))
        if error not in (0, 234) or needed.value > 128: return dict(query_win32=error, count=needed.value)
        data = (Info * needed.value)()
        count.value = needed.value
        error = query(session, c.byref(needed), c.byref(count), data, c.byref(reason))
        return dict(query_win32=error, processes=[dict(pid=p.process.pid, app=p.app) for p in data[:count.value]] if not error else [],
                    limit='files only, not directory/driver ownership; transient holders may already have closed')
    finally:
        end(session)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=int, default=4, choices=range(1, 13))
    parser.add_argument('--hold', type=Path, help=argparse.SUPPRESS)
    parser.add_argument('--directory', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.hold:
        events = []
        handle = opened(args.hold, 3, args.directory, events)
        print(json.dumps(dict(ready=True, events=events)), flush=True)
        sys.stdin.readline()
        closed(handle, events)
        print(json.dumps(dict(closed=True, events=events[-1:])), flush=True)
        return 0
    output = ROOT / 'target/validation/current'
    root = Path(tempfile.mkdtemp(prefix='infdev-publication-diag-', dir=output))
    report = dict(schema=1, **stamp(), root=masked(root), procmon='NOT AVAILABLE; no kernel stacks or filter attribution', cases=[])
    destination = root / 'observations.json'
    # Also flush partial observations if an unexpected API exception interrupts
    # a case. Never lose the failed fixture or pretend incomplete work passed.
    atexit.register(lambda: destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8'))
    modes = ['closed'] * args.samples + ['own-child', 'own-directory', 'external-child', 'external-directory']
    for index, mode in enumerate(modes):
        events = []
        case = dict(index=index, mode=mode, events=events, status='started')
        report['cases'].append(case)
        parent = root / f'case-{index:02}-{mode}' / 'infdev-v2'
        parent.mkdir(parents=True)
        stage, target = parent / '.new-stage', parent / 'Surface42'
        stage.mkdir()
        for name in ('chunks', 'nodes'): (stage / name).mkdir()
        payload = b'isolated publication evidence\n'
        for name in ('checkpoint.json', 'session.lock'):
            file = stage / name
            file.write_bytes(payload if name != 'session.lock' else b'')
            handle = opened(file, 0, False, events, access=0xc0000000)
            ok = bool(flush(handle))
            events.append(dict(**stamp(), operation='FlushFileBuffers', handle=handle, success=ok,
                               win32=0 if ok else c.get_last_error()))
            closed(handle, events)
        case.update(source=path_facts(stage, events), target=path_facts(target, events))
        held, child = None, None
        try:
            if mode.startswith('own-'):
                held = opened(stage if mode.endswith('directory') else stage / 'session.lock', 3, mode.endswith('directory'), events)
                case['known_holder_pid'] = os.getpid()
            if mode.startswith('external-'):
                command = [sys.executable, str(Path(__file__).resolve()), '--hold', str(stage if mode.endswith('directory') else stage / 'session.lock')]
                if mode.endswith('directory'): command.append('--directory')
                child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
                ready = json.loads(child.stdout.readline())
                assert ready['ready']
                events.extend(ready['events'])
                case['known_holder_pid'] = child.pid
            if mode != 'closed': case['restart_manager_before'] = owners([stage / 'session.lock', stage / 'checkpoint.json'])
            before = time.perf_counter()
            success = bool(move(str(stage), str(target)))
            error = 0 if success else c.get_last_error()
            events.append(dict(**stamp(), operation='MoveFileW', success=success, win32=error,
                               elapsed_ms=(time.perf_counter() - before) * 1000))
            case.update(success=success, win32=error, expected='success' if mode == 'closed' else 'sharing refusal',
                        target_exists=target.exists(), stage_exists=stage.exists(),
                        payload_unchanged=((target if success else stage) / 'checkpoint.json').read_bytes() == payload)
            if not success: case['restart_manager_after'] = owners([stage / 'session.lock', stage / 'checkpoint.json'])
        finally:
            if held is not None: closed(held, events)
            if child:
                stdout, stderr = child.communicate('release\n', timeout=10)
                case['child_exit'] = child.returncode
                if stdout: events.extend(json.loads(stdout)['events'])
                if stderr: case['child_stderr_sha256'] = hashlib.sha256(stderr.encode()).hexdigest()
        case['status'] = 'complete'
        destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
        print(f'case={index} mode={mode} success={case["success"]} win32={case["win32"]} retained=True', flush=True)
    expected = all(case['payload_unchanged'] and (case['success'] == (case['mode'] == 'closed')) for case in report['cases'])
    report['controlled_expectations_pass'] = expected
    report['historical_root_cause'] = 'UNKNOWN; controlled holders do not attribute the historical error'
    destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print('Evidence:', masked(destination))
    return 0 if expected else 1


if __name__ == '__main__':
    sys.exit(main())
