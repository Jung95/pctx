#!/usr/bin/env python3
"""Isolated forward-source replacement/data-retention evidence, not version migration.

No models, schedules, hooks, network, registered commands or global installation.
Requires POSIX process groups; run serially after performance measurements.
"""
import argparse
import base64
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import sqlite3
import subprocess
import tarfile
import tempfile
import time


class Failure(Exception):
    def __init__(self, code, phase):
        self.code, self.phase = code, phase


def require(condition, code, phase):
    if not condition:
        raise Failure(code, phase)


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536), b''):
            h.update(chunk)
    return h.hexdigest()


def unpack(archive, destination, expected_binary, expected_archive):
    """Same archive allowlist as validate-archive.py, with aggregate bounds.

    Extraction is into an exclusive private directory, with data-filtered regular
    files/directories only. No links, traversal, private spec or private state.
    """
    phase = 'archive_validation'
    digest = sha(archive)
    sidecar = archive.with_suffix(archive.suffix + '.sha256')
    require(sidecar.is_file(), 'ARCHIVE_CHECKSUM_MISSING', phase)
    require(sidecar.read_text().split()[0] == digest, 'ARCHIVE_CHECKSUM_MISMATCH', phase)
    if expected_archive:
        require(expected_archive == digest, 'ARCHIVE_BINDING_MISMATCH', phase)
    with tarfile.open(archive, 'r:gz') as tf:
        entries = tf.getmembers()
        require(0 < len(entries) <= 4096, 'ARCHIVE_ENTRY_BOUND', phase)
        seen, tops, total = set(), set(), 0
        for entry in entries:
            path = Path(entry.name)
            require(path.parts and not path.is_absolute() and '..' not in path.parts,
                    'ARCHIVE_PATH_DENIED', phase)
            require(entry.isfile() or entry.isdir(), 'ARCHIVE_TYPE_DENIED', phase)
            require(not any(part.startswith('._') or part in {'.git', '.toolchain', '.pctx', '.codex'}
                            for part in path.parts), 'ARCHIVE_PRIVATE_PATH_DENIED', phase)
            require(path.name != 'PCTX-implementation-spec-v0.6.md', 'PRIVATE_SPEC_DENIED', phase)
            key = path.as_posix()
            require(key not in seen, 'ARCHIVE_DUPLICATE_DENIED', phase)
            seen.add(key)
            tops.add(path.parts[0])
            total += entry.size
            require(0 <= entry.size <= 256 * 1024 * 1024 and total <= 512 * 1024 * 1024,
                    'ARCHIVE_SIZE_BOUND', phase)
        require(len(tops) == 1, 'ARCHIVE_LAYOUT_DENIED', phase)
        tf.extractall(destination, filter='data')
    bundle = destination / tops.pop()
    executable = bundle / 'pctx'
    require(executable.is_file() and sha(executable) == expected_binary,
            'BINARY_BINDING_MISMATCH', phase)
    for relative in ['LICENSE', 'THIRD_PARTY_NOTICES.txt', 'sbom.json', 'README.md',
                     'docs/cli/install.md', 'docs/cli/formats.md',
                     'docs/PCTX-implementation-spec-v0.6-en.md']:
        require((bundle / relative).is_file(), 'PACKAGE_REQUIRED_FILE_MISSING', phase)
    for _, target in re.findall(r'\[([^]]+)\]\(([^)]+)\)', (bundle / 'README.md').read_text()):
        if target.startswith('docs/'):
            require('..' not in Path(target).parts and (bundle / target).is_file(),
                    'PACKAGE_LINK_MISSING', phase)
    return executable, {'archive_sha256': digest, 'binary_sha256': expected_binary,
                        'entry_count': len(entries), 'allowlist': 'validate-archive.py regular-file/directory contract'}


def logical_databases(data):
    """Compare logical rows, never SQLite physical bytes/WAL layout.

    Fixture databases only; row digests avoid printing task/session/private data.
    Values are typed and blobs are base64 encoded before hashing.
    """
    snapshots = {}
    for path in sorted(data.rglob('*.sqlite3')):
        require(not path.is_symlink(), 'DATABASE_LINK_DENIED', 'logical_snapshot')
        db = sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=2)
        try:
            tables = db.execute("SELECT name,sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetchall()
            snapshot = {}
            for name, schema in tables:
                quoted = '"' + name.replace('"', '""') + '"'
                rows = []
                for row in db.execute('SELECT * FROM ' + quoted):
                    values = [{'blob': base64.b64encode(v).decode()} if isinstance(v, bytes) else v for v in row]
                    rows.append(hashlib.sha256(json.dumps(values, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest())
                    require(len(rows) <= 10000, 'DATABASE_ROW_BOUND', 'logical_snapshot')
                snapshot[name] = {'schema': schema, 'rows': sorted(rows)}
            snapshots[path.relative_to(data).as_posix()] = snapshot
        finally:
            db.close()
    require(bool(snapshots), 'DATABASE_SNAPSHOT_EMPTY', 'logical_snapshot')
    return snapshots


def retained(before, after):
    for database, tables in before.items():
        require(database in after, 'DATABASE_LOST', 'retention')
        for name, old in tables.items():
            require(name in after[database], 'TABLE_LOST', 'retention')
            new = after[database][name]
            require(not (Counter(old['rows']) - Counter(new['rows'])), 'LOGICAL_ROWS_LOST_OR_CHANGED', 'retention')


def file_hashes(root, exclude_sqlite=False):
    return {p.relative_to(root).as_posix(): sha(p) for p in sorted(root.rglob('*'))
            if p.is_file() and not (exclude_sqlite and
                (p.name.endswith('.sqlite3') or p.name.endswith('.sqlite3-wal') or
                 p.name.endswith('.sqlite3-shm') or p.name.endswith('.sqlite3-journal')))}


class Harness:
    def __init__(self, root):
        self.root = root
        self.project = root / 'project'
        self.data = root / 'data'
        self.prefix = root / 'install'
        self.executable = self.prefix / 'bin/pctx'
        for directory in [self.project, self.executable.parent, root / 'home', root / 'tmp', root / 'captures']:
            directory.mkdir(parents=True)
        # Deliberately allowlisted environment: no inherited actor/capability,
        # adapter, credential, host user config, PATH or proxy state.
        self.env = {'HOME': str(root / 'home'), 'USERPROFILE': str(root / 'home'),
                    'TMPDIR': str(root / 'tmp'), 'TMP': str(root / 'tmp'), 'TEMP': str(root / 'tmp'),
                    'PCTX_DATA_DIR': str(self.data), 'PCTX_USER_CONFIG': str(root / 'absent-config'),
                    'PCTX_ACTOR': 'owner', 'LANG': 'C.UTF-8', 'LC_ALL': 'C',
                    'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull,
                    'GIT_TERMINAL_PROMPT': '0'}
        self.steps = []

    def install(self, source):
        pending = self.executable.with_name('pctx.pending')
        with source.open('rb') as src, pending.open('xb') as dst:
            shutil.copyfileobj(src, dst, 65536)
            dst.flush()
            os.fsync(dst.fileno())
        pending.chmod(0o700)
        os.replace(pending, self.executable)
        directory = os.open(self.executable.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)

    def run(self, phase, arguments, version=False):
        output_path = self.root / 'captures' / (str(len(self.steps)) + '.stdout')
        error_path = output_path.with_suffix('.stderr')
        argv = [str(self.executable)] + (arguments if version else ['--root', str(self.project), '--format', 'json'] + arguments)
        started = time.monotonic()
        with output_path.open('xb') as stdout, error_path.open('xb') as stderr:
            child = subprocess.Popen(argv, env=self.env, cwd=self.project, stdin=subprocess.DEVNULL,
                                     stdout=stdout, stderr=stderr, start_new_session=True)
            try:
                while True:
                    if max(output_path.stat().st_size, error_path.stat().st_size) > 1024 * 1024:
                        raise subprocess.TimeoutExpired(argv, 30)
                    code = child.poll()
                    if code is not None:
                        break
                    if time.monotonic() - started >= 30:
                        raise subprocess.TimeoutExpired(argv, 30)
                    time.sleep(0.01)
            except subprocess.TimeoutExpired:
                # Signal the owned group before reaping its root; never run fixture
                # commands that create independent/background jobs.
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    raise Failure('CHILD_CLEANUP_UNCONFIRMED', phase) from None
                raise Failure('CHILD_TIMEOUT', phase) from None
            finally:
                if child.poll() is None:
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    child.wait(timeout=5)
        require(output_path.stat().st_size <= 1024 * 1024 and error_path.stat().st_size <= 1024 * 1024,
                'CLI_CAPTURE_BOUND', phase)
        require(code == 0, 'CLI_NONZERO', phase)
        self.steps.append({'phase': phase, 'exit': code, 'elapsed_ms': round((time.monotonic() - started) * 1000)})
        if version:
            text = output_path.read_text().strip()
            require(bool(re.fullmatch(r'pctx [A-Za-z0-9.+-]+', text)), 'VERSION_SHAPE', phase)
            return text
        value = json.loads(output_path.read_bytes())
        require(value.get('status') == 'ok', 'CLI_STATUS_NOT_OK', phase)
        return value


def validate(args):
    require(os.name == 'posix', 'POSIX_FIXTURE_REQUIRED', 'platform')
    with tempfile.TemporaryDirectory(prefix='pctx-source-upgrade-') as td:
        root = Path(td).resolve()
        root.chmod(0o700)
        old, old_binding = unpack(args.old_archive.resolve(), root / 'old', args.old_binary_sha256, args.old_archive_sha256)
        new, new_binding = unpack(args.new_archive.resolve(), root / 'new', args.new_binary_sha256, args.new_archive_sha256)
        require(args.old_binary_sha256 != args.new_binary_sha256, 'IDENTICAL_BINARY_NOT_UPGRADE', 'binding')
        require(args.old_source_revision != args.new_source_revision, 'IDENTICAL_SOURCE_NOT_UPGRADE', 'binding')
        h = Harness(root)
        source = h.project / 'auth.py'
        source.write_text('def upgrade_auth():\n    return "retained-original"\n')
        h.install(old)
        old_version = h.run('old_version', ['--version'], version=True)
        old_init = h.run('old_init', ['init'])
        h.run('old_index', ['index', 'update'])
        found = h.run('old_symbol_lookup', ['find', 'upgrade_auth', '--kind', 'symbol'])
        require(any(item.get('path') == 'auth.py' for item in found['data']['items']), 'OLD_LOOKUP_MISSING', 'old_symbol_lookup')
        checkpoint = h.run('old_checkpoint', ['checkpoint', 'create', '--name', 'upgrade-baseline', '--pin'])['data']
        checkpoint_id = checkpoint['id']
        definition = root / 'task.json'
        definition.write_text(json.dumps({'schema_version': 1, 'title': 'Isolated source upgrade retention',
            'scope': ['auth.py'], 'acceptance': [{'id': 'retained', 'description': 'Source remains available', 'evidence_check_keys': ['unit']}],
            'checks': [{'key': 'unit', 'kind': 'test'}]}))
        task = h.run('old_task', ['task', 'create', '--from-file', str(definition)])['data']['task_id']
        agent = h.run('old_agent', ['agent', 'register', '--name', 'upgrade-fixture', '--kind', 'agent'])['data']['agent_id']
        session = h.run('old_session', ['session', 'attach', '--agent', agent, '--runtime', 'manual', '--adapter-version', 'fixture-v1'])['data']['session_id']
        packet = h.run('old_context', ['context', 'get', '--task-id', task, '--session', session,
                     '--scope', 'auth.py', '--budget-bytes', '12000'])['data']
        require(bool(packet.get('context_id')), 'CONTEXT_RECEIPT_MISSING', 'old_context')
        baseline_db = logical_databases(h.data)
        baseline_project = file_hashes(h.project)
        require(checkpoint['files'].get('auth.py') == sha(source), 'CHECKPOINT_SOURCE_HASH_MISSING', 'old_checkpoint')
        registry_before = json.loads((h.data / 'registry.json').read_bytes())
        h.install(new)
        require(sha(h.executable) == args.new_binary_sha256, 'INSTALLED_BINARY_MISMATCH', 'new_install')
        require(logical_databases(h.data) == baseline_db, 'REPLACEMENT_MUTATED_DATA', 'new_install')
        new_version = h.run('new_version', ['--version'], version=True)
        current = h.run('new_status', ['status'])
        for key in ['project_id', 'workspace_id']:
            require(current[key] == old_init[key], 'PROJECT_BINDING_CHANGED', 'new_status')
        require(json.loads((h.data / 'registry.json').read_bytes()) == registry_before, 'REGISTRY_BINDING_CHANGED', 'new_status')
        found_new = h.run('new_existing_index_lookup', ['find', 'upgrade_auth', '--kind', 'symbol'])
        require(any(item.get('path') == 'auth.py' for item in found_new['data']['items']), 'NEW_LOOKUP_MISSING', 'new_existing_index_lookup')
        require(found_new['generation_id'] == found['generation_id'], 'INDEX_GENERATION_CHANGED', 'new_existing_index_lookup')
        read = h.run('new_read', ['read', 'auth.py'])
        require('retained-original' in read['data']['text'], 'NEW_SOURCE_READ_MISSING', 'new_read')
        listed = h.run('new_checkpoint_list', ['checkpoint', 'list'])['data']
        require(any(item == dict(checkpoint, policy_filtered=True) for item in listed['items']), 'CHECKPOINT_MANIFEST_NOT_RETAINED', 'new_checkpoint_list')
        shown = h.run('new_task_show', ['task', 'show', task])['data']
        require(shown['task_id'] == task, 'TASK_NOT_RETAINED', 'new_task_show')
        reconciled = h.run('new_session_reconcile', ['session', 'reconcile', '--session', session])['data']
        require(reconciled['session_id'] == session, 'SESSION_NOT_RETAINED', 'new_session_reconcile')
        retained(baseline_db, logical_databases(h.data))
        require(file_hashes(h.project) == baseline_project, 'SOURCE_OR_CONFIG_MUTATED', 'retention')
        source.write_text('def upgrade_auth():\n    return "explicit-fixture-change"\n')
        changes = h.run('new_changes', ['changes', '--since', checkpoint_id])['data']['items']
        require(any(item.get('path') == 'auth.py' and item.get('change') == 'modified' for item in changes), 'MODIFICATION_NOT_OBSERVED', 'new_changes')
        changed_read = h.run('new_changed_read', ['read', 'auth.py'])['data']['text']
        require('explicit-fixture-change' in changed_read and 'retained-original' not in changed_read, 'READ_STALE', 'new_changed_read')
        project_before_removal = file_hashes(h.project)
        db_before_removal = logical_databases(h.data)
        data_before_removal = file_hashes(h.data, exclude_sqlite=True)
        shutil.rmtree(h.prefix)
        require(not h.executable.exists() and file_hashes(h.project) == project_before_removal
                and logical_databases(h.data) == db_before_removal and file_hashes(h.data, exclude_sqlite=True) == data_before_removal,
                'REMOVAL_DAMAGED_RETAINED_DATA', 'removal')
        return {'schema_version': 1, 'status': 'passed', 'claim': 'forward_source_replacement_and_data_retention',
            'old': dict(old_binding, cli_version=old_version, source_revision=args.old_source_revision, source_binding='caller_supplied'),
            'new': dict(new_binding, cli_version=new_version, source_revision=args.new_source_revision, source_binding='caller_supplied'),
            'same_cli_version': old_version == new_version, 'forward_version_upgrade': 'not_verified',
            'schema_migration': 'not_verified', 'semantic_differences': 'not_verified',
            'checks': {'same_project_workspace_control_binding': True, 'existing_index_lookup': True,
                'checkpoint_manifest_retained': True, 'task_session_context_logical_rows_retained': True,
                'source_config_immutable_before_explicit_fixture_edit': True, 'explicit_modification_detected': True,
                'read_returns_current_source': True, 'prefix_removal_preserves_project_and_data': True},
            'logical_database_count': len(baseline_db), 'commands': h.steps,
            'provider_usage': 'unknown', 'delivery_receipt': 'not_observed',
            'limitations': ['POSIX isolated CLI fixture only; no Windows runtime proof',
                'source revision/order binding supplied by caller; ancestry unknown; binary/archive hashes verified',
                'no version transition, schema migration, rollback or all historical versions proof',
                'owned process-group timeout cleanup; no escaped-process containment claim']}


def digest(value):
    if not re.fullmatch(r'[0-9a-f]{64}', value):
        raise argparse.ArgumentTypeError('expected lowercase SHA256')
    return value


def revision(value):
    if not re.fullmatch(r'[0-9a-f]{40}', value):
        raise argparse.ArgumentTypeError('expected full lowercase source revision')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('old_archive', type=Path)
    parser.add_argument('new_archive', type=Path)
    for generation in ['old', 'new']:
        parser.add_argument('--' + generation + '-binary-sha256', required=True, type=digest)
        parser.add_argument('--' + generation + '-archive-sha256', type=digest)
        parser.add_argument('--' + generation + '-source-revision', required=True, type=revision)
    args = parser.parse_args()
    try:
        report = validate(args)
    except Failure as error:
        report = {'schema_version': 1, 'status': 'failed', 'code': error.code, 'phase': error.phase,
                  'claim': 'forward_source_replacement_and_data_retention', 'raw_cli_output': 'omitted'}
    except Exception as error:
        # Never include native filenames, subprocess output, task data or secrets.
        report = {'schema_version': 1, 'status': 'failed', 'code': 'VALIDATOR_ERROR',
                  'error_type': type(error).__name__, 'raw_error': 'omitted'}
    if report['status'] != 'passed':
        report.update({'forward_version_upgrade': 'not_verified', 'schema_migration': 'not_verified',
                       'old_expected_binary_sha256': args.old_binary_sha256,
                       'new_expected_binary_sha256': args.new_binary_sha256,
                       'old_declared_source_revision': args.old_source_revision,
                       'new_declared_source_revision': args.new_source_revision})
    print(json.dumps(report, indent=2))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
