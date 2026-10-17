"""Mac upload boundaries and shared retry logic without Apple credentials/network."""
import argparse
import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('mac_upload', ROOT/'backends/apple/mac-upload.py')
upload = importlib.util.module_from_spec(spec); spec.loader.exec_module(upload)
spec = importlib.util.spec_from_file_location('ios_upload_tests', ROOT/'.github/scripts/ios-upload.test.py')
ios_tests = importlib.util.module_from_spec(spec); spec.loader.exec_module(ios_tests)


class FakeApple(ios_tests.FakeApple):
    def __init__(self, *args, platform='MAC_OS', **kwargs):
        super().__init__(*args, bundle=upload.APP_ID, **kwargs)
        self.platform = platform

    def get(self, path, query=None):
        result = super().get(path, query)
        for item in result.get('included', []):
            item['attributes']['platform'] = self.platform
        return result


class MacUploadTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.pkg = self.root/'Float View.pkg'; self.pkg.write_bytes(b'signed fixture')
        self.output = self.root/'output'; self.prior = self.root/'prior'; self.prior.mkdir()
        self.record = {'schema':1,'purpose':'store','configuration':'Release','dirty':False,'team':'TESTTEAM01',
                       'bundleIdentifier':upload.APP_ID,'commit':'a'*40,'version':'1.0.0','sourceVersion':'v1.0.0-rc.1',
                       'buildNumber':'75','package':self.pkg.name,'packageSHA256':hashlib.sha256(self.pkg.read_bytes()).hexdigest(),
                       'ci':{'commit':'a'*40,'appVersion':'1.0.0','buildNumber':'75','runId':'123','repository':'owner/repo',
                             'buildJob':'mac-store','workflowPath':'.github/workflows/build-apps.yml','channel':'candidate','tag':'v1.0.0-rc.1'}}
        self.env = patch.dict(os.environ,{'ASC_APP_ID':'123456','GITHUB_RUN_ATTEMPT':'1','GITHUB_SHA':'a'*40,
                             'GITHUB_RUN_ID':'123','GITHUB_REPOSITORY':'owner/repo','FLOAT_VIEW_BUILD_NUMBER':'75',
                             'APPLE_TEAM_ID':'TESTTEAM01','GITHUB_REF':'refs/tags/v1.0.0-rc.1'},clear=True)
        self.env.start(); self.addCleanup(self.env.stop)

    def transfer(self, api, **options):
        return upload.transfer(api,self.record,self.pkg,self.output,self.prior,sleep=lambda _:None,polls=2,**options)

    def receipt(self):
        return json.loads((self.output/'upload-receipt.json').read_text())

    def test_mac_upload_uses_mac_platform_pkg_hash_and_one_transport(self):
        api=FakeApple([None,'PROCESSING','VALID']); r=self.transfer(api)
        self.assertEqual(r['status'],'processed'); self.assertEqual(api.uploads,[self.pkg])
        self.assertEqual(api.queries[1][1]['filter[preReleaseVersion.platform]'],'MAC_OS')
        self.assertEqual(r['packageSHA256'],self.record['packageSHA256']); self.assertNotIn('ipaSHA256',r)

    def test_check_only_never_uploads_including_invalid_consumed_build(self):
        for state in [None,'VALID','INVALID']:
            api=FakeApple([state]); self.assertEqual(self.transfer(api,check_only=True)['status'],'checkedOnly')
            self.assertEqual(api.uploads,[])

    def test_other_app_or_platform_cannot_consume_mac_number(self):
        api=FakeApple(['VALID'],platform='IOS')
        with self.assertRaisesRegex(ValueError,'platform'): self.transfer(api)
        self.assertEqual(api.uploads,[])
        api=FakeApple([None]); api.bundle='wrong.app'
        with self.assertRaisesRegex(ValueError,'bundle ID'): self.transfer(api)
        self.assertEqual(api.uploads,[])

    def test_consumed_number_requires_exact_prior_package_receipt(self):
        api=FakeApple(['VALID'])
        with self.assertRaisesRegex(ValueError,'matching prior'): self.transfer(api)
        receipt={k:self.record[k] for k in ['commit','version','buildNumber','packageSHA256']}
        receipt.update(appId='123456',runId='123',transportAccepted=True)
        (self.prior/'upload-receipt.json').write_text(json.dumps(receipt)); os.environ['GITHUB_RUN_ATTEMPT']='2'
        api=FakeApple(['VALID']); self.assertEqual(self.transfer(api)['status'],'processed'); self.assertEqual(api.uploads,[])
        receipt['packageSHA256']='changed'; (self.prior/'upload-receipt.json').write_text(json.dumps(receipt))
        with self.assertRaisesRegex(ValueError,'matching prior'): self.transfer(FakeApple(['VALID']))

    def test_unknown_retry_and_uncertain_transport_never_reupload(self):
        os.environ['GITHUB_RUN_ATTEMPT']='2'; api=FakeApple([None])
        with self.assertRaisesRegex(ValueError,'earlier attempt'): self.transfer(api)
        self.assertEqual(api.uploads,[]); os.environ['GITHUB_RUN_ATTEMPT']='1'; api=FakeApple([None],fail=True)
        with self.assertRaisesRegex(ValueError,'uncertain'): self.transfer(api)
        self.assertEqual(len(api.uploads),1); self.assertEqual(self.receipt()['status'],'uploadStarted')

    def test_invalid_or_pending_processing_retains_transport_receipt(self):
        for state in ['INVALID','FAILED',None]:
            with self.subTest(state=state), self.assertRaises(ValueError): self.transfer(FakeApple([None,state]))
            self.assertTrue(self.receipt()['transportAccepted'])

    def test_candidate_checks_source_job_hash_and_package_path(self):
        path=self.root/'package.json'; path.write_text(json.dumps(self.record)); self.assertEqual(upload.candidate(self.root)[1],self.pkg)
        for field,value in [('dirty',True),('configuration','Debug'),('bundleIdentifier','other.app'),('team','OTHERTEAM1'),
                            ('commit','b'*40),('sourceVersion','v1.0.0'),('package','../other.pkg'),('packageSHA256','changed')]:
            record=copy.deepcopy(self.record); record[field]=value; path.write_text(json.dumps(record))
            with self.subTest(field=field),self.assertRaises(ValueError): upload.candidate(self.root)
        for field,value in [('buildJob','ios-store'),('runId','other'),('channel','nightly'),('tag','v1.0.0-rc.0')]:
            record=copy.deepcopy(self.record); record['ci'][field]=value; path.write_text(json.dumps(record))
            with self.subTest(field=field),self.assertRaises(ValueError): upload.candidate(self.root)

    def test_host_and_readiness_fail_before_credential_use(self):
        args=['mac-upload.py',str(self.root),str(self.output),'--prior-receipts',str(self.prior)]
        hosted={'GITHUB_ACTIONS':'true','RUNNER_ENVIRONMENT':'github-hosted','RUNNER_OS':'macOS','GITHUB_EVENT_NAME':'push'}
        for overrides in [{},hosted,{**hosted,'MACOS_STORE_UPLOAD_READY':'true','GITHUB_REF':'refs/heads/master'}]:
            with patch.dict(os.environ,overrides),patch('sys.argv',args),patch.object(upload.shared,'credentials') as credentials:
                with contextlib.redirect_stderr(io.StringIO()),self.assertRaises(SystemExit): upload.main()
                credentials.assert_not_called()

    def test_hosted_read_only_check_works_with_upload_readiness_off(self):
        (self.root/'package.json').write_text(json.dumps(self.record))
        args=['mac-upload.py',str(self.root),str(self.output),'--prior-receipts',str(self.prior),'--check-only']
        hosted={'GITHUB_ACTIONS':'true','RUNNER_ENVIRONMENT':'github-hosted','RUNNER_OS':'macOS',
                'GITHUB_EVENT_NAME':'workflow_dispatch','MACOS_STORE_UPLOAD_READY':'false'}
        api=FakeApple([None])
        @contextlib.contextmanager
        def credentials():
            yield api
        with patch.dict(os.environ,hosted),patch('sys.argv',args),patch.object(upload.shared,'credentials',credentials), \
             patch.object(upload,'verify_package') as verify,contextlib.redirect_stdout(io.StringIO()):
            upload.main()
        verify.assert_called_once_with(self.pkg,self.record)
        self.assertEqual(api.uploads,[])
        self.assertEqual(self.receipt()['status'],'checkedOnly')

    def test_altool_transport_uses_documented_mac_package_metadata(self):
        class API:
            auth=['--api-key','TESTKEY123']; env={}
        api=upload.MacAppleAPI(API(),self.record)
        with patch.object(upload.shared,'apple_tool') as tool:
            api.upload(self.pkg)
            args=tool.call_args.args
            self.assertEqual(args[:2],('--upload-package',str(self.pkg)))
            self.assertEqual(args[args.index('--platform')+1],'macos')
            self.assertEqual(args[args.index('--apple-id')+1],'123456')
            self.assertEqual(args[args.index('--bundle-version')+1],'75')
            self.assertEqual(args[args.index('--bundle-id')+1],upload.APP_ID)


if __name__=='__main__': unittest.main()
