"""Play delivery boundary and interrupted-edit tests; no Google credentials/network."""
import base64
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
import urllib.parse
import zipfile
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('upload', Path(__file__).with_name('android-upload.py'))
upload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upload)


class FakePlay:
    def __init__(self, record, present=False, completed=False, newer=False, conflict=False, fail=None):
        self.record = record
        self.present, self.completed, self.newer, self.conflict = present, completed, newer, conflict
        self.fail = fail
        self.calls = []
        self.uploads = 0
        self.edits = {}
        self.counter = 0

    def bundle(self):
        return {'versionCode': self.record['buildNumber'], 'sha256': 'wrong' if self.conflict else self.record['bundleSha256']}

    def call(self, method, suffix='', body=None):
        self.calls.append((method, suffix, body))
        if self.fail == 'create' and method == 'POST' and not suffix:
            raise upload.GoogleError(503)
        if method == 'POST' and not suffix:
            self.counter += 1
            edit = 'edit' + str(self.counter)
            self.edits.clear()  # Creating an edit invalidates the previous edit for this account.
            self.edits[edit] = {'present': self.present, 'completed': self.completed}
            return {'id': edit}
        edit = suffix.split('/')[1].split(':')[0]
        if edit not in self.edits:
            raise upload.GoogleError(404)
        state = self.edits[edit]
        if method == 'GET' and suffix == '/' + edit:
            return {'id': edit}
        if suffix.endswith('/bundles'):
            return {'bundles': [self.bundle()] if state['present'] else []}
        if method == 'GET' and suffix.endswith('/tracks/internal'):
            codes = [str(self.record['buildNumber'] + 1)] if self.newer else [str(self.record['buildNumber'])]
            return {'track': 'internal', 'releases': [{'versionCodes': codes, 'status': 'completed'}]
                    if state['completed'] or self.newer else []}
        if method == 'PUT':
            assert suffix.endswith('/tracks/internal') and body['track'] == 'internal'
            assert body['releases'][0]['status'] == 'completed'
            state['completed'] = True
        elif ':commit?' in suffix:
            assert suffix.endswith('changesInReviewBehavior=ERROR_IF_IN_REVIEW')
            if self.fail == 'review':
                raise ValueError('Google API HTTP 400')
            self.present, self.completed = state['present'], state['completed']
            self.edits.clear()
            if self.fail == 'commit':
                raise ValueError('commit outcome uncertain')
        elif method == 'DELETE':
            del self.edits[edit]
        else:
            raise AssertionError((method, suffix))
        return {}

    def upload(self, edit, bundle):
        self.uploads += 1
        self.edits[edit]['present'] = True
        if self.fail == 'upload':
            raise ValueError('upload outcome uncertain')
        return self.bundle()


class UploadTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bundle = self.root/'FloatView-v1.1.0-rc.5.aab'
        self.bundle.write_bytes(b'signed bundle fixture')
        self.record = {'schema': 1, 'configuration': 'Release', 'packageId': upload.PACKAGE, 'uploaded': False,
                       'version': 'v1.1.0-rc.5', 'buildNumber': 123, 'attempt': '1', 'commit': 'a'*40,
                       'runURL': 'https://github.com/owner/repo/actions/runs/456',
                       'bundleSha256': hashlib.sha256(self.bundle.read_bytes()).hexdigest(),
                       'uploadCertificateSha256': 'b'*64}
        self.output, self.prior = self.root/'receipt', self.root/'prior'
        self.env = patch.dict(os.environ, {'GITHUB_RUN_ATTEMPT': '1', 'PATH': os.environ.get('PATH', '')}, clear=True)
        self.env.start()
        self.addCleanup(self.env.stop)

    def deliver(self, api, **kwargs):
        return upload.deliver(api, self.record, self.bundle, self.output, self.prior, **kwargs)

    def receipt(self):
        return json.loads((self.output/'upload-receipt.json').read_text())

    def retry(self):
        attempt = int(os.environ['GITHUB_RUN_ATTEMPT'])
        directory = self.prior/str(attempt)
        directory.mkdir(parents=True, exist_ok=True)
        (directory/'upload-receipt.json').write_text(json.dumps(self.receipt()))
        os.environ['GITHUB_RUN_ATTEMPT'] = str(attempt + 1)

    def test_repeated_edit_creation_failures_allow_first_upload_on_rerun(self):
        api = FakePlay(self.record, fail='create')
        for _ in range(2):
            with self.assertRaisesRegex(upload.GoogleError, 'HTTP 503'):
                self.deliver(api)
            self.assertIs(self.receipt()['uploadMayHaveStarted'], False)
            self.assertEqual(api.uploads, 0)
            self.retry()
        api.fail = None
        self.assertEqual(self.deliver(api)['status'], 'delivered')
        self.assertEqual(api.uploads, 1)
        self.assertIs(self.receipt()['uploadMayHaveStarted'], True)

    def test_failure_after_edit_creation_before_upload_allows_retry(self):
        api = FakePlay(self.record)
        with patch.object(api, 'call', side_effect=[{'id': 'lost-edit'}, upload.GoogleError(503)]):
            with self.assertRaises(upload.GoogleError):
                self.deliver(api)
        self.assertIs(self.receipt()['uploadMayHaveStarted'], False)
        self.retry()
        self.assertEqual(self.deliver(api)['status'], 'delivered')
        self.assertEqual(api.uploads, 1)

    def test_upload_uncertainty_survives_later_creation_failure_and_check(self):
        api = FakePlay(self.record, fail='upload')
        with self.assertRaisesRegex(ValueError, 'uncertain'):
            self.deliver(api)
        self.assertIs(self.receipt()['uploadMayHaveStarted'], True)
        self.retry()
        api.edits.clear()  # The uploaded bundle is no longer observable.
        api.fail = 'create'
        with self.assertRaisesRegex(upload.GoogleError, 'HTTP 503'):
            self.deliver(api)
        self.assertIs(self.receipt()['uploadMayHaveStarted'], True)
        self.retry()
        api.fail = None
        self.assertEqual(self.deliver(api, check_only=True)['status'], 'checkedOnly')
        self.assertIs(self.receipt()['uploadMayHaveStarted'], True)
        self.retry()
        with self.assertRaisesRegex(ValueError, 'not observable'):
            self.deliver(api)
        self.assertEqual(api.uploads, 1)

    def test_incomplete_legacy_or_mismatched_history_never_authorizes_upload(self):
        for history in ('missing', 'legacy', 'mismatched'):
            with self.subTest(history=history):
                self.prior = self.root/history
                os.environ['GITHUB_RUN_ATTEMPT'] = '1'
                api = FakePlay(self.record, fail='create')
                with self.assertRaises(upload.GoogleError):
                    self.deliver(api)
                old = self.receipt()
                if history == 'legacy':
                    del old['uploadMayHaveStarted']
                elif history == 'mismatched':
                    old['bundleSha256'] = 'different'
                self.prior.mkdir()
                (self.prior/'upload-receipt.json').write_text(json.dumps(old))
                os.environ['GITHUB_RUN_ATTEMPT'] = '3' if history == 'missing' else '2'
                api.fail = None
                with self.assertRaisesRegex(ValueError, 'not observable|No matching earlier receipt'):
                    self.deliver(api)
                self.assertEqual(api.uploads, 0)

    def test_new_delivery_uploads_once_verifies_hash_and_committed_internal_track(self):
        api = FakePlay(self.record)
        self.assertEqual(self.deliver(api)['status'], 'delivered')
        self.assertEqual(api.uploads, 1)
        self.assertEqual(len([c for c in api.calls if ':commit?' in c[1]]), 1)
        self.assertEqual(self.receipt()['bundleSha256'], self.record['bundleSha256'])

    def test_check_only_no_upload_track_write_or_commit(self):
        api = FakePlay(self.record)
        self.assertEqual(self.deliver(api, check_only=True)['status'], 'checkedOnly')
        self.assertEqual(api.uploads, 0)
        self.assertFalse(any(method == 'PUT' or ':commit?' in path for method, path, _ in api.calls))

    def test_check_only_before_upload_allows_later_delivery(self):
        api = FakePlay(self.record)
        self.assertEqual(self.deliver(api, check_only=True)['status'], 'checkedOnly')
        self.assertIs(self.receipt()['uploadMayHaveStarted'], False)
        self.retry()
        real_upload = api.upload
        def send(edit, bundle):
            self.assertIs(self.receipt()['uploadMayHaveStarted'], True)
            self.assertEqual(self.receipt()['status'], 'uploadStarted')
            return real_upload(edit, bundle)
        with patch.object(api, 'upload', side_effect=send):
            self.assertEqual(self.deliver(api)['status'], 'delivered')
        self.assertEqual(api.uploads, 1)

    def test_already_delivered_identical_hash_is_idempotent(self):
        api = FakePlay(self.record, present=True, completed=True)
        self.assertEqual(self.deliver(api)['status'], 'alreadyDelivered')
        self.assertEqual(api.uploads, 0)
        self.assertFalse(any(c[0] == 'PUT' for c in api.calls))

    def test_conflicting_hash_and_newer_build_stop_before_mutation(self):
        for options, message in [({'present': True, 'conflict': True}, 'different bundle'), ({'newer': True}, 'newer internal')]:
            api = FakePlay(self.record, **options)
            with self.assertRaisesRegex(ValueError, message):
                self.deliver(api)
            self.assertEqual(api.uploads, 0)
            self.assertFalse(any(c[0] == 'PUT' for c in api.calls))

    def test_timeout_after_upload_resumes_saved_edit_without_second_upload(self):
        api = FakePlay(self.record, fail='upload')
        with self.assertRaisesRegex(ValueError, 'uncertain'):
            self.deliver(api)
        self.assertEqual(self.receipt()['status'], 'uploadStarted')
        self.retry()
        api.fail = None
        self.assertEqual(self.deliver(api)['status'], 'delivered')
        self.assertEqual(api.uploads, 1)

    def test_timeout_after_commit_recognizes_committed_hash_and_track(self):
        api = FakePlay(self.record, fail='commit')
        with self.assertRaisesRegex(ValueError, 'uncertain'):
            self.deliver(api)
        self.retry()
        self.assertEqual(self.deliver(api)['status'], 'alreadyDelivered')
        self.assertEqual(api.uploads, 1)

    def test_unobservable_earlier_upload_and_missing_receipts_never_reupload(self):
        os.environ['GITHUB_RUN_ATTEMPT'] = '2'
        api = FakePlay(self.record)
        with self.assertRaisesRegex(ValueError, 'No matching earlier receipt'):
            self.deliver(api)
        self.assertEqual(api.uploads, 0)
        os.environ['GITHUB_RUN_ATTEMPT'] = '1'
        api.fail = 'upload'
        with self.assertRaises(ValueError):
            self.deliver(api)
        self.retry()
        for state in api.edits.values():
            state['present'] = False
        with self.assertRaisesRegex(ValueError, 'not observable'):
            self.deliver(api)
        self.assertEqual(api.uploads, 1)

    def test_existing_review_error_never_changes_track_or_commit_options(self):
        api = FakePlay(self.record, fail='review')
        with self.assertRaisesRegex(ValueError, 'HTTP 400'):
            self.deliver(api)
        self.assertEqual(self.receipt()['status'], 'commitStarted')
        self.assertEqual(len([c for c in api.calls if ':commit?' in c[1]]), 1)

    def test_check_of_pending_edit_preserves_it_without_new_edit_or_writes(self):
        api = FakePlay(self.record, fail='upload')
        with self.assertRaises(ValueError):
            self.deliver(api)
        self.retry()
        before = api.counter
        calls = len(api.calls)
        self.assertEqual(self.deliver(api, check_only=True)['status'], 'checkedOnly')
        self.assertEqual(api.counter, before)
        self.assertEqual(api.uploads, 1)
        self.assertTrue(self.receipt().get('editId'))
        self.assertFalse(any(method in ('PUT', 'POST', 'DELETE') for method, _, _ in api.calls[calls:]))

    def test_prior_edit_permission_error_does_not_create_or_invalidate_an_edit(self):
        api = FakePlay(self.record, fail='upload')
        with self.assertRaises(ValueError):
            self.deliver(api)
        self.retry()
        before = api.counter
        real_call = api.call
        def call(method, suffix='', body=None):
            if method == 'GET' and suffix == '/edit1':
                raise upload.GoogleError(403)
            return real_call(method, suffix, body)
        api.call = call
        with self.assertRaisesRegex(ValueError, 'HTTP 403'):
            self.deliver(api)
        self.assertEqual(api.counter, before)
        self.assertEqual(self.receipt()['editId'], 'edit1')

    def test_google_upload_response_requires_matching_code_and_hash(self):
        api = FakePlay(self.record, conflict=True)
        with self.assertRaisesRegex(ValueError, 'hash/version mismatch'):
            self.deliver(api)
        self.assertFalse(any(c[0] == 'PUT' for c in api.calls))

    def test_candidate_rejects_metadata_and_bytes_before_signature_commands(self):
        with patch.dict(os.environ, {'ANDROID_UPLOAD_CERT_SHA256': 'b'*64}), patch.object(upload, 'command') as command:
            with patch('subprocess.run') as run:
                run.return_value.returncode = 0
                # identity() calls git twice, matching the source/tag and approved SHA.
                run.return_value.stdout = 'a'*40 + '\n'
                env = {'FLOAT_VIEW_VERSION': self.record['version'], 'GITHUB_REF': 'refs/tags/' + self.record['version'],
                       'GITHUB_REPOSITORY': 'owner/repo', 'GITHUB_WORKFLOW_REF': 'owner/repo/.github/workflows/build-apps.yml@refs/tags/' + self.record['version'],
                       'GITHUB_RUN_NUMBER': '63', 'GITHUB_RUN_ID': '456', 'GITHUB_SHA': 'a'*40,
                       'FLOAT_VIEW_BUILD_NUMBER': '123'}
                with patch.dict(os.environ, env):
                    for key, value in [('commit', 'c'*40), ('packageId', 'wrong'), ('configuration', 'Debug'),
                                       ('buildNumber', 124), ('bundleSha256', 'bad'), ('attempt', '2'), ('runURL', 'other')]:
                        record = copy.deepcopy(self.record)
                        record[key] = value
                        (self.root/'package.json').write_text(json.dumps(record))
                        with self.subTest(key=key), self.assertRaises(ValueError):
                            upload.candidate(self.root)
                    command.assert_not_called()

    def test_missing_credentials_and_readiness_fail_before_google_calls(self):
        with self.assertRaisesRegex(ValueError, 'Missing/oversized'):
            upload.authenticate()
        with patch('sys.argv', ['android-upload.py', str(self.root), str(self.output), '--prior-receipts', str(self.prior)]), patch.object(upload, 'request') as request:
            with self.assertRaisesRegex(ValueError, 'readiness'):
                upload.main()
            request.assert_not_called()

    def test_real_rsa_service_account_assertion_has_bounded_scope_and_valid_signature(self):
        private, public = self.root/'key.pem', self.root/'public.pem'
        upload.command('openssl', 'genpkey', '-algorithm', 'RSA', '-pkeyopt', 'rsa_keygen_bits:2048', '-out', str(private))
        upload.command('openssl', 'pkey', '-in', str(private), '-pubout', '-out', str(public))
        credential = {'type': 'service_account', 'client_email': 'test@example.iam.gserviceaccount.com',
                      'token_uri': upload.TOKEN_URL, 'private_key': private.read_text()}
        def exchange(method, url, data, **kwargs):
            self.assertEqual((method, url), ('POST', upload.TOKEN_URL))
            assertion = urllib.parse.parse_qs(data.decode())['assertion'][0]
            header, body, signature = assertion.split('.')
            claims = json.loads(base64.urlsafe_b64decode(body + '=='))
            self.assertEqual(claims['scope'], 'https://www.googleapis.com/auth/androidpublisher')
            self.assertEqual(claims['aud'], upload.TOKEN_URL)
            self.assertEqual(claims['exp'] - claims['iat'], 1800)
            sig = self.root/'signature'
            sig.write_bytes(base64.urlsafe_b64decode(signature + '=='))
            upload.command('openssl', 'dgst', '-sha256', '-verify', str(public), '-signature', str(sig),
                           data=(header + '.' + body).encode())
            return {'access_token': 'fixture-token'}
        with patch.dict(os.environ, {'GOOGLE_PLAY_SERVICE_ACCOUNT_JSON': json.dumps(credential)}), patch.object(upload, 'request', side_effect=exchange):
            self.assertEqual(upload.authenticate(), 'fixture-token')
            self.assertNotIn('GOOGLE_PLAY_SERVICE_ACCOUNT_JSON', os.environ)

    def test_real_jar_signature_accepts_original_and_rejects_tampered_bundle(self):
        store = self.root/'fixture.p12'
        upload.command('keytool', '-genkeypair', '-alias', 'fixture', '-keyalg', 'RSA', '-keysize', '2048',
                       '-validity', '365', '-dname', 'CN=Test Fixture', '-keystore', str(store),
                       '-storepass', 'fixture-password', '-keypass', 'fixture-password')
        with zipfile.ZipFile(self.bundle, 'w') as jar:
            jar.writestr('base/assets/index.html', 'original')
        upload.command('jarsigner', '-keystore', str(store), '-storepass', 'fixture-password', str(self.bundle), 'fixture')
        der = upload.command('keytool', '-exportcert', '-keystore', str(store), '-storepass', 'fixture-password', '-alias', 'fixture')
        self.record['uploadCertificateSha256'] = hashlib.sha256(der).hexdigest()
        env = {'FLOAT_VIEW_VERSION': self.record['version'], 'GITHUB_REF': 'refs/tags/' + self.record['version'],
               'GITHUB_REPOSITORY': 'owner/repo', 'GITHUB_WORKFLOW_REF': 'owner/repo/.github/workflows/build-apps.yml@refs/tags/' + self.record['version'],
               'GITHUB_RUN_NUMBER': '63', 'GITHUB_RUN_ID': '456', 'GITHUB_SHA': 'a'*40,
               'FLOAT_VIEW_BUILD_NUMBER': '123', 'ANDROID_UPLOAD_CERT_SHA256': self.record['uploadCertificateSha256']}
        real_run = upload.subprocess.run
        def run(args, **kwargs):
            if args[0] == 'git':
                return upload.subprocess.CompletedProcess(args, 0, stdout='a'*40 + '\n', stderr='')
            return real_run(args, **kwargs)
        with patch.dict(os.environ, env), patch('subprocess.run', side_effect=run):
            self.record['bundleSha256'] = hashlib.sha256(self.bundle.read_bytes()).hexdigest()
            (self.root/'package.json').write_text(json.dumps(self.record))
            self.assertEqual(upload.candidate(self.root)[1], self.bundle)
            # Rehashing changed bytes cannot bypass the independent JAR signature check.
            with zipfile.ZipFile(self.bundle) as archive:
                entries = {name: archive.read(name) for name in archive.namelist()}
            entries['base/assets/index.html'] = b'tampered'
            with zipfile.ZipFile(self.bundle, 'w') as archive:
                for name, data in entries.items():
                    archive.writestr(name, data)
            self.record['bundleSha256'] = hashlib.sha256(self.bundle.read_bytes()).hexdigest()
            (self.root/'package.json').write_text(json.dumps(self.record))
            with self.assertRaises(ValueError):
                upload.candidate(self.root)

    def test_endpoints_cannot_target_production_other_apps_or_other_hosts(self):
        api = upload.PlayAPI('private token')
        for suffix in ['/edit1/tracks/production', '/edit1/tracks/alpha', '/../../other', 'https://other.example']:
            with self.assertRaisesRegex(ValueError, 'endpoint'):
                api.call('PUT', suffix, {})


if __name__ == '__main__':
    unittest.main()
