"""Validate a CI Play bundle without uploading or changing publisher state."""
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

ABIS = {"arm64-v8a": (2, 183), "armeabi-v7a": (1, 40), "x86": (1, 3), "x86_64": (2, 62)}
ANDROID = "{http://schemas.android.com/apk/res/android}"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args):
    result = subprocess.run(args, text=True, capture_output=True)
    require(result.returncode == 0, f"{args[0]} validation failed: {result.stderr[:2000]}")
    return result.stdout


def elf_alignment(data, abi):
    require(data[:4] == b"\x7fELF" and data[5] == 1, "Expected little-endian ELF")
    elf_class, machine = ABIS[abi]
    require(data[4] == elf_class and struct.unpack_from("<H", data, 18)[0] == machine,
            f"Wrong ELF architecture for {abi}")
    wide = elf_class == 2
    offset = struct.unpack_from("<Q" if wide else "<I", data, 32 if wide else 28)[0]
    size, count = struct.unpack_from("<HH", data, 54 if wide else 42)
    require(size >= (56 if wide else 32) and count > 0 and offset + size * count <= len(data),
            "Invalid ELF program headers")
    alignments = []
    for index in range(count):
        header = offset + index * size
        if struct.unpack_from("<I", data, header)[0] != 1:
            continue
        alignment = struct.unpack_from("<Q" if wide else "<I", data, header + (48 if wide else 28))[0]
        require(alignment > 0 and alignment & (alignment - 1) == 0, "Invalid ELF alignment")
        if wide:
            require(alignment >= 16384, f"{abi} LOAD segment is not 16 KB aligned")
        alignments.append(alignment)
    require(alignments, "ELF has no LOAD segments")
    return alignments


def inspect_bundle(bundle):
    with zipfile.ZipFile(bundle) as archive:
        names = archive.namelist()
        require(len(names) == len(set(names)), "Duplicate bundle entries")
        require(archive.testzip() is None, "Corrupt bundle")
        expected = {f"base/lib/{abi}/libfloat_view_application_runtime_android_jni.so" for abi in ABIS}
        actual = {name for name in names if name.endswith(".so")}
        require(actual == expected, "Unexpected or missing native libraries")
        require("base/assets/index.html" in names, "Missing bundled frontend")
        require(not any(name.startswith(("base/assets/conformance/", "base/assets/fixtures/")) for name in names),
                "Test resources in bundle")
        return {abi: elf_alignment(archive.read(f"base/lib/{abi}/libfloat_view_application_runtime_android_jni.so"), abi)
                for abi in ABIS}


def verify_manifest(xml, version, number):
    root = ET.fromstring(xml)
    require(root.attrib.get("package") == "fail.acheron.floatview", "Wrong package")
    require(root.attrib.get(ANDROID + "versionName") == version.removeprefix("v"), "Wrong versionName")
    require(root.attrib.get(ANDROID + "versionCode") == str(number), "Wrong versionCode")
    application = root.find("application")
    require(application is not None and application.attrib.get(ANDROID + "debuggable", "false") == "false",
            "Debuggable store package")
    sdk = root.find("uses-sdk")
    require(sdk is not None and sdk.attrib.get(ANDROID + "targetSdkVersion") == "36", "Unexpected target SDK")


def identity(env):
    version = env.get("FLOAT_VIEW_VERSION", "")
    require(re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?", version),
            "Store bundles require a stable or candidate version")
    require(env.get("GITHUB_REF") == "refs/tags/" + version, "Store bundles require the exact release tag")
    repo = env.get("GITHUB_REPOSITORY", "")
    require(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo), "Invalid repository")
    require(env.get("GITHUB_WORKFLOW_REF", "").startswith(repo + "/.github/workflows/build-apps.yml@refs/"),
            "Unexpected workflow counter")
    for key in ("GITHUB_RUN_NUMBER", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT", "FLOAT_VIEW_BUILD_NUMBER"):
        require(re.fullmatch(r"[1-9][0-9]*", env.get(key, "")), "Invalid CI build identity")
    number = int(env["FLOAT_VIEW_BUILD_NUMBER"])
    require(number == int(env["GITHUB_RUN_NUMBER"]) + 60 and number <= 2100000000, "Wrong CI build number")
    commit = run("git", "rev-parse", "HEAD").strip()
    require(commit == env.get("GITHUB_SHA") == run("git", "rev-parse", version + "^{commit}").strip(), "Source/tag mismatch")
    return {"version": version, "buildNumber": number, "commit": commit,
            "runURL": f"https://github.com/{repo}/actions/runs/{env['GITHUB_RUN_ID']}",
            "attempt": env["GITHUB_RUN_ATTEMPT"]}


def main():
    bundle, bundletool, certificate, output = map(Path, sys.argv[1:])
    record = identity(os.environ)
    run("java", "-jar", str(bundletool), "validate", "--bundle=" + str(bundle))
    config = run("java", "-jar", str(bundletool), "dump", "config", "--bundle=" + str(bundle))
    require("PAGE_ALIGNMENT_16K" in config, "Bundle does not request 16 KB APK alignment")
    manifest = run("java", "-jar", str(bundletool), "dump", "manifest", "--bundle=" + str(bundle), "--module=base")
    verify_manifest(manifest, record["version"], record["buildNumber"])
    # Verify the actual JAR signer against the certificate exported from the selected upload key.
    signed_certificate = run("keytool", "-printcert", "-jarfile", str(bundle), "-rfc")
    import ssl
    pem = re.findall(r"-----BEGIN CERTIFICATE-----.*?-----END CERTIFICATE-----", signed_certificate, re.S)
    require(len(pem) == 1, "Expected exactly one bundle signer")
    signer = ssl.PEM_cert_to_DER_cert(pem[0])
    require(signer == certificate.read_bytes(), "Bundle signer differs from upload key")
    fingerprint = hashlib.sha256(signer).hexdigest()
    require(fingerprint == os.environ.get("ANDROID_UPLOAD_CERT_SHA256", "").lower(),
            "Upload key differs from the certificate registered with Play")
    record.update(schema=1, packageId="fail.acheron.floatview", configuration="Release",
                  bundleSha256=hashlib.sha256(bundle.read_bytes()).hexdigest(),
                  uploadCertificateSha256=fingerprint,
                  nativeLoadAlignments=inspect_bundle(bundle), apkPageAlignment="16K", uploaded=False)
    output.write_text(json.dumps(record, indent=2) + "\n")
    print("Signed Play bundle validated; no Google upload performed.")


if __name__ == "__main__":
    main()
