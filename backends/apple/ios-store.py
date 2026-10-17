#!/usr/bin/env python3
"""Local iOS archive/export preparation. This tool never uploads a build."""

import argparse
import base64
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


def verify_frontend(app):
    html = (app / "index.html").read_text()
    for name in ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.txt"]:
        text = (ROOT / name).read_text()
        # Vite emits raw-text imports as JS template literals or quoted strings.
        template = "`" + text.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${") + "`"
        quoted = [json.dumps(text, ensure_ascii=ascii_only) for ascii_only in [False, True]]
        require(template in html or any(value in html for value in quoted),
                "Shared frontend is missing or has stale full legal text: " + name)


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
        binary = executable.read_bytes()
        for marker in [b"FLOAT_VIEW_CONFORMANCE_RESET", b"FixtureBluetoothAdapter", b"setInspectable:",
                       b"floatViewPerformance", b"FloatView frontend performance"]:
            require(marker not in binary, "Release executable contains a development entry point: " + marker.decode())
        architectures = run("lipo", "-archs", str(executable), capture=True).decode().split()
        require(architectures == ["arm64"], "Expected an arm64 device executable")
        require(info.get("CFBundleSupportedPlatforms") == ["iPhoneOS"], "Expected an iOS device bundle")
        manifest_source = ROOT / "backends/apple/ios" / (
            "float-view" if identifier == APP_ID else "FloatViewLiveActivity") / "PrivacyInfo.xcprivacy"
        manifest = bundle / "PrivacyInfo.xcprivacy"
        require(manifest.is_file() and load_plist(manifest) == load_plist(manifest_source),
                "Missing or changed app/extension privacy manifest")
        if identifier == APP_ID:
            if 2 in info.get("UIDeviceFamily", []):
                required_orientations = {"UIInterfaceOrientationPortrait", "UIInterfaceOrientationPortraitUpsideDown",
                                         "UIInterfaceOrientationLandscapeLeft", "UIInterfaceOrientationLandscapeRight"}
                orientations = info.get("UISupportedInterfaceOrientations~ipad",
                                        info.get("UISupportedInterfaceOrientations", []))
                require(required_orientations.issubset(set(orientations)),
                        "iPad store support requires all interface orientations for multitasking")
            require(info.get("ITSAppUsesNonExemptEncryption") is False,
                    "Missing reviewed encryption declaration")
    require((app / "index.html").is_file(), "Archive is missing the shared frontend")
    verify_frontend(app)
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
    ci = None
    if os.environ.get("FLOAT_VIEW_BUILD_RECORD"):
        ci = json.loads(Path(os.environ["FLOAT_VIEW_BUILD_RECORD"]).read_text())
        require(ci.get("schema") == 1 and ci.get("commit") == commit
                and ci.get("appVersion") == version[1:] and ci.get("buildNumber") == number
                and ci.get("runId") == os.environ.get("GITHUB_RUN_ID")
                and ci.get("workflowPath") == ".github/workflows/build-apps.yml"
                and ((ci.get("channel") == "stable" and ci.get("tag") == version)
                     or (ci.get("channel") == "candidate" and re.fullmatch(re.escape(version) + r"-rc\.[1-9]\d*", ci.get("tag", ""))))
                and os.environ.get("GITHUB_REF") == "refs/tags/" + ci.get("tag", ""),
                "CI build record does not match the source/version/run")
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
        manual = getattr(args, "manual_signing", False)
        command += ["DEVELOPMENT_TEAM=" + team, "CODE_SIGN_STYLE=" + ("Manual" if manual else "Automatic"),
                    "CODE_SIGNING_ALLOWED=YES", "CODE_SIGNING_REQUIRED=YES"]
        if manual:
            identity = os.environ.get("CODE_SIGN_IDENTITY", "")
            require(re.fullmatch(r"[A-F0-9]{40}", identity), "Missing selected distribution identity")
            for setting in ["FLOAT_VIEW_IOS_APP_PROFILE", "FLOAT_VIEW_IOS_EXTENSION_PROFILE"]:
                require(os.environ.get(setting), "Missing " + setting)
                command += [setting + "=" + os.environ[setting]]
            command += ["CODE_SIGN_IDENTITY=" + identity]
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
    record = {"schema": 1, "purpose": "store" if store else "unsigned-check", "configuration": "Release",
              "commit": commit, "dirty": dirty, "version": version[1:], "buildNumber": number,
              "team": team, "toolchain": tools, "appSHA256": tree_digest(app)}
    if ci:
        ci["buildAttempt"] = os.environ.get("GITHUB_RUN_ATTEMPT")
        ci["buildJob"] = os.environ.get("GITHUB_JOB")
        record["ci"] = ci
    if store and getattr(args, "manual_signing", False):
        record["signingIdentity"] = os.environ["CODE_SIGN_IDENTITY"]
        record["profiles"] = {APP_ID: os.environ["FLOAT_VIEW_IOS_APP_PROFILE"],
                              EXTENSION_ID: os.environ["FLOAT_VIEW_IOS_EXTENSION_PROFILE"]}
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
    if "profiles" in record:
        options.update(signingStyle="manual", provisioningProfiles=record["profiles"],
                       signingCertificate=record["signingIdentity"])
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


def ci_build(args):
    require(os.environ.get("GITHUB_ACTIONS") == "true"
            and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
            and os.environ.get("RUNNER_OS") == "macOS", "CI profile import requires a hosted macOS runner")
    require(re.fullmatch(r"refs/tags/v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-rc\.[1-9]\d*)?", os.environ.get("GITHUB_REF", ""))
            and os.environ.get("GITHUB_EVENT_NAME") in ["push", "workflow_dispatch"],
            "CI store signing requires a stable release tag or numbered candidate; never master or nightly")
    require(os.environ.get("FLOAT_VIEW_BUILD_RECORD"), "CI requires the shared build record")
    require(not args.output.exists(), "Output already exists")
    # Clear both profile secrets before invoking any child tool or Xcode build.
    encoded = {name: os.environ.pop(name, "") for name in
               ["IOS_APP_PROFILE_BASE64", "IOS_EXTENSION_PROFILE_BASE64"]}
    require(all(encoded.values()), "Missing iOS App Store provisioning profile secrets")
    installed = []
    try:
        with tempfile.TemporaryDirectory(prefix="float-view-ios-profiles-") as temporary:
            for index, (name, identifier, setting) in enumerate([
                    ("IOS_APP_PROFILE_BASE64", APP_ID, "FLOAT_VIEW_IOS_APP_PROFILE"),
                    ("IOS_EXTENSION_PROFILE_BASE64", EXTENSION_ID, "FLOAT_VIEW_IOS_EXTENSION_PROFILE")]):
                path = Path(temporary) / (str(index) + ".mobileprovision")
                path.write_bytes(base64.b64decode("".join(encoded.pop(name).split()), validate=True))
                path.chmod(0o600)
                profile = plistlib.loads(run("security", "cms", "-D", "-i", str(path), capture=True))
                require(not profile.get("IsXcodeManaged", False),
                        "CI manual signing requires profiles created in the developer portal, not Xcode-managed profiles")
                team = os.environ.get("APPLE_TEAM_ID")
                require(profile.get("TeamIdentifier") == [team], "Profile belongs to another team")
                require(profile["Entitlements"].get("application-identifier") ==
                        profile["ApplicationIdentifierPrefix"][0] + "." + identifier,
                        "Wrong app/extension provisioning profile")
                require(profile["ExpirationDate"] > datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
                        and not profile.get("ProvisionedDevices") and not profile.get("ProvisionsAllDevices")
                        and profile["Entitlements"].get("get-task-allow", False) is False,
                        "Expected an unexpired App Store distribution profile")
                require(os.environ.get("CODE_SIGN_IDENTITY") in
                        [hashlib.sha1(certificate).hexdigest().upper() for certificate in profile["DeveloperCertificates"]],
                        "Profile does not authorize the selected distribution certificate")
                uuid = profile["UUID"]
                require(re.fullmatch(r"[A-Fa-f0-9]{8}(?:-[A-Fa-f0-9]{4}){3}-[A-Fa-f0-9]{12}", uuid),
                        "Invalid profile UUID")
                destination = Path.home() / "Library/Developer/Xcode/UserData/Provisioning Profiles" / (uuid + ".mobileprovision")
                destination.parent.mkdir(parents=True, exist_ok=True)
                require(not destination.exists(), "Refusing to replace an installed profile")
                destination.write_bytes(path.read_bytes())
                destination.chmod(0o600)
                installed.append(destination)
                os.environ[setting] = uuid
            archive(argparse.Namespace(command="archive", output=args.output / "archive",
                                       manual_signing=True, allow_provisioning_updates=False))
            export(argparse.Namespace(archive=args.output / "archive", output=args.output / "export",
                                      allow_provisioning_updates=False))
    finally:
        for path in installed:
            path.unlink(missing_ok=True)
        for setting in ["FLOAT_VIEW_IOS_APP_PROFILE", "FLOAT_VIEW_IOS_EXTENSION_PROFILE"]:
            os.environ.pop(setting, None)


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
    command = commands.add_parser("ci-build", help="Hosted-runner manual signing; no upload")
    command.add_argument("output", type=lambda value: Path(value).resolve())
    args = parser.parse_args()
    try:
        if args.command == "ci-build":
            ci_build(args)
        else:
            export(args) if args.command == "export" else archive(args)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
