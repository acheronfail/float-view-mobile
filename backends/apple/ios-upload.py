#!/usr/bin/env python3
"""Protected App Store Connect upload; never invites testers or submits review."""
import argparse
import base64
import contextlib
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

sys.dont_write_bytecode = True
APP_ID = "fail.acheron.floatview"


def require(value, message):
    if not value:
        raise ValueError(message)


def apple_tool(*arguments, env, timeout=60):
    # Capture output: Apple's JWT command prints its bearer token on stderr.
    result = subprocess.run(["xcrun", "altool", *arguments], env=env,
                            capture_output=True, timeout=timeout)
    require(result.returncode == 0, "Apple tool failed; inspect the app/build status before retrying")
    require(len(result.stdout) + len(result.stderr) <= 4_194_304, "Apple tool output exceeded bounds")
    return (result.stdout + result.stderr).decode(errors="replace")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, new_url):
        return None


class AppleAPI:
    def __init__(self, key, issuer, path):
        self.key, self.issuer, self.path = key, issuer, path
        self.env = dict(os.environ, API_PRIVATE_KEYS_DIR=str(path.parent))
        self.auth = ["--api-key", key, "--api-issuer", issuer, "--p8-file-path", str(path)]
        self.token = None
        self.expiry = 0

    def bearer(self):
        if self.expiry < time.time() + 60:
            output = apple_tool("--generate-jwt", *self.auth, env=self.env)
            tokens = set(re.findall(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+", output))
            require(len(tokens) == 1, "Apple tool did not return exactly one JWT")
            token = tokens.pop()
            claims = json.loads(base64.urlsafe_b64decode(token.split(".")[1] + "=="))
            require(claims.get("iss") == self.issuer and claims.get("aud") == "appstoreconnect-v1"
                    and time.time() + 60 < claims.get("exp", 0) <= time.time() + 1_300,
                    "Unexpected Apple JWT claims/lifetime")
            self.token, self.expiry = token, claims["exp"]
        return self.token

    def get(self, path, query=None):
        require(re.fullmatch(r"/v1/(?:apps/[0-9]+|builds)", path), "Unexpected API endpoint")
        url = "https://api.appstoreconnect.apple.com" + path
        if query:
            url += "?" + urllib.parse.urlencode(query)
        request = urllib.request.Request(url, headers={"Authorization": "Bearer " + self.bearer()})
        try:
            with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
                data = response.read(1_048_577)
                require(len(data) <= 1_048_576, "Apple API response exceeded bounds")
                return json.loads(data)
        except urllib.error.HTTPError as error:
            raise ValueError(f"Apple API returned HTTP {error.code}; no upload retry was attempted") from None

    def upload(self, ipa):
        apple_tool("--upload-app", "--file", str(ipa), "--type", "ios",
                   *self.auth, "--output-format", "json", env=self.env, timeout=1_200)


@contextlib.contextmanager
def credentials():
    key = os.environ.get("ASC_API_KEY_ID", "")
    issuer = os.environ.get("ASC_API_ISSUER_ID", "")
    encoded = os.environ.pop("ASC_API_PRIVATE_KEY_BASE64", "")
    require(re.fullmatch(r"[A-Z0-9]{10}", key), "Missing/invalid ASC_API_KEY_ID")
    require(re.fullmatch(r"[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}", issuer),
            "Missing/invalid ASC_API_ISSUER_ID")
    require(encoded and len(encoded) <= 8_192, "Missing/oversized API private key secret")
    with tempfile.TemporaryDirectory(prefix="float-view-asc-") as directory:
        path = Path(directory) / ("AuthKey_" + key + ".p8")
        value = base64.b64decode("".join(encoded.split()), validate=True)
        require(value.startswith(b"-----BEGIN PRIVATE KEY-----") and len(value) <= 4_096,
                "Expected a PKCS8 App Store Connect private key")
        path.write_bytes(value)
        path.chmod(0o600)
        yield AppleAPI(key, issuer, path)


def candidate(directory):
    record = json.loads((directory / "export.json").read_text())
    ci = record.get("ci", {})
    require(record.get("schema") == 1 and record.get("purpose") == "store"
            and record.get("configuration") == "Release" and record.get("dirty") is False
            and record.get("team") == os.environ.get("APPLE_TEAM_ID"),
            "Expected a clean verified Release store export")
    require(ci.get("commit") == record.get("commit") == os.environ.get("GITHUB_SHA")
            and ci.get("runId") == os.environ.get("GITHUB_RUN_ID")
            and ci.get("workflowPath") == ".github/workflows/build-apps.yml"
            and ci.get("buildJob") == "ios-store"
            and ci.get("appVersion") == record.get("version")
            and ci.get("buildNumber") == record.get("buildNumber") == os.environ.get("FLOAT_VIEW_BUILD_NUMBER")
            and ci.get("repository") == os.environ.get("GITHUB_REPOSITORY")
            and ((ci.get("channel") == "stable" and ci.get("tag") == "v" + record.get("version", ""))
                 or (ci.get("channel") == "candidate" and re.fullmatch(
                     "v" + re.escape(record.get("version", "")) + r"-rc\.[1-9]\d*", ci.get("tag", ""))))
            and os.environ.get("GITHUB_REF") == "refs/tags/" + ci.get("tag", ""),
            "Store export differs from the approved CI source/build/run")
    name = record.get("ipa", "")
    require(name == Path(name).name and name.endswith(".ipa"), "Invalid IPA filename")
    ipa = directory / name
    require(ipa.is_file() and not ipa.is_symlink()
            and hashlib.sha256(ipa.read_bytes()).hexdigest() == record.get("ipaSHA256"),
            "Store IPA changed since verification")
    return record, ipa


def verify_export(ipa, record):
    spec = importlib.util.spec_from_file_location("ios_store", Path(__file__).with_name("ios-store.py"))
    store = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(store)
    with tempfile.TemporaryDirectory(prefix="float-view-upload-check-") as directory:
        subprocess.run(["ditto", "-x", "-k", str(ipa), directory], check=True, timeout=60)
        apps = list((Path(directory) / "Payload").glob("*.app"))
        require(len(apps) == 1, "Expected exactly one exported app")
        store.verify_distribution(apps[0], record["version"], record["buildNumber"], record["team"])


def existing_build(api, app, record, *, platform="IOS"):
    response = api.get("/v1/builds", {"filter[app]": app, "filter[version]": record["buildNumber"],
                                   "filter[preReleaseVersion.platform]": platform,
                                   "include": "preReleaseVersion", "limit": "2"})
    values = response["data"]
    require(len(values) <= 1 and not response.get("links", {}).get("next"), "Ambiguous consumed build number")
    if not values:
        return None
    value = values[0]
    relation = value["relationships"]["preReleaseVersion"]["data"]["id"]
    versions = [v for v in response.get("included", [])
                if v.get("type") == "preReleaseVersions" and v.get("id") == relation]
    require(value["attributes"]["version"] == record["buildNumber"] and len(versions) == 1
            and versions[0]["attributes"]["version"] == record["version"]
            and versions[0]["attributes"]["platform"] == platform,
            "Apple build has a different marketing version/platform; use a fresh CI run")
    return {"id": value["id"], "processingState": value["attributes"]["processingState"]}


def write_receipt(path, receipt):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(receipt, indent=2) + "\n")
    temporary.replace(path)


def transfer(api, record, ipa, output, prior, *, sleep=time.sleep, polls=20, check_only=False,
             bundle_id=APP_ID, platform="IOS", hash_field="ipaSHA256"):
    app = os.environ.get("ASC_APP_ID", "")
    require(re.fullmatch(r"[1-9][0-9]{0,19}", app), "Set ASC_APP_ID to the numeric app record ID")
    response = api.get("/v1/apps/" + app)
    require(response["data"]["id"] == app and response["data"]["attributes"]["bundleId"] == bundle_id,
            "App Store Connect record does not match the approved bundle ID")
    output.mkdir(parents=True, exist_ok=True)
    receipt = {"schema": 1, "appId": app, "bundleId": bundle_id, "commit": record["commit"],
               "version": record["version"], "buildNumber": record["buildNumber"],
               hash_field: record[hash_field], "runId": record["ci"]["runId"],
               "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"), "status": "checked"}
    path = output / "upload-receipt.json"
    write_receipt(path, receipt)
    existing = existing_build(api, app, record, platform=platform)
    if check_only:
        receipt.update(status="checkedOnly", appleBuild=existing)
        write_receipt(path, receipt)
        return receipt
    if existing:
        receipt.update(status="alreadyReceived", appleBuild=existing)
        write_receipt(path, receipt)
        trusted = False
        for file in list(prior.rglob("upload-receipt.json"))[:100]:
            old = json.loads(file.read_text())
            trusted |= (old.get("transportAccepted") is True and all(old.get(k) == receipt[k]
                        for k in ["appId", "commit", "version", "buildNumber", hash_field, "runId"]))
        require(trusted, "Number already consumed without a matching prior transport receipt; inspect Apple and use a fresh run")
    else:
        require(os.environ.get("GITHUB_RUN_ATTEMPT") == "1",
                "An earlier attempt may have submitted this number; do not reupload while Apple visibility is uncertain. Use a fresh run")
        receipt["status"] = "uploadStarted"
        write_receipt(path, receipt)
        api.upload(ipa)  # Exactly one transport attempt. Never retry a failed/uncertain submission.
        receipt.update(status="transportAccepted", transportAccepted=True)
        write_receipt(path, receipt)
    for _ in range(polls):
        state = existing or existing_build(api, app, record, platform=platform)
        if state:
            receipt["appleBuild"] = state
            write_receipt(path, receipt)
            require(state["processingState"] not in ["FAILED", "INVALID"],
                    "Apple rejected this consumed number; inspect processing findings and use a fresh CI run")
            if state["processingState"] == "VALID":
                receipt["status"] = "processed"
                write_receipt(path, receipt)
                return receipt
        existing = None
        sleep(30)
    raise ValueError("Apple processing is pending/uncertain; receipt retained. Query before retrying; never blindly reupload")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("export", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--prior-receipts", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true", help="Verify auth/app/consumed number without uploading")
    args = parser.parse_args()
    try:
        require(os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted"
                and os.environ.get("RUNNER_OS") == "macOS"
                and re.fullmatch(r"refs/tags/v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-rc\.[1-9]\d*)?", os.environ.get("GITHUB_REF", ""))
                and os.environ.get("GITHUB_EVENT_NAME") in ["push", "workflow_dispatch"],
                "Apple delivery requires an approved hosted stable release tag or numbered candidate; never master or nightly")
        require(args.check_only or os.environ.get("IOS_STORE_UPLOAD_READY") == "true", "Upload readiness is not enabled")
        record, ipa = candidate(args.export)
        with credentials() as api:
            verify_export(ipa, record)
            transfer(api, record, ipa, args.output, args.prior_receipts, check_only=args.check_only)
        print("Read-only Apple app/build checks passed; no upload" if args.check_only else
              "Apple processed the recorded version/build; no testers invited or review submitted")
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
