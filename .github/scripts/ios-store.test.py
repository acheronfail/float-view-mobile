"""Store-package boundary checks, with no Apple account or signing keys."""
import argparse
import base64
import hashlib
import datetime
import importlib.util
import json
import os
from pathlib import Path
import plistlib
import subprocess
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "ios_store", Path(__file__).resolve().parents[2] / "backends/apple/ios-store.py")
store = importlib.util.module_from_spec(spec)
spec.loader.exec_module(store)
TEAM = "TESTTEAM01"


class StoreTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.app = self.root / "float-view.app"
        self.extension = self.app / "PlugIns/FloatViewLiveActivity.appex"
        self.operations = []
        self.profile_changes = {}
        self.signature_changes = {}
        for bundle, identifier in [(self.app, store.APP_ID), (self.extension, store.EXTENSION_ID)]:
            bundle.mkdir(parents=True)
            (bundle / "Info.plist").write_bytes(plistlib.dumps({
                "CFBundleIdentifier": identifier, "CFBundleShortVersionString": "1.0.0",
                "CFBundleVersion": "101", "CFBundleExecutable": "executable",
                "CFBundleSupportedPlatforms": ["iPhoneOS"], "ITSAppUsesNonExemptEncryption": False}))
            (bundle / "executable").write_bytes(b"fixture-executable")
            source = store.ROOT / "backends/apple/ios" / (
                "float-view" if identifier == store.APP_ID else "FloatViewLiveActivity") / "PrivacyInfo.xcprivacy"
            shutil.copyfile(source, bundle / "PrivacyInfo.xcprivacy")
        (self.app / "index.html").write_text("shared frontend")
        self.runner = patch.object(store, "run", side_effect=self.run_tool)
        self.runner.start()
        self.addCleanup(self.runner.stop)

    def run_tool(self, *args, capture=False):
        self.operations.append(args)
        if args[0] == "lipo":
            return b"arm64\n"
        if args[:2] == ("xcodebuild", "-version"):
            return b"Xcode 26.0\nBuild version TEST"
        if args[0] == "xcrun" and args[1] != "swift":
            return b"26.0\n"
        if args[:3] == ("git", "rev-parse", "HEAD"):
            return b"a" * 40
        if args[:3] == ("git", "status", "--porcelain"):
            return b""
        if args[0] in ("security", "codesign") or args[:2] == ("xcrun", "swift"):
            if args[0] == "codesign" and "-R" in args:
                self.assertTrue(args[args.index("-R") + 1].startswith("="),
                                "codesign requires = before an inline requirement; otherwise it reads a file")
            identifier = store.EXTENSION_ID if "FloatViewLiveActivity.appex" in args[-1] else store.APP_ID
            entitlements = {"application-identifier": TEAM + "." + identifier,
                            "com.apple.developer.team-identifier": TEAM, "get-task-allow": False}
            if args[0] in ("codesign", "xcrun"):
                entitlements.update(self.signature_changes)
                return plistlib.dumps(entitlements) if args[0] == "xcrun" else b""
            profile = {"ApplicationIdentifierPrefix": [TEAM], "TeamIdentifier": [TEAM],
                       "ExpirationDate": datetime.datetime(2099, 1, 1), "Entitlements": entitlements}
            profile.update(self.profile_changes)
            return plistlib.dumps(profile)
        raise AssertionError("Unexpected tool: " + repr(args))

    def test_distribution_checks_both_bundles(self):
        store.verify_distribution(self.app, "1.0.0", "101", TEAM)
        self.assertEqual(len([x for x in self.operations if x[0] == "security"]), 2)
        self.assertEqual(len([x for x in self.operations if x[:2] == ("codesign", "--verify")]), 2)

    def test_development_profile_rejected(self):
        self.profile_changes["ProvisionedDevices"] = ["test-device"]
        with self.assertRaisesRegex(ValueError, "App Store profile"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_missing_or_changed_packaged_privacy_manifest_rejected(self):
        for bundle in [self.app, self.extension]:
            with self.subTest(bundle=bundle.name):
                path = bundle / "PrivacyInfo.xcprivacy"
                original = path.read_bytes()
                path.unlink()
                with self.assertRaisesRegex(ValueError, "privacy manifest"):
                    store.bundles(self.app, "1.0.0", "101")
                path.write_bytes(plistlib.dumps({"NSPrivacyTracking": True}))
                with self.assertRaisesRegex(ValueError, "privacy manifest"):
                    store.bundles(self.app, "1.0.0", "101")
                path.write_bytes(original)

    def test_missing_encryption_declaration_rejected(self):
        path = self.app / "Info.plist"
        info = plistlib.loads(path.read_bytes())
        del info["ITSAppUsesNonExemptEncryption"]
        path.write_bytes(plistlib.dumps(info))
        with self.assertRaisesRegex(ValueError, "encryption declaration"):
            store.bundles(self.app, "1.0.0", "101")

    def test_enterprise_profile_rejected(self):
        self.profile_changes["ProvisionsAllDevices"] = True
        with self.assertRaisesRegex(ValueError, "App Store profile"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_other_team_rejected(self):
        self.profile_changes["TeamIdentifier"] = ["OTHERTEAM1"]
        with self.assertRaisesRegex(ValueError, "another team"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_expired_profile_rejected(self):
        self.profile_changes["ExpirationDate"] = datetime.datetime(2000, 1, 1)
        with self.assertRaisesRegex(ValueError, "expired"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_debugger_entitlement_rejected(self):
        self.signature_changes["get-task-allow"] = True
        with self.assertRaisesRegex(ValueError, "debugger"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_wrong_application_entitlement_rejected(self):
        self.signature_changes["application-identifier"] = TEAM + ".wrong"
        with self.assertRaisesRegex(ValueError, "application identifier"):
            store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_signature_failure_propagates(self):
        original = self.run_tool
        def fail_signature(*args, **kwargs):
            if args[:2] == ("codesign", "--verify"):
                raise subprocess.CalledProcessError(1, args)
            return original(*args, **kwargs)
        with patch.object(store, "run", side_effect=fail_signature):
            with self.assertRaises(subprocess.CalledProcessError):
                store.verify_distribution(self.app, "1.0.0", "101", TEAM)

    def test_extension_version_mismatch_rejected(self):
        path = self.extension / "Info.plist"
        info = plistlib.loads(path.read_bytes())
        info["CFBundleVersion"] = "100"
        path.write_bytes(plistlib.dumps(info))
        with self.assertRaisesRegex(ValueError, "build number mismatch"):
            store.bundles(self.app, "1.0.0", "101")

    def test_missing_extension_rejected(self):
        with self.assertRaisesRegex(ValueError, "exactly one"):
            store.bundles(self.extension, "1.0.0", "101")

    def test_old_sdk_allowed_only_for_unsigned_check(self):
        with patch.object(store, "run", side_effect=[b"Xcode 16.4", b"18.5"]):
            store.toolchain(False)
        with patch.object(store, "run", side_effect=[b"Xcode 16.4", b"18.5"]):
            with self.assertRaisesRegex(ValueError, "Xcode 26"):
                store.toolchain(True)

    def test_explicit_metadata_required_before_build(self):
        args = argparse.Namespace(command="archive-check", output=self.root / "output")
        for version, number in [("", "101"), ("v1.0.0-nightly.1.gabcdef1", "101"), ("v1.0.0", ""), ("v1.0.0", "0")]:
            with self.subTest(version=version, number=number):
                with patch.dict(os.environ, {"FLOAT_VIEW_VERSION": version, "FLOAT_VIEW_BUILD_NUMBER": number}):
                    with self.assertRaises(ValueError):
                        store.archive(args)
        self.assertEqual(self.operations, [])

    def test_unsigned_archive_cannot_be_exported(self):
        (self.root / "archive.json").write_text(json.dumps({"schema": 1, "purpose": "unsigned-check", "dirty": False}))
        with self.assertRaisesRegex(ValueError, "signed, clean-source"):
            store.export(argparse.Namespace(archive=self.root, output=self.root / "output"))
        self.assertFalse((self.root / "output").exists())

    def test_signed_archive_refuses_dirty_source_before_building(self):
        original = self.run_tool
        def dirty(*args, **kwargs):
            if args[:3] == ("git", "status", "--porcelain"):
                return b" M backends/apple/ios-store.py"
            return original(*args, **kwargs)
        environment = {"FLOAT_VIEW_VERSION": "v1.0.0", "FLOAT_VIEW_BUILD_NUMBER": "101", "APPLE_TEAM_ID": TEAM}
        with patch.dict(os.environ, environment), patch.object(store, "run", side_effect=dirty):
            with self.assertRaisesRegex(ValueError, "Commit source changes"):
                store.archive(argparse.Namespace(command="archive", output=self.root / "output"))
        self.assertFalse((self.root / "output").exists())

    def test_changed_archive_cannot_be_exported(self):
        archive = self.root / "archive"
        target = archive / "FloatView.xcarchive/Products/Applications/float-view.app"
        target.mkdir(parents=True)
        (archive / "archive.json").write_text(json.dumps({"schema": 1, "purpose": "store", "dirty": False, "appSHA256": "incorrect"}))
        with self.assertRaisesRegex(ValueError, "changed"):
            store.export(argparse.Namespace(archive=archive, output=self.root / "output"))

    def test_resource_changes_invalidate_archive_digest(self):
        before = store.tree_digest(self.app)
        (self.app / "index.html").write_text("different shared frontend")
        self.assertNotEqual(before, store.tree_digest(self.app))

    def test_archive_always_release_and_one_team_for_both_targets(self):
        original = self.run_tool
        output = self.root / "archive"
        def build(*args, **kwargs):
            if args[0] == "just":
                return b""
            if args[0] == "xcodebuild" and "archive" in args:
                self.operations.append(args)
                target = output / "FloatView.xcarchive/Products/Applications/float-view.app"
                shutil.copytree(self.app, target)
                return b""
            return original(*args, **kwargs)
        environment = {"FLOAT_VIEW_VERSION": "v1.0.0", "FLOAT_VIEW_BUILD_NUMBER": "101", "APPLE_TEAM_ID": TEAM,
                       "CODE_SIGN_IDENTITY": "A" * 40, "FLOAT_VIEW_IOS_APP_PROFILE": "app-profile",
                       "FLOAT_VIEW_IOS_EXTENSION_PROFILE": "extension-profile"}
        with patch.dict(os.environ, environment), patch.object(store, "run", side_effect=build):
            store.archive(argparse.Namespace(command="archive", output=output, allow_provisioning_updates=False,
                                             manual_signing=True))
        command = next(x for x in self.operations if x[0] == "xcodebuild" and "archive" in x)
        self.assertEqual(command[command.index("-configuration") + 1], "Release")
        self.assertIn("DEVELOPMENT_TEAM=" + TEAM, command)
        self.assertIn("CODE_SIGNING_REQUIRED=YES", command)
        self.assertIn("CODE_SIGN_STYLE=Manual", command)
        self.assertIn("FLOAT_VIEW_IOS_APP_PROFILE=app-profile", command)
        self.assertIn("FLOAT_VIEW_IOS_EXTENSION_PROFILE=extension-profile", command)
        self.assertNotIn("-allowProvisioningUpdates", command)
        record = json.loads((output / "archive.json").read_text())
        self.assertEqual(record["commit"], "a" * 40)
        self.assertFalse(record["dirty"])

    def test_export_stays_local_and_preserves_versions(self):
        archive = self.root / "archive"
        target = archive / "FloatView.xcarchive/Products/Applications/float-view.app"
        shutil.copytree(self.app, target)
        record = {"schema": 1, "purpose": "store", "dirty": False, "team": TEAM,
                  "version": "1.0.0", "buildNumber": "101", "appSHA256": store.tree_digest(target)}
        (archive / "archive.json").write_text(json.dumps(record))
        record["profiles"] = {store.APP_ID: "app-profile", store.EXTENSION_ID: "extension-profile"}
        record["signingIdentity"] = "A" * 40
        (archive / "archive.json").write_text(json.dumps(record))
        output = self.root / "export"
        original = self.run_tool
        def export(*args, **kwargs):
            if args[:2] == ("xcodebuild", "-exportArchive"):
                options = store.load_plist(Path(args[args.index("-exportOptionsPlist") + 1]))
                self.assertEqual(options["destination"], "export")
                self.assertEqual(options["method"], "app-store-connect")
                self.assertFalse(options["manageAppVersionAndBuildNumber"])
                if "profiles" in record:
                    self.assertEqual(options["signingStyle"], "manual")
                    self.assertEqual(options["provisioningProfiles"], record["profiles"])
                    self.assertEqual(options["signingCertificate"], record["signingIdentity"])
                else:
                    self.assertEqual(options["signingStyle"], "automatic")
                    self.assertNotIn("provisioningProfiles", options)
                (output / "float-view.ipa").write_bytes(b"export-fixture")
                return b""
            if args[0] == "ditto":
                shutil.copytree(self.app, Path(args[-1]) / "Payload/float-view.app")
                return b""
            return original(*args, **kwargs)
        with patch.object(store, "run", side_effect=export):
            store.export(argparse.Namespace(archive=archive, output=output, allow_provisioning_updates=False))
        result = json.loads((output / "export.json").read_text())
        self.assertEqual(result["ipaSHA256"], store.digest(output / "float-view.ipa"))
        del record["profiles"], record["signingIdentity"]
        (archive / "archive.json").write_text(json.dumps(record))
        output = self.root / "automatic-export"
        with patch.object(store, "run", side_effect=export):
            store.export(argparse.Namespace(archive=archive, output=output, allow_provisioning_updates=False))
        self.assertTrue((output / "export.json").exists())


    def test_ci_profile_import_refuses_workstation(self):
        with patch.dict(os.environ, {"GITHUB_ACTIONS": "false"}):
            with self.assertRaisesRegex(ValueError, "hosted macOS"):
                store.ci_build(argparse.Namespace(output=self.root / "output"))
        self.assertEqual(self.operations, [])

    def ci_environment(self, changes=None):
        profiles = []
        certificate = b"test-distribution-certificate"
        for index, identifier in enumerate([store.APP_ID, store.EXTENSION_ID]):
            profile = {"UUID": f"00000000-0000-0000-0000-{index:012d}",
                       "TeamIdentifier": [TEAM], "ApplicationIdentifierPrefix": [TEAM],
                       "ExpirationDate": datetime.datetime(2099, 1, 1),
                       "DeveloperCertificates": [certificate],
                       "Entitlements": {"application-identifier": TEAM + "." + identifier,
                                        "get-task-allow": False}}
            if index == 0 and changes:
                changes(profile)
            profiles.append(base64.b64encode(plistlib.dumps(profile)).decode())
        return {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted", "RUNNER_OS": "macOS",
                "APPLE_TEAM_ID": TEAM, "CODE_SIGN_IDENTITY": hashlib.sha1(certificate).hexdigest().upper(),
                "FLOAT_VIEW_BUILD_RECORD": str(self.root / "ci-record.json"),
                "IOS_APP_PROFILE_BASE64": profiles[0], "IOS_EXTENSION_PROFILE_BASE64": profiles[1]}

    def ci_decode(self, *args, **kwargs):
        self.assertNotIn("IOS_APP_PROFILE_BASE64", os.environ)
        self.assertNotIn("IOS_EXTENSION_PROFILE_BASE64", os.environ)
        self.assertEqual(args[:3], ("security", "cms", "-D"))
        return Path(args[-1]).read_bytes()

    def test_ci_profiles_cleared_before_tools_and_removed_on_success_or_build_failure(self):
        for fail in [False, True]:
            home = self.root / str(fail)
            def build(args):
                self.assertTrue(args.manual_signing)
                self.assertFalse(args.allow_provisioning_updates)
                self.assertEqual(len(list(home.rglob("*.mobileprovision"))), 2)
                self.assertNotIn("IOS_APP_PROFILE_BASE64", os.environ)
                if fail:
                    raise ValueError("build failed")
            with patch.dict(os.environ, self.ci_environment()), patch.object(Path, "home", return_value=home), \
                 patch.object(store, "run", side_effect=self.ci_decode), \
                 patch.object(store, "archive", side_effect=build), patch.object(store, "export") as export:
                args = argparse.Namespace(output=self.root / ("output-" + str(fail)))
                if fail:
                    with self.assertRaisesRegex(ValueError, "build failed"):
                        store.ci_build(args)
                    export.assert_not_called()
                else:
                    store.ci_build(args)
                    export.assert_called_once()
                self.assertEqual(list(home.rglob("*.mobileprovision")), [])
                self.assertNotIn("FLOAT_VIEW_IOS_APP_PROFILE", os.environ)

    def test_ci_rejects_wrong_team_identity_or_development_profiles(self):
        changes = [
            lambda p: p.update(IsXcodeManaged=True),
            lambda p: p.update(TeamIdentifier=["OTHERTEAM1"]),
            lambda p: p.update(DeveloperCertificates=[b"wrong certificate"]),
            lambda p: p.update(ProvisionedDevices=["test-device"]),
            lambda p: p.update(ExpirationDate=datetime.datetime(2000, 1, 1)),
            lambda p: p["Entitlements"].update({"application-identifier": TEAM + ".wrong"}),
        ]
        for change in changes:
            home = self.root / "rejected-home"
            with patch.dict(os.environ, self.ci_environment(change)), patch.object(Path, "home", return_value=home), \
                 patch.object(store, "run", side_effect=self.ci_decode), patch.object(store, "archive") as archive:
                with self.assertRaises(ValueError):
                    store.ci_build(argparse.Namespace(output=self.root / "rejected-output"))
                archive.assert_not_called()
                self.assertEqual(list(home.rglob("*.mobileprovision")), [])

    def test_ci_missing_profile_secret_fails_without_building(self):
        environment = self.ci_environment()
        environment["IOS_APP_PROFILE_BASE64"] = ""
        with patch.dict(os.environ, environment), patch.object(store, "archive") as archive:
            with self.assertRaisesRegex(ValueError, "Missing iOS"):
                store.ci_build(argparse.Namespace(output=self.root / "missing-output"))
            archive.assert_not_called()

    def test_archive_refuses_ci_record_for_another_build(self):
        record = self.root / "build-record.json"
        record.write_text(json.dumps({"schema": 1, "commit": "a" * 40, "appVersion": "1.0.0",
                                      "buildNumber": "999", "runId": "77",
                                      "workflowPath": ".github/workflows/build-apps.yml"}))
        environment = {"FLOAT_VIEW_VERSION": "v1.0.0", "FLOAT_VIEW_BUILD_NUMBER": "101",
                       "FLOAT_VIEW_BUILD_RECORD": str(record), "GITHUB_RUN_ID": "77", "APPLE_TEAM_ID": TEAM}
        with patch.dict(os.environ, environment):
            with self.assertRaisesRegex(ValueError, "CI build record"):
                store.archive(argparse.Namespace(command="archive", output=self.root / "wrong-ci"))



if __name__ == "__main__":
    unittest.main()
