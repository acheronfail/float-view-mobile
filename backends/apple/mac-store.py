#!/usr/bin/env python3
"""Build and inspect Mac sandbox checks or store packages. Never uploads."""

import argparse
import base64
import datetime
import hashlib
import json
import os
import re
import platform
import plistlib
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
APPLE = ROOT / "backends/apple"
STORE_ID = "fail.acheron.floatview.macos"
PROBE_ID = STORE_ID + ".sandbox-check"
ENTITLEMENTS = APPLE / "mac-store.entitlements"
PRIVACY = APPLE / "ios/float-view/PrivacyInfo.xcprivacy"
DEVELOPMENT_MARKERS = (
    b"--browser-host", b"--test-host", b"FLOAT_VIEW_FRONTEND_HTML",
    b"FLOAT_VIEW_LOG_ROOT", b"FLOAT_VIEW_CONFORMANCE_PORT",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args, cwd=ROOT, capture=False, env=None):
    result = subprocess.run(
        [str(a) for a in args], cwd=cwd, env=env,
        stdout=subprocess.PIPE if capture else None,
    )
    require(result.returncode == 0, f"{args[0]} failed; command arguments suppressed")
    return result.stdout.decode().strip() if capture else None


def inspect_app(app, version, number, architectures, identifier, expected_entitlements):
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    require(info["CFBundleIdentifier"] == identifier, "Unexpected Mac bundle identifier")
    require(info["CFBundleShortVersionString"] == version, "Version differs from build inputs")
    require(info["CFBundleVersion"] == number, "Build number differs from build inputs")
    require(info.get("LSApplicationCategoryType") == "public.app-category.utilities", "Missing Mac category")
    require(bool(info.get("NSHumanReadableCopyright")), "Missing copyright")
    binary = app / "Contents/MacOS/FloatViewMac"
    actual_archs = set(run("xcrun", "lipo", "-archs", binary, capture=True).split())
    require(actual_archs == set(architectures), "Unexpected executable architectures")
    data = binary.read_bytes()
    require(not any(marker in data for marker in DEVELOPMENT_MARKERS), "Release contains development entry points")
    run("codesign", "--verify", "--deep", "--strict", app)
    actual_entitlements = plistlib.loads(subprocess.check_output(
        ["xcrun", "swift", str(APPLE / "signing-entitlements.swift"), str(app)]
    ))
    require(actual_entitlements == expected_entitlements, "Unexpected sandbox entitlements")
    resources = app / "Contents/Resources"
    require((resources / "index.html").is_file(), "Missing embedded frontend")
    require((resources / "FloatView.icns").is_file(), "Missing app icon")
    require((resources / "PrivacyInfo.xcprivacy").read_bytes() == PRIVACY.read_bytes(), "Privacy manifest mismatch")
    require(b'id="app-loading"' in (resources / "index.html").read_bytes(), "Invalid frontend")
    return {"architectures": sorted(actual_archs), "sandboxEntitlementsVerified": True,
            "releaseDevelopmentEntryPointsExcluded": True, "privacyManifestVerified": True,
            "executableSHA256": hashlib.sha256(data).hexdigest()}


def inspect_probe(app, version, number, architectures):
    require(not (app / "Contents/embedded.provisionprofile").exists(), "Probe must not embed a store profile")
    return inspect_app(app, version, number, architectures, PROBE_ID,
                       plistlib.loads(ENTITLEMENTS.read_bytes()))


def build_app(output, version, number, architectures, identifier, name):
    run("just", "build")
    run("bash", "backends/core/build-apple-runtime.sh", "macosx", " ".join(architectures), "Release",
        ROOT / "backends/core/target/apple/macos")
    scratch = output / "build"
    swift_args = ["-c", "release", "--scratch-path", str(scratch)]
    for arch in architectures:
        swift_args += ["--arch", arch]
    run("swift", "build", *swift_args, "--product", "FloatViewMac", cwd=APPLE)
    binary_root = Path(run("swift", "build", *swift_args, "--show-bin-path", cwd=APPLE, capture=True))
    candidates = [binary_root / "FloatViewApple_FloatViewMac.bundle/Contents/Resources/Resources",
                  binary_root / "FloatViewApple_FloatViewMac.bundle/Resources"]
    resources = next((p for p in candidates if (p / "index.html").is_file()), None)
    require(resources is not None, "Cannot locate SwiftPM app resources")
    app = output / (name + ".app")
    run("bash", APPLE / "package-macos.sh", binary_root / "FloatViewMac", app, resources)
    info_path = app / "Contents/Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    info.update(CFBundleIdentifier=identifier, CFBundleDisplayName=name, CFBundleName=name,
                CFBundleShortVersionString=version, CFBundleVersion=number)
    sdk_version = run("xcrun", "--sdk", "macosx", "--show-sdk-version", capture=True)
    sdk_build = run("xcrun", "--sdk", "macosx", "--show-sdk-build-version", capture=True)
    info.update(CFBundleSupportedPlatforms=["MacOSX"], DTPlatformName="macosx",
                DTSDKName="macosx" + sdk_version, DTSDKBuild=sdk_build)
    info_path.write_bytes(plistlib.dumps(info))
    return app


def sandbox_check(args):
    output = Path(args.output).resolve()
    require(not output.exists(), "Output already exists; choose a new directory")
    metadata = run("node", "--strip-types", ".github/scripts/version.ts", "native", capture=True).split()
    require(len(metadata) == 2, "Set FLOAT_VIEW_VERSION and FLOAT_VIEW_BUILD_NUMBER")
    version, number = metadata
    architectures = ["arm64", "x86_64"] if args.arch == "universal" else [platform.machine()]
    require(set(architectures) <= {"arm64", "x86_64"}, "Unsupported Mac architecture")
    output.mkdir(parents=True)
    app = build_app(output, version, number, architectures, PROBE_ID, "Float View Sandbox Check")
    run("codesign", "--force", "--sign", "-", "--entitlements", ENTITLEMENTS, app)
    checks = inspect_probe(app, version, number, architectures)
    record = {"schema": 1, "purpose": "local sandbox feasibility; not uploadable",
              "storeBundleIdentifier": STORE_ID, "probeBundleIdentifier": PROBE_ID,
              "version": version, "buildNumber": number,
              "source": run("git", "rev-parse", "HEAD", capture=True),
              "dirty": bool(run("git", "status", "--porcelain", capture=True)),
              "app": str(app), "checks": checks, "runtimeVerified": False}
    (output / "sandbox-check.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


def validate_profile(profile, team, identity):
    require(re.fullmatch(r"[A-Z0-9]{10}", team), "Set the publishing APPLE_TEAM_ID")
    require(re.fullmatch(r"[A-F0-9]{40}", identity), "Set the Apple Distribution certificate SHA1")
    require(profile.get("TeamIdentifier") == [team] and profile.get("Platform") == ["OSX"],
            "Profile must target this team and macOS")
    require(profile.get("ExpirationDate", datetime.datetime.min) > datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None),
            "Profile is expired")
    require(not profile.get("ProvisionedDevices") and not profile.get("ProvisionsAllDevices")
            and not profile.get("IsXcodeManaged"), "Expected a manually created Mac App Store profile")
    entitlements = profile.get("Entitlements", {})
    require(entitlements.get("com.apple.application-identifier") == team + "." + STORE_ID
            and entitlements.get("com.apple.developer.team-identifier") == team,
            "Profile app/team identity mismatch")
    require(not entitlements.get("get-task-allow") and not entitlements.get("com.apple.security.get-task-allow"),
            "Distribution must not allow debugger attachment")
    certificates = profile.get("DeveloperCertificates", [])
    require(len(certificates) == 1 and hashlib.sha1(certificates[0]).hexdigest().upper() == identity,
            "Profile does not select the Apple Distribution signer")
    expected = plistlib.loads(ENTITLEMENTS.read_bytes())
    expected.update({"com.apple.application-identifier": team + "." + STORE_ID,
                     "com.apple.developer.team-identifier": team})
    # Do not copy wildcard keychain groups or other unused profile capabilities.
    return expected


def verify_notices(app):
    html = (app / "Contents/Resources/index.html").read_text()
    for name in ["LICENSE", "NOTICE", "THIRD_PARTY_NOTICES.txt"]:
        text = (ROOT / name).read_text()
        template = "`" + text.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${") + "`"
        require(template in html or any(json.dumps(text, ensure_ascii=value) in html for value in [False, True]),
                "Missing or stale bundled legal text: " + name)


def package(args):
    signed = args.command == "package"
    output = Path(args.output).resolve()
    require(not output.exists(), "Output already exists; choose a new directory")
    tag = os.environ.get("FLOAT_VIEW_VERSION", "")
    require(re.fullmatch(r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-rc\.[1-9]\d*)?", tag),
            "Store preparation requires vX.Y.Z or vX.Y.Z-rc.N; never nightly")
    version, number = run("node", "--strip-types", ".github/scripts/version.ts", "native", capture=True).split()
    commit = run("git", "rev-parse", "HEAD", capture=True)
    dirty = bool(run("git", "status", "--porcelain", capture=True))
    require(not signed or not dirty, "Commit source changes before signed store packaging")
    if os.environ.get("GITHUB_ACTIONS") == "true":
        require(signed and os.environ.get("GITHUB_REF") == "refs/tags/" + tag
                and os.environ.get("GITHUB_EVENT_NAME") in ["push", "workflow_dispatch"],
                "CI signing requires an explicit stable/candidate tag")
        record_path = os.environ.get("FLOAT_VIEW_BUILD_RECORD", "")
        require(record_path, "CI requires the shared build record")
        ci = json.loads(Path(record_path).read_text())
        require(ci.get("schema") == 1 and ci.get("commit") == commit and ci.get("tag") == tag
                and ci.get("appVersion") == version and ci.get("buildNumber") == number
                and ci.get("runId") == os.environ.get("GITHUB_RUN_ID")
                and ci.get("workflowPath") == ".github/workflows/build-apps.yml",
                "CI source/version/build number differs from shared build record")
    else:
        ci = None
    team = os.environ.get("APPLE_TEAM_ID", "")
    identity = os.environ.get("CODE_SIGN_IDENTITY", "")
    expected = plistlib.loads(ENTITLEMENTS.read_bytes())
    profile_bytes = None
    if signed:
        require(args.profile is not None, "Supply the Mac App Store profile")
        profile_bytes = Path(args.profile).read_bytes()
        profile = plistlib.loads(run("security", "cms", "-D", "-i", args.profile, capture=True).encode())
        expected = validate_profile(profile, team, identity)
        require(re.fullmatch(r"3rd Party Mac Developer Installer: .+ \(" + re.escape(team) + r"\)",
                             os.environ.get("MAC_INSTALLER_IDENTITY", "")), "Select the team's Mac Installer Distribution identity")
    tools = {"xcode": run("xcodebuild", "-version", capture=True),
             "macSDK": run("xcrun", "--sdk", "macosx", "--show-sdk-version", capture=True)}
    require(int(tools["macSDK"].split(".")[0]) >= 26 and re.search(r"Xcode (?:2[6-9]|[3-9]\d)\.", tools["xcode"]),
            "Mac store preparation requires Xcode/macOS SDK 26+")
    output.mkdir(parents=True)
    app = build_app(output, version, number, ["arm64", "x86_64"], STORE_ID, "Float View")
    if profile_bytes:
        (app / "Contents/embedded.provisionprofile").write_bytes(profile_bytes)
    entitlement_path = output / "entitlements.plist"
    entitlement_path.write_bytes(plistlib.dumps(expected))
    signing_keychain = os.environ.get("CODE_SIGN_KEYCHAIN") if signed else None
    run("codesign", "--force", "--sign", identity if signed else "-", "--entitlements", entitlement_path,
        *(["--keychain", signing_keychain] if signing_keychain else []),
        *(["--timestamp"] if signed else []), app)
    checks = inspect_app(app, version, number, ["arm64", "x86_64"], STORE_ID, expected)
    verify_notices(app)
    pkg = None
    if signed:
        run("codesign", "--verify", "--strict", "-R",
            '=anchor apple generic and certificate leaf[field.1.2.840.113635.100.6.1.4] exists'
            ' and certificate leaf[subject.OU] = "' + team + '"', app)
        pkg = output / "Float View.pkg"
        keychain = os.environ.get("MAC_INSTALLER_KEYCHAIN")
        run("productbuild", "--component", app, "/Applications", "--sign", os.environ["MAC_INSTALLER_IDENTITY"],
            *(["--keychain", keychain] if keychain else []), pkg)
        signature = run("pkgutil", "--check-signature", pkg, capture=True)
        require("Status: signed by a developer certificate issued by Apple" in signature
                and os.environ["MAC_INSTALLER_IDENTITY"] in signature, "Installer signer/chain mismatch")
        require(run("git", "rev-parse", "HEAD", capture=True) == commit
                and not run("git", "status", "--porcelain", capture=True), "Source changed during packaging")
    record = {"schema": 1, "purpose": "store" if signed else "unsigned-check", "commit": commit,
              "dirty": dirty, "version": version, "sourceVersion": tag, "buildNumber": number,
              "bundleIdentifier": STORE_ID, "checks": checks, "toolchain": tools,
              "package": pkg.name if pkg else None,
              "packageSHA256": hashlib.sha256(pkg.read_bytes()).hexdigest() if pkg else None,
              "ci": ci, "uploaded": False}
    (output / "package.json").write_text(json.dumps(record, indent=2) + "\n")
    print("Verified Mac package prepared; nothing uploaded: " + str(output))


def ci_build(args):
    require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
            and os.environ.get("RUNNER_OS") == "macOS", "CI profile import requires a hosted Mac runner")
    require(re.fullmatch(r"refs/tags/v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-rc\.[1-9]\d*)?",
                         os.environ.get("GITHUB_REF", "")), "CI store signing requires a stable/candidate tag")
    encoded = os.environ.pop("MAC_STORE_PROFILE_BASE64", "")
    require(encoded and len(encoded) <= 131072, "Missing/oversized Mac store profile")
    with tempfile.TemporaryDirectory(prefix="float-view-mac-profile-") as directory:
        path = Path(directory) / "store.provisionprofile"
        path.write_bytes(base64.b64decode("".join(encoded.split()), validate=True))
        path.chmod(0o600)
        package(argparse.Namespace(command="package", output=args.output, profile=path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    check = commands.add_parser("sandbox-check", help="Local ad-hoc sandboxed Release proof; no upload")
    check.add_argument("output")
    check.add_argument("--arch", choices=["universal", "native"], default="universal")
    for name in ["package-check", "package"]:
        command = commands.add_parser(name, help="Rebuild universal Release package; never uploads")
        command.add_argument("output")
        if name == "package":
            command.add_argument("--profile", required=True)
    ci = commands.add_parser("ci-build", help="Hosted-runner store packaging; no upload")
    ci.add_argument("output")
    args = parser.parse_args()
    try:
        if args.command == "ci-build":
            ci_build(args)
        else:
            sandbox_check(args) if args.command == "sandbox-check" else package(args)
    except ValueError as error:
        parser.exit(1, str(error) + "\n")
    except (KeyError, OSError, subprocess.CalledProcessError):
        parser.exit(1, "Mac packaging failed; inspect preceding tool diagnostics (command arguments suppressed).\n")


if __name__ == "__main__":
    main()
