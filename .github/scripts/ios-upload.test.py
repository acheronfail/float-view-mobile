"""Upload/receipt boundaries with fake Apple responses; no credentials or network."""
import base64
import contextlib
import hashlib
import importlib.util
import json
import io
import os
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("upload", Path(__file__).resolve().parents[2] / "backends/apple/ios-upload.py")
upload = importlib.util.module_from_spec(spec)
spec.loader.exec_module(upload)


class FakeApple:
    def __init__(self, states, *, version="1.0.0", bundle=upload.APP_ID, fail=False):
        self.states, self.version, self.bundle, self.fail = states, version, bundle, fail
        self.uploads, self.queries = [], []

    def get(self, path, query=None):
        self.queries.append((path, query))
        if path.startswith("/v1/apps/"):
            return {"data": {"id": "123456", "attributes": {"bundleId": self.bundle}}}
        state = self.states.pop(0) if len(self.states) > 1 else self.states[0]
        if state is None:
            return {"data": []}
        return {"data": [{"id": "apple-build", "attributes": {"version": "75", "processingState": state},
                          "relationships": {"preReleaseVersion": {"data": {"id": "prerelease"}}}}],
                "included": [{"id": "prerelease", "type": "preReleaseVersions",
                              "attributes": {"version": self.version, "platform": "IOS"}}]}

    def upload(self, ipa):
        self.uploads.append(ipa)
        if self.fail:
            raise ValueError("Transport uncertain")


class UploadTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.ipa = self.root / "Float View.ipa"
        self.ipa.write_bytes(b"synthetic IPA")
        self.record = {"schema": 1, "purpose": "store", "configuration": "Release", "dirty": False,
                       "commit": "a" * 40, "team": "TESTTEAM01", "version": "1.0.0", "buildNumber": "75", "ipa": self.ipa.name,
                       "ipaSHA256": hashlib.sha256(self.ipa.read_bytes()).hexdigest(),
                       "ci": {"commit": "a" * 40, "appVersion": "1.0.0", "buildNumber": "75",
                              "runId": "123", "repository": "owner/repo", "buildJob": "ios-store",
                              "workflowPath": ".github/workflows/build-apps.yml", "channel": "stable", "tag": "v1.0.0"}}
        self.output = self.root / "output"
        self.prior = self.root / "prior"
        self.prior.mkdir()
        self.env = patch.dict(os.environ, {"ASC_APP_ID": "123456", "GITHUB_RUN_ATTEMPT": "1",
                              "GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "123", "GITHUB_REPOSITORY": "owner/repo",
                              "FLOAT_VIEW_BUILD_NUMBER": "75", "APPLE_TEAM_ID": "TESTTEAM01", "GITHUB_REF": "refs/tags/v1.0.0"}, clear=True)
        self.env.start()
        self.addCleanup(self.env.stop)

    def transfer(self, api):
        return upload.transfer(api, self.record, self.ipa, self.output, self.prior, sleep=lambda _: None, polls=2)

    def receipt(self):
        return json.loads((self.output / "upload-receipt.json").read_text())

    def test_first_upload_checks_app_and_consumed_number_then_polls_processing(self):
        api = FakeApple([None, "PROCESSING", "VALID"])
        receipt = self.transfer(api)
        self.assertEqual(receipt["status"], "processed")
        self.assertEqual(receipt["appleBuild"]["id"], "apple-build")
        self.assertEqual(api.uploads, [self.ipa])
        self.assertEqual(api.queries[0][0], "/v1/apps/123456")
        self.assertEqual(api.queries[1][1]["filter[version]"], "75")
        self.assertEqual(api.queries[1][1]["filter[preReleaseVersion.platform]"], "IOS")

    def test_wrong_app_never_uploads(self):
        api = FakeApple([None], bundle="another.app")
        with self.assertRaisesRegex(ValueError, "does not match"):
            self.transfer(api)
        self.assertEqual(api.uploads, [])

    def test_check_only_queries_unused_or_consumed_numbers_without_upload_or_receipt_permission(self):
        for state in [None, "VALID", "INVALID"]:
            api = FakeApple([state])
            receipt = upload.transfer(api, self.record, self.ipa, self.output, self.prior, check_only=True)
            self.assertEqual(receipt["status"], "checkedOnly")
            self.assertEqual(api.uploads, [])

    def test_api_auth_failure_never_attempts_upload(self):
        api = FakeApple([None])
        with patch.object(api, "get", side_effect=ValueError("Apple API returned HTTP 401")):
            with self.assertRaisesRegex(ValueError, "401"):
                self.transfer(api)
        self.assertEqual(api.uploads, [])

    def test_consumed_number_without_source_receipt_is_not_assumed_to_be_our_binary(self):
        api = FakeApple(["VALID"])
        with self.assertRaisesRegex(ValueError, "without a matching"):
            self.transfer(api)
        self.assertEqual(api.uploads, [])
        self.assertEqual(self.receipt()["status"], "alreadyReceived")

    def test_received_matching_prior_submission_is_queried_without_reupload(self):
        prior = {k: self.record[k] for k in ["commit", "version", "buildNumber", "ipaSHA256"]}
        prior.update(appId="123456", runId="123", transportAccepted=True)
        (self.prior / "upload-receipt.json").write_text(json.dumps(prior))
        os.environ["GITHUB_RUN_ATTEMPT"] = "2"
        api = FakeApple(["VALID"])
        self.assertEqual(self.transfer(api)["status"], "processed")
        self.assertEqual(api.uploads, [])

    def test_source_mismatch_in_prior_receipt_cannot_authorize_retry(self):
        (self.prior / "upload-receipt.json").write_text(json.dumps({"transportAccepted": True}))
        with self.assertRaisesRegex(ValueError, "without a matching"):
            self.transfer(FakeApple(["VALID"]))

    def test_unknown_prior_attempt_never_blindly_reuploads(self):
        os.environ["GITHUB_RUN_ATTEMPT"] = "2"
        api = FakeApple([None])
        with self.assertRaisesRegex(ValueError, "earlier attempt"):
            self.transfer(api)
        self.assertEqual(api.uploads, [])

    def test_failed_transport_keeps_attempt_receipt_and_never_retries(self):
        api = FakeApple([None], fail=True)
        with self.assertRaisesRegex(ValueError, "uncertain"):
            self.transfer(api)
        self.assertEqual(len(api.uploads), 1)
        self.assertEqual(self.receipt()["status"], "uploadStarted")
        self.assertNotIn("transportAccepted", self.receipt())

    def test_failed_invalid_or_unseen_processing_keeps_receipt(self):
        for state in ["FAILED", "INVALID", None]:
            with self.subTest(state=state), self.assertRaises(ValueError):
                self.transfer(FakeApple([None, state]))
            self.assertTrue(self.receipt()["transportAccepted"])

    def test_other_marketing_version_cannot_consume_this_run(self):
        api = FakeApple(["VALID"], version="2.0.0")
        with self.assertRaisesRegex(ValueError, "different marketing"):
            self.transfer(api)
        self.assertEqual(api.uploads, [])

    def test_candidate_requires_exact_verified_artifact_and_ci_identity(self):
        path = self.root / "export.json"
        path.write_text(json.dumps(self.record))
        self.assertEqual(upload.candidate(self.root)[1], self.ipa)
        for field, value in [("dirty", True), ("configuration", "Debug"), ("commit", "b" * 40),
                             ("ipa", "../other.ipa"), ("ipaSHA256", "changed")]:
            changed = {**self.record, field: value}
            path.write_text(json.dumps(changed))
            with self.subTest(field=field), self.assertRaises(ValueError):
                upload.candidate(self.root)
        self.record["ci"]["runId"] = "another"
        path.write_text(json.dumps(self.record))
        with self.assertRaises(ValueError):
            upload.candidate(self.root)

    def test_candidate_tag_channel_and_marketing_version_must_agree(self):
        self.record["ci"].update(channel="candidate", tag="v1.0.0-rc.1")
        os.environ["GITHUB_REF"] = "refs/tags/v1.0.0-rc.1"
        path = self.root / "export.json"
        path.write_text(json.dumps(self.record))
        self.assertEqual(upload.candidate(self.root)[1], self.ipa)
        for channel, tag in [("stable", "v1.0.0-rc.1"), ("candidate", "v1.0.0"),
                             ("candidate", "v1.0.1-rc.1"), ("candidate", "v1.0.0-rc.0")]:
            self.record["ci"].update(channel=channel, tag=tag)
            os.environ["GITHUB_REF"] = "refs/tags/" + tag
            path.write_text(json.dumps(self.record))
            with self.subTest(channel=channel, tag=tag), self.assertRaisesRegex(ValueError, "CI source"):
                upload.candidate(self.root)

    def test_candidate_main_preserves_upload_gate_and_uses_same_receipts(self):
        self.record["ci"].update(channel="candidate", tag="v1.0.0-rc.1")
        (self.root / "export.json").write_text(json.dumps(self.record))
        hosted = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted", "RUNNER_OS": "macOS",
                  "GITHUB_REF": "refs/tags/v1.0.0-rc.1", "GITHUB_EVENT_NAME": "push"}
        args = ["ios-upload.py", str(self.root), str(self.output), "--prior-receipts", str(self.prior)]
        with patch.dict(os.environ, hosted), patch("sys.argv", args), patch.object(upload, "credentials") as key:
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit): upload.main()
            key.assert_not_called()
        api = FakeApple([None, "VALID"])
        with patch.dict(os.environ, {**hosted, "IOS_STORE_UPLOAD_READY": "true"}), patch("sys.argv", args), \
             patch.object(upload, "verify_export"), patch.object(upload, "credentials", return_value=contextlib.nullcontext(api)):
            with contextlib.redirect_stdout(io.StringIO()): upload.main()
        self.assertEqual(len(api.uploads), 1)
        self.assertEqual(self.receipt()["status"], "processed")
        self.assertEqual(self.receipt()["version"], "1.0.0")

    def test_private_key_is_cleared_from_child_environment_and_removed_after_failure(self):
        os.environ.update(ASC_API_KEY_ID="TESTKEY123", ASC_API_ISSUER_ID="00000000-0000-4000-8000-000000000001",
                          ASC_API_PRIVATE_KEY_BASE64=base64.b64encode(b"-----BEGIN PRIVATE KEY-----\nfixture").decode())
        with self.assertRaisesRegex(ValueError, "stop"):
            with upload.credentials() as api:
                path = api.path
                self.assertTrue(path.is_file())
                self.assertEqual(path.stat().st_mode & 0o777, 0o600)
                self.assertNotIn("ASC_API_PRIVATE_KEY_BASE64", api.env)
                raise ValueError("stop")
        self.assertFalse(path.exists())

    def test_token_from_apple_stderr_is_used_without_printing_or_regenerating(self):
        issuer = "00000000-0000-4000-8000-000000000001"
        claims = base64.urlsafe_b64encode(json.dumps({"iss": issuer, "aud": "appstoreconnect-v1",
                                                    "exp": int(time.time()) + 600}).encode()).decode().rstrip("=")
        token = "eyJhbGciOiJFUzI1NiJ9." + claims + ".synthetic"
        api = upload.AppleAPI("TESTKEY123", issuer, self.root / "key.p8")
        with patch.object(upload, "apple_tool", return_value="JWT: " + token) as tool:
            self.assertEqual(api.bearer(), token)
            self.assertEqual(api.bearer(), token)
            self.assertEqual(tool.call_count, 1)

    def test_apple_structured_product_errors_fail_even_with_exit_zero(self):
        from types import SimpleNamespace
        for report in [{'product-errors':[{'message':'Validation failed'}]}, {'product-errors':[]}]:
            result=SimpleNamespace(returncode=0,stdout=json.dumps(report).encode(),stderr=b'')
            with patch.object(upload.subprocess,'run',return_value=result):
                if report['product-errors']:
                    with self.assertRaisesRegex(ValueError,'product errors'):
                        upload.apple_tool('--validate-app','fixture.pkg',env={})
                else:
                    self.assertIn('product-errors',upload.apple_tool('--validate-app','fixture.pkg',env={}))

    def test_upload_readiness_and_host_guards_run_before_key_use(self):
        args = ["ios-upload.py", str(self.root), str(self.output), "--prior-receipts", str(self.prior)]
        hosted = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted", "RUNNER_OS": "macOS",
                  "GITHUB_REF": "refs/tags/v1.0.0", "GITHUB_EVENT_NAME": "push"}
        for overrides in [{}, {**hosted}, *[
                {**hosted, "IOS_STORE_UPLOAD_READY": "true", "GITHUB_REF": ref}
                for ref in ["refs/heads/master", "refs/heads/other", "refs/tags/v1.0.0-nightly.1.gabcdef0",
                            "refs/tags/v1.0.0-rc.0", "refs/tags/v1.0.0-rc.01"]]]:
            with patch.dict(os.environ, overrides), patch("sys.argv", args), patch.object(upload, "credentials") as key:
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                    upload.main()
                key.assert_not_called()

    def test_explicit_check_only_main_checks_auth_without_upload_readiness(self):
        (self.root / "export.json").write_text(json.dumps(self.record))
        api = FakeApple([None])
        hosted = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted", "RUNNER_OS": "macOS",
                  "GITHUB_REF": "refs/tags/v1.0.0", "GITHUB_EVENT_NAME": "push"}
        args = ["ios-upload.py", str(self.root), str(self.output), "--prior-receipts", str(self.prior), "--check-only"]
        with patch.dict(os.environ, hosted), patch("sys.argv", args), patch.object(upload, "verify_export") as verify:
            with patch.object(upload, "credentials", return_value=contextlib.nullcontext(api)):
                with contextlib.redirect_stdout(io.StringIO()):
                    upload.main()
            verify.assert_called_once()
        self.assertEqual(api.uploads, [])
        self.assertEqual(self.receipt()["status"], "checkedOnly")


if __name__ == "__main__":
    unittest.main()
