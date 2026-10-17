#!/usr/bin/env python3
"""Protected Mac TestFlight upload; no tester invitations, review or release."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
APP_ID = 'fail.acheron.floatview.macos'


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


shared = load_module('apple_upload', Path(__file__).with_name('ios-upload.py'))
store = load_module('mac_store', Path(__file__).with_name('mac-store.py'))
require = shared.require


def candidate(directory):
    record = json.loads((directory / 'package.json').read_text())
    ci = record.get('ci') or {}
    require(record.get('schema') == 1 and record.get('purpose') == 'store'
            and record.get('configuration') == 'Release' and record.get('dirty') is False
            and record.get('team') == os.environ.get('APPLE_TEAM_ID')
            and record.get('bundleIdentifier') == APP_ID, 'Expected a clean verified Mac Release store package')
    version = record.get('version', '')
    require(ci.get('commit') == record.get('commit') == os.environ.get('GITHUB_SHA')
            and ci.get('runId') == os.environ.get('GITHUB_RUN_ID')
            and ci.get('workflowPath') == '.github/workflows/build-apps.yml'
            and ci.get('buildJob') == 'mac-store'
            and ci.get('appVersion') == version
            and ci.get('buildNumber') == record.get('buildNumber') == os.environ.get('FLOAT_VIEW_BUILD_NUMBER')
            and ci.get('repository') == os.environ.get('GITHUB_REPOSITORY')
            and record.get('sourceVersion') == ci.get('tag')
            and ((ci.get('channel') == 'stable' and ci.get('tag') == 'v' + version)
                 or (ci.get('channel') == 'candidate' and re.fullmatch('v' + re.escape(version) + r'-rc\.[1-9]\d*', ci.get('tag', ''))))
            and os.environ.get('GITHUB_REF') == 'refs/tags/' + ci.get('tag', ''),
            'Mac package differs from the approved CI source/build/run')
    name = record.get('package', '')
    require(name == Path(name).name and name.endswith('.pkg'), 'Invalid Mac package filename')
    pkg = directory / name
    require(pkg.is_file() and not pkg.is_symlink()
            and hashlib.sha256(pkg.read_bytes()).hexdigest() == record.get('packageSHA256'),
            'Mac package changed since signing')
    return record, pkg


def verify_package(pkg, record):
    signature = store.run('pkgutil', '--check-signature', pkg, capture=True)
    team = record['team']
    require('Status: signed by a developer certificate issued by Apple' in signature
            and re.search(r'3rd Party Mac Developer Installer: .+ \(' + re.escape(team) + r'\)', signature),
            'Installer must have an Apple Mac Installer Distribution signer for this team')
    with tempfile.TemporaryDirectory(prefix='float-view-mac-upload-check-') as directory:
        expanded = Path(directory) / 'expanded'
        store.run('pkgutil', '--expand-full', pkg, expanded)
        apps = list(expanded.rglob('*.app'))
        require(len(apps) == 1 and not apps[0].is_symlink(), 'Expected exactly one Mac app payload')
        app = apps[0]
        profile = plistlib.loads(store.run('security', 'cms', '-D', '-i', app / 'Contents/embedded.provisionprofile', capture=True).encode())
        prefix = str(Path(directory) / 'app-cert-')
        store.run('codesign', '-d', '--extract-certificates=' + prefix, app)
        leaf = Path(prefix + '0').read_bytes()
        expected = store.validate_profile(profile, team, hashlib.sha1(leaf).hexdigest().upper())
        checks = store.inspect_app(app, record['version'], record['buildNumber'], ['arm64', 'x86_64'], APP_ID, expected)
        require(checks['executableSHA256'] == record['checks']['executableSHA256'], 'Executable differs from signing record')
        store.verify_notices(app)
        store.run('codesign', '--verify', '--strict', '-R',
                  '=anchor apple generic and certificate leaf[field.1.2.840.113635.100.6.1.4] exists'
                  ' and certificate leaf[subject.OU] = "' + team + '"', app)


class MacAppleAPI:
    def __init__(self, api, record):
        self.api, self.record = api, record

    def get(self, path, query=None):
        return self.api.get(path, query)

    def upload(self, pkg):
        shared.apple_tool('--upload-package', str(pkg), '--platform', 'macos',
                          '--apple-id', os.environ['ASC_APP_ID'],
                          '--bundle-version', self.record['buildNumber'],
                          '--bundle-short-version-string', self.record['version'],
                          '--bundle-id', APP_ID, *self.api.auth, '--output-format', 'json',
                          env=self.api.env, timeout=1200)


def transfer(api, record, pkg, output, prior, **options):
    return shared.transfer(api, record, pkg, output, prior, bundle_id=APP_ID,
                           platform='MAC_OS', hash_field='packageSHA256', **options)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--prior-receipts', type=Path, required=True)
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    try:
        require(os.environ.get('GITHUB_ACTIONS') == 'true' and os.environ.get('RUNNER_ENVIRONMENT') == 'github-hosted'
                and os.environ.get('RUNNER_OS') == 'macOS'
                and re.fullmatch(r'refs/tags/v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-rc\.[1-9]\d*)?', os.environ.get('GITHUB_REF', ''))
                and os.environ.get('GITHUB_EVENT_NAME') in ['push', 'workflow_dispatch'],
                'Mac delivery requires an approved hosted stable/candidate tag; never master or nightly')
        require(args.check_only or os.environ.get('MACOS_STORE_UPLOAD_READY') == 'true', 'Mac upload readiness is not enabled')
        record, pkg = candidate(args.package)
        with shared.credentials() as api:
            verify_package(pkg, record)
            transfer(MacAppleAPI(api, record), record, pkg, args.output, args.prior_receipts, check_only=args.check_only)
        print('Read-only Mac Apple app/build checks passed; no upload' if args.check_only else
              'Apple processed the Mac build; no testers invited or review submitted')
    except ValueError as error:
        parser.exit(1, str(error) + '\n')
    except (KeyError, OSError, subprocess.SubprocessError):
        parser.exit(1, 'Mac delivery failed; inspect the retained receipt and Apple status before retrying.\n')


if __name__ == '__main__':
    main()
