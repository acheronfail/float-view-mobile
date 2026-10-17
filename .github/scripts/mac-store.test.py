"""Mac store profile, source and package boundaries without Apple credentials."""
import argparse
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    'mac_store', Path(__file__).resolve().parents[2] / 'backends/apple/mac-store.py')
store = importlib.util.module_from_spec(spec)
spec.loader.exec_module(store)
TEAM = 'TESTTEAM01'
CERT = b'test-distribution-certificate'
IDENTITY = hashlib.sha1(CERT).hexdigest().upper()


def profile():
    return {'TeamIdentifier': [TEAM], 'Platform': ['OSX'],
            'ExpirationDate': datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None) + datetime.timedelta(days=2),
            'DeveloperCertificates': [CERT], 'Entitlements': {
                'com.apple.application-identifier': TEAM + '.' + store.STORE_ID,
                'com.apple.developer.team-identifier': TEAM,
                'keychain-access-groups': [TEAM + '.*']}}


class MacStoreTests(unittest.TestCase):
    def test_profile_keeps_only_required_capabilities(self):
        expected = store.validate_profile(profile(), TEAM, IDENTITY)
        self.assertTrue(expected['com.apple.security.app-sandbox'])
        self.assertNotIn('keychain-access-groups', expected)
        self.assertNotIn('get-task-allow', expected)

    def test_rejects_incorrect_platform_team_and_distribution(self):
        for changes in [
            {'TeamIdentifier': ['OTHERTEAM1']}, {'Platform': ['iOS']},
            {'ExpirationDate': datetime.datetime.min}, {'ProvisionedDevices': ['test-device']},
            {'ProvisionsAllDevices': True}, {'IsXcodeManaged': True},
            {'DeveloperCertificates': [b'other']}, {'DeveloperCertificates': []},
        ]:
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                store.validate_profile(profile() | changes, TEAM, IDENTITY)

    def test_rejects_wrong_app_and_debug_permissions(self):
        for key, value in [('com.apple.application-identifier', TEAM + '.wrong.app'),
                           ('com.apple.developer.team-identifier', 'OTHERTEAM1'),
                           ('get-task-allow', True), ('com.apple.security.get-task-allow', True)]:
            item = profile()
            item['Entitlements'][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                store.validate_profile(item, TEAM, IDENTITY)

    def test_rejects_malformed_signer_inputs(self):
        for team, identity in [('', IDENTITY), (TEAM, '-'), (TEAM, 'invalid')]:
            with self.subTest(team=team, identity=identity), self.assertRaises(ValueError):
                store.validate_profile(profile(), team, identity)

    def test_nightly_never_reaches_build_or_signing(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, {
            'FLOAT_VIEW_VERSION': 'v1.0.0-nightly.1.gabcdef'}, clear=True), patch.object(store, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'never nightly'):
                store.package(argparse.Namespace(command='package', output=Path(directory)/'output', profile=None))
            run.assert_not_called()

    def test_dirty_signed_source_rejected_before_credentials(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, {
            'FLOAT_VIEW_VERSION': 'v1.0.0-rc.1'}, clear=True), patch.object(store, 'run', side_effect=[
                '1.0.0 101', 'a'*40, ' M source.swift']) as run:
            with self.assertRaisesRegex(ValueError, 'Commit source changes'):
                store.package(argparse.Namespace(command='package', output=Path(directory)/'output', profile=None))
            self.assertEqual(run.call_count, 3)

    def test_ci_master_rejected_before_credentials(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, {
            'FLOAT_VIEW_VERSION': 'v1.0.0-rc.1', 'GITHUB_ACTIONS': 'true',
            'GITHUB_REF': 'refs/heads/master', 'GITHUB_EVENT_NAME': 'push'}, clear=True), patch.object(store, 'run', side_effect=[
                '1.0.0 101', 'a'*40, '']) as run:
            with self.assertRaisesRegex(ValueError, 'explicit stable/candidate tag'):
                store.package(argparse.Namespace(command='package', output=Path(directory)/'output', profile=None))
            self.assertEqual(run.call_count, 3)

    def test_legal_text_must_be_complete_and_current(self):
        with tempfile.TemporaryDirectory() as directory:
            app = Path(directory)
            resources = app/'Contents/Resources'; resources.mkdir(parents=True)
            html = resources/'index.html'
            html.write_text('<script>' + ';'.join(json.dumps((store.ROOT/name).read_text()) for name in
                            ['LICENSE', 'NOTICE', 'THIRD_PARTY_NOTICES.txt']) + '</script>')
            store.verify_notices(app)
            html.write_text('<p>AGPL-3.0-or-later</p>')
            with self.assertRaisesRegex(ValueError, 'Missing or stale bundled legal text'):
                store.verify_notices(app)

    def test_workflow_keeps_mac_signing_protected_and_publication_independent(self):
        text = (store.ROOT/'.github/workflows/build-apps.yml').read_text()
        job = text.split('\n  mac-store:\n', 1)[1].split('\n  ios-upload:', 1)[0]
        self.assertIn("needs.prepare.outputs.mac-store-build == 'true'", job)
        self.assertIn("vars.MACOS_STORE_SIGNING_ENABLED == 'true'", job)
        self.assertIn('environment: apple-store-signing', job)
        self.assertIn('needs.prepare.outputs.build-number', job)
        self.assertNotIn('--upload-app', job)
        ios = text.split('\n  ios-store:\n', 1)[1].split('\n  mac-store:', 1)[0]
        for signing_job in [ios, job]:
            self.assertIn('environment: apple-store-signing', signing_job)
            self.assertIn('secrets.APPLE_DISTRIBUTION_P12_BASE64', signing_job)
            self.assertIn('secrets.APPLE_DISTRIBUTION_P12_PASSWORD', signing_job)
            self.assertNotIn('secrets.IOS_CODE_SIGN_P12', signing_job)
            self.assertNotIn('secrets.MAC_APP_SIGN_P12', signing_job)

        release = text.split('\n  release:\n', 1)[1]
        self.assertIn('needs: [prepare, ipa, apk, macos]', release)



if __name__ == '__main__':
    unittest.main()
