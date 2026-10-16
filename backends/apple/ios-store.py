#!/usr/bin/env python3
"""Local iOS archive/export preparation. This tool never uploads a build."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[2]
APP_ID = "fail.acheron.floatview"
EXTENSION_ID = APP_ID + ".live-activity"


def run(*args, capture=False):
    return subprocess.run(
        args, cwd=ROOT, check=True, stdout=subprocess.PIPE if capture else None
    ).stdout


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_plist(path):
    with path.open("rb") as source:
        return plistlib.load(source)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_digest(path):
    # Include metadata, resources, profiles and signatures, not just executables.
    result = hashlib.sha256()
    for file in sorted(path.rglob("*")):
        require(not file.is_symlink(), "Unexpected symlink in iOS app bundle")
        if file.is_file():
            result.update(file.relative_to(path).as_posix().encode() + b"\0")
            result.update(bytes.fromhex(digest(file)))
    return result.hexdigest()


def toolchain(store):
    xcode = run("xcodebuild", "-version", capture=True).decode().strip()
    sdk = run("xcrun", "--sdk", "iphoneos", "--show-sdk-version", capture=True).decode().strip()
    version = re.search(r"^Xcode (\d+)", xcode)
    require(version is not None and re.fullmatch(r"\d+(?:\.\d+)*", sdk), "Cannot identify Xcode/iOS SDK")
    if store:
        require(int(version[1]) >= 26 and int(sdk.split(".")[0]) >= 26,
                "Store signing requires Xcode 26+ and iOS SDK 26+; use archive-check for unsigned structural checks")
    return {"xcode": xcode, "iosSDK": sdk}


def bundles(app, version, number):
    extensions = sorted((app / "PlugIns").glob("*.appex"))
    require(len(extensions) == 1, "Expected exactly one Live Activity extension")
    result = [(app, APP_ID), (extensions[0], EXTENSION_ID)]
    for bundle, identifier in result:
        info = load_plist(bundle / "Info.plist")
        require(info["CFBundleIdentifier"] == identifier, "Unexpected app/extension identifier")
        require(info["CFBundleShortVersionString"] == version, "App/extension marketing version mismatch")
        require(info["CFBundleVersion"] == number, "App/extension build number mismatch")
        executable = bundle / info["CFBundleExecutable"]
        architectures = run("lipo", "-archs", str(executable), capture=True).decode().split()
        require(architectures == ["arm64"], "Expected an arm64 device executable")
        require(info.get("CFBundleSupportedPlatforms") == ["iPhoneOS"], "Expected an iOS device bundle")
    require((app / "index.html").is_file(), "Archive is missing the shared frontend")
    return result


def verify_distribution(app, version, number, team):
    for bundle, identifier in bundles(app, version, number):
        run("codesign", "--verify", "--strict", "--verbose=2", "-R",
            '=anchor apple generic and certificate leaf[field.1.2.840.113635.100.6.1.4] exists'
            ' and certificate leaf[subject.OU] = "' + team + '"', str(bundle))
        profile = plistlib.loads(run("security", "cms", "-D", "-i", str(bundle / "embedded.mobileprovision"), capture=True))
        require(profile.get("TeamIdentifier") == [team], "Provisioning profile belongs to another team")
        require(profile["ExpirationDate"] > datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None),
                "Provisioning profile has expired")
        require(not profile.get("ProvisionedDevices") and not profile.get("ProvisionsAllDevices"),
                "Expected an App Store profile, not development/ad-hoc/enterprise")
        prefix = profile["ApplicationIdentifierPrefix"][0]
        expected_id = prefix + "." + identifier
        for entitlements in [profile["Entitlements"], plistlib.loads(
                run("xcrun", "swift", str(ROOT / "backends/apple/signing-entitlements.swift"),
                    str(bundle), capture=True))]:
            require(entitlements.get("application-identifier") == expected_id,
                    "Provisioning/signature application identifier mismatch")
            require(entitlements.get("com.apple.developer.team-identifier") == team,
                    "Provisioning/signature team mismatch")
            require(entitlements.get("get-task-allow", False) is False, "Export must disable debugger attachment")


def archive(args):
    store = args.command == "archive"
    version = os.environ.get("FLOAT_VIEW_VERSION", "")
    number = os.environ.get("FLOAT_VIEW_BUILD_NUMBER", "")
    require(re.fullmatch(r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)", version),
            "Set FLOAT_VIEW_VERSION to vX.Y.Z; no stable tag is needed")
    require(re.fullmatch(r"[1-9]\d{0,9}", number) and int(number) <= 2_100_000_000,
            "Set an explicit positive FLOAT_VIEW_BUILD_NUMBER (maximum 2100000000)")
    tools = toolchain(store)
    team = os.environ.get("APPLE_TEAM_ID", "") if store else None
    if store:
        require(re.fullmatch(r"[A-Z0-9]{10}", team), "Set APPLE_TEAM_ID to the publishing team")
    require(not args.output.exists(), "Output already exists; choose a new archive directory")
    commit = run("git", "rev-parse", "HEAD", capture=True).decode().strip()
    dirty = bool(run("git", "status", "--porcelain", capture=True).strip())
    require(not store or not dirty, "Commit source changes before making a signed store archive")
    # A clean check applies before generating the ignored embedded frontend.
    run("just", "build")
    args.output.mkdir(parents=True)
    archive_path = args.output / "FloatView.xcarchive"
    command = ["xcodebuild", "-project", "backends/apple/ios/float-view.xcodeproj",
               "-scheme", "float-view", "-configuration", "Release", "-sdk", "iphoneos",
               "-destination", "generic/platform=iOS", "-derivedDataPath",
               str(Path(tempfile.gettempdir()) / "float-view-ios-store"),
               "-archivePath", str(archive_path), "MARKETING_VERSION=" + version[1:],
               "CURRENT_PROJECT_VERSION=" + number]
    if store:
        command += ["DEVELOPMENT_TEAM=" + team, "CODE_SIGN_STYLE=Automatic",
                    "CODE_SIGNING_ALLOWED=YES", "CODE_SIGNING_REQUIRED=YES"]
        if args.allow_provisioning_updates:
            command += ["-allowProvisioningUpdates"]
    else:
        command += ["CODE_SIGNING_ALLOWED=NO"]
    run(*command, "archive")
    app = archive_path / "Products/Applications/float-view.app"
    archived_bundles = bundles(app, version[1:], number)
    if store:
        for bundle, _ in archived_bundles:
            run("codesign", "--verify", "--strict", "-R",
                '=anchor apple generic and certificate leaf[subject.OU] = "' + team + '"', str(bundle))
    require(not (archive_path / "Products/Applications/float-view-lifecycle-tests.xctest").exists(),
            "Unexpected lifecycle test product")
    if store:
        require(run("git", "rev-parse", "HEAD", capture=True).decode().strip() == commit
                and not run("git", "status", "--porcelain", capture=True).strip(),
                "Source changed during archiving; make a fresh archive from a clean revision")
    record = {"schema": 1, "purpose": "store" if store else "unsigned-check",
              "commit": commit, "dirty": dirty, "version": version[1:], "buildNumber": number,
              "team": team, "toolchain": tools, "appSHA256": tree_digest(app)}
    (args.output / "archive.json").write_text(json.dumps(record, indent=2) + "\n")
    print("Archive prepared: " + str(args.output))


def export(args):
    toolchain(True)
    record = json.loads((args.archive / "archive.json").read_text())
    require(record.get("schema") == 1 and record.get("purpose") == "store" and not record.get("dirty"),
            "Only a signed, clean-source store archive can be exported")
    require(not args.output.exists(), "Output already exists; choose a new export directory")
    archive_path = args.archive / "FloatView.xcarchive"
    app = archive_path / "Products/Applications/float-view.app"
    require(tree_digest(app) == record["appSHA256"], "Archive app changed after it was recorded")
    args.output.mkdir(parents=True)
    options = {"method": "app-store-connect", "destination": "export", "teamID": record["team"],
               "signingStyle": "automatic", "manageAppVersionAndBuildNumber": False,
               "uploadSymbols": True}
    with tempfile.TemporaryDirectory(prefix="float-view-ios-export-") as temporary:
        options_path = Path(temporary) / "ExportOptions.plist"
        options_path.write_bytes(plistlib.dumps(options))
        command = ["xcodebuild", "-exportArchive", "-archivePath", str(archive_path),
                   "-exportPath", str(args.output), "-exportOptionsPlist", str(options_path)]
        if args.allow_provisioning_updates:
            command += ["-allowProvisioningUpdates"]
        run(*command)
        ipas = list(args.output.glob("*.ipa"))
        require(len(ipas) == 1, "Expected exactly one exported IPA")
        unpacked = Path(temporary) / "unpacked"
        run("ditto", "-x", "-k", str(ipas[0]), str(unpacked))
        apps = list((unpacked / "Payload").glob("*.app"))
        require(len(apps) == 1, "Expected exactly one exported app")
        verify_distribution(apps[0], record["version"], record["buildNumber"], record["team"])
        record["ipaSHA256"] = digest(ipas[0])
        record["ipa"] = ipas[0].name
    (args.output / "export.json").write_text(json.dumps(record, indent=2) + "\n")
    print("Verified store export (not uploaded): " + str(args.output))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ["archive-check", "archive", "export"]:
        command = commands.add_parser(name)
        if name == "export":
            command.add_argument("archive", type=lambda value: Path(value).resolve())
        command.add_argument("output", type=lambda value: Path(value).resolve())
        if name != "archive-check":
            command.add_argument("--allow-provisioning-updates", action="store_true",
                                 help="Allow Xcode to create/update profiles using its signed-in account")
    args = parser.parse_args()
    try:
        export(args) if args.command == "export" else archive(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
