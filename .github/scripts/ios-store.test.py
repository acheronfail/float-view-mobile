"""Store-package boundary checks, with no Apple account or signing keys."""
import argparse
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
                "CFBundleSupportedPlatforms": ["iPhoneOS"]}))
            (bundle / "executable").write_bytes(b"fixture-executable")
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
        environment = {"FLOAT_VIEW_VERSION": "v1.0.0", "FLOAT_VIEW_BUILD_NUMBER": "101", "APPLE_TEAM_ID": TEAM}
        with patch.dict(os.environ, environment), patch.object(store, "run", side_effect=build):
            store.archive(argparse.Namespace(command="archive", output=output, allow_provisioning_updates=False))
        command = next(x for x in self.operations if x[0] == "xcodebuild" and "archive" in x)
        self.assertEqual(command[command.index("-configuration") + 1], "Release")
        self.assertIn("DEVELOPMENT_TEAM=" + TEAM, command)
        self.assertIn("CODE_SIGNING_REQUIRED=YES", command)
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
        output = self.root / "export"
        original = self.run_tool
        def export(*args, **kwargs):
            if args[:2] == ("xcodebuild", "-exportArchive"):
                options = store.load_plist(Path(args[args.index("-exportOptionsPlist") + 1]))
                self.assertEqual(options["destination"], "export")
                self.assertEqual(options["method"], "app-store-connect")
                self.assertFalse(options["manageAppVersionAndBuildNumber"])
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


if __name__ == "__main__":
    unittest.main()
