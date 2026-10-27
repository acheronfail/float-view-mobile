"""Deliver an approved, verified CI bundle to Play internal testing only."""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import ssl
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

PACKAGE = "fail.acheron.floatview"
BASE = "https://androidpublisher.googleapis.com/androidpublisher/v3/applications/" + PACKAGE + "/edits"
TOKEN_URL = "https://oauth2.googleapis.com/token"


def require(value, message):
    if not value:
        raise ValueError(message)


def command(*args, data=None):
    result = subprocess.run(args, input=data, capture_output=True, timeout=120)
    require(result.returncode == 0, f"{args[0]} validation failed")
    return result.stdout


class GoogleError(ValueError):
    def __init__(self, code):
        self.code = code
        super().__init__(f"Google API HTTP {code}; inspect the saved receipt before retrying")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, new_url):
        return None


def request(method, url, data=None, token=None, content_type="application/json"):
    headers = {"Content-Type": content_type}
    if hasattr(data, "fileno"):
        headers["Content-Length"] = str(os.fstat(data.fileno()).st_size)
    if token:
        headers["Authorization"] = "Bearer " + token
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.build_opener(NoRedirect).open(req, timeout=180) as response:
            body = response.read(4_194_305)
            require(len(body) <= 4_194_304, "Google response exceeds bounds")
            return json.loads(body) if body else {}
    except urllib.error.HTTPError as error:
        # Never expose credential responses, bearer tokens or unbounded server text.
        raise GoogleError(error.code) from None
    except (OSError, TimeoutError):
        raise ValueError("Google request outcome uncertain; inspect the saved receipt before retrying") from None


def authenticate():
    raw = os.environ.pop("GOOGLE_PLAY_SERVICE_ACCOUNT_JSON", "")
    require(0 < len(raw) <= 32768, "Missing/oversized GOOGLE_PLAY_SERVICE_ACCOUNT_JSON")
    credential = json.loads(raw)
    require(credential.get("type") == "service_account"
            and credential.get("token_uri") == TOKEN_URL
            and re.fullmatch(r"[A-Za-z0-9_.-]+@[A-Za-z0-9.-]+\.iam\.gserviceaccount\.com", credential.get("client_email", "")),
            "Expected a Google service-account JSON key")
    key = credential.get("private_key", "")
    require(key.startswith("-----BEGIN PRIVATE KEY-----") and len(key) < 8192, "Invalid service-account private key")
    def encode(value):
        return base64.urlsafe_b64encode(json.dumps(value, separators=(",", ":")).encode()).rstrip(b"=")
    now = int(time.time())
    unsigned = encode({"alg": "RS256", "typ": "JWT"}) + b"." + encode({
        "iss": credential["client_email"], "scope": "https://www.googleapis.com/auth/androidpublisher",
        "aud": TOKEN_URL, "iat": now, "exp": now + 1800})
    with tempfile.TemporaryDirectory(prefix="float-view-google-") as folder:
        path = Path(folder) / "key.pem"
        path.touch(mode=0o600)
        path.write_text(key)
        signed = command("openssl", "dgst", "-sha256", "-sign", str(path), data=unsigned)
    assertion = (unsigned + b"." + base64.urlsafe_b64encode(signed).rstrip(b"=")).decode()
    result = request("POST", TOKEN_URL, urllib.parse.urlencode({
        "grant_type": "urn:ietf:params:oauth:grant-type:jwt-bearer", "assertion": assertion}).encode(),
        content_type="application/x-www-form-urlencoded")
    token = result.get("access_token", "")
    require(isinstance(token, str) and 0 < len(token) < 8192, "Google returned no valid access token")
    return token


class PlayAPI:
    def __init__(self, token):
        self.token = token

    def call(self, method, suffix="", body=None):
        require(re.fullmatch(r"(?:/[A-Za-z0-9_-]+(?:/bundles|/tracks/internal|:commit\?changesInReviewBehavior=ERROR_IF_IN_REVIEW)?)?", suffix),
                "Unexpected Google endpoint")
        data = json.dumps(body).encode() if body is not None else (b"" if method == "POST" else None)
        return request(method, BASE + suffix, data, self.token)

    def upload(self, edit, bundle):
        url = BASE.replace("/androidpublisher/v3/", "/upload/androidpublisher/v3/") + "/" + edit + "/bundles?uploadType=media"
        with bundle.open("rb") as stream:
            return request("POST", url, stream, self.token, "application/octet-stream")


def candidate(directory):
    spec = importlib.util.spec_from_file_location("android_store", Path(__file__).with_name("android-store.py"))
    store = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(store)
    identity = store.identity(os.environ)
    record = json.loads((directory / "package.json").read_text())
    require(record.get("schema") == 1 and record.get("configuration") == "Release"
            and record.get("packageId") == PACKAGE and record.get("uploaded") is False,
            "Expected a verified Release Play package")
    for field in ("version", "buildNumber", "commit", "runURL"):
        require(record.get(field) == identity[field], "Bundle differs from approved CI identity: " + field)
    require(str(record.get("attempt", "")).isdigit()
            and 1 <= int(record["attempt"]) <= int(identity["attempt"]), "Invalid signing attempt")
    bundle = directory / ("FloatView-" + identity["version"] + ".aab")
    require(bundle.is_file() and 0 < bundle.stat().st_size <= 536870912, "Missing/oversized AAB")
    with bundle.open("rb") as stream:
        require(hashlib.file_digest(stream, "sha256").hexdigest() == record.get("bundleSha256"), "AAB hash mismatch")
    certificate = command("keytool", "-printcert", "-jarfile", str(bundle), "-rfc").decode()
    pems = re.findall(r"-----BEGIN CERTIFICATE-----.*?-----END CERTIFICATE-----", certificate, re.S)
    require(len(pems) == 1, "Expected exactly one upload certificate")
    fingerprint = hashlib.sha256(ssl.PEM_cert_to_DER_cert(pems[0])).hexdigest()
    require(fingerprint == record.get("uploadCertificateSha256") == os.environ.get("ANDROID_UPLOAD_CERT_SHA256"),
            "Bundle signer does not match registered upload certificate")
    with tempfile.TemporaryDirectory(prefix="float-view-verify-") as folder:
        pem = Path(folder) / "certificate.pem"
        pem.write_text(pems[0])
        trust = Path(folder) / "trust.p12"
        command("keytool", "-importcert", "-noprompt", "-alias", "upload", "-file", str(pem),
                "-keystore", str(trust), "-storepass", "public-certificate-only")
        command("jarsigner", "-verify", "-strict", "-keystore", str(trust), "-storepass", "public-certificate-only", str(bundle))
    return record, bundle


def remote_bundle(api, edit, record):
    matches = [b for b in api.call("GET", "/" + edit + "/bundles").get("bundles", [])
               if str(b.get("versionCode")) == str(record["buildNumber"])]
    require(len(matches) <= 1, "Duplicate remote version code")
    if matches:
        require(matches[0].get("sha256", "").lower() == record["bundleSha256"],
                "Google version code is occupied by a different bundle; use a fresh run")
    return bool(matches)


def deliver(api, record, bundle, output, prior, check_only=False):
    output.mkdir(parents=True, exist_ok=True)
    receipt = {k: record[k] for k in ("version", "buildNumber", "commit", "runURL", "bundleSha256")}
    receipt.update(schema=1, packageId=PACKAGE, track="internal", attempt=os.environ["GITHUB_RUN_ATTEMPT"], status="checking")
    def save(status, edit=None):
        receipt["status"] = status
        if edit:
            receipt["editId"] = edit
        (output / "upload-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    previous = []
    if prior.exists():
        for path in sorted(prior.rglob("upload-receipt.json")):
            require(path.stat().st_size <= 16384, "Oversized earlier receipt")
            old = json.loads(path.read_text())
            if all(old.get(k) == receipt[k] for k in ("packageId", "track", "version", "buildNumber", "commit", "runURL", "bundleSha256")):
                previous.append(old)
    attempt = int(os.environ["GITHUB_RUN_ATTEMPT"])
    # Only a complete history of explicit pre-upload receipts proves that a
    # retry can send bytes safely. Legacy/missing receipts remain uncertain.
    prior_attempts = {int(old["attempt"]) for old in previous}
    never_uploaded = (prior_attempts == set(range(1, attempt))
                      and all(old.get("uploadMayHaveStarted") is False for old in previous))
    receipt["uploadMayHaveStarted"] = not never_uploaded
    save("checking")
    edit = None
    resumed = False
    # Google permits only one edit per account/app. Inspect the previous edit
    # BEFORE creating another, which would invalidate the pending upload.
    inspectable = [old for old in previous if old.get("editId")]
    if inspectable:
        old = max(inspectable, key=lambda r: int(r["attempt"]))
        old_edit = old.get("editId", "")
        if old_edit:
            require(re.fullmatch(r"[A-Za-z0-9_-]+", old_edit), "Invalid earlier Google edit ID")
            save("checkingPriorEdit", old_edit)
            try:
                api.call("GET", "/" + old_edit)
                edit, resumed = old_edit, True
            except GoogleError as error:
                if error.code != 404:
                    raise
    if edit is None:
        edit = api.call("POST", "", {})["id"]
    require(isinstance(edit, str) and re.fullmatch(r"[A-Za-z0-9_-]+", edit), "Invalid Google edit ID")
    present = remote_bundle(api, edit, record)
    track = api.call("GET", "/" + edit + "/tracks/internal")
    codes = [int(code) for release in track.get("releases", []) for code in release.get("versionCodes", [])]
    require(not codes or max(codes) <= record["buildNumber"], "A newer internal build exists; refusing to roll back")
    if check_only:
        if not resumed:
            api.call("DELETE", "/" + edit)
        save("checkedOnly", edit if resumed else None)
        return receipt
    completed = any(str(record["buildNumber"]) in release.get("versionCodes", [])
                    and release.get("status") == "completed" for release in track.get("releases", []))
    if present and completed and not resumed:
        api.call("DELETE", "/" + edit)
        save("alreadyDelivered")
        return receipt
    if not present and attempt > 1 and not never_uploaded:
        require(previous, "No matching earlier receipt; check Google or use a fresh run, never blind re-upload")
        raise ValueError("Earlier upload is not observable; inspect Google and use a fresh run")
    if not present:
        receipt["uploadMayHaveStarted"] = True
        save("uploadStarted", edit)
        uploaded = api.upload(edit, bundle)
        require(str(uploaded.get("versionCode")) == str(record["buildNumber"])
                and uploaded.get("sha256", "").lower() == record["bundleSha256"], "Google upload hash/version mismatch")
    save("bundleVerified", edit)
    api.call("PUT", "/" + edit + "/tracks/internal", {"track": "internal", "releases": [{
        "name": record["version"] + " (" + str(record["buildNumber"]) + ")",
        "versionCodes": [str(record["buildNumber"])], "status": "completed"}]})
    save("commitStarted", edit)
    api.call("POST", "/" + edit + ":commit?changesInReviewBehavior=ERROR_IF_IN_REVIEW")
    save("committed", edit)
    # Confirm committed state in a fresh snapshot, not just the previous edit.
    verify = api.call("POST", "", {})["id"]
    verified = remote_bundle(api, verify, record)
    live = api.call("GET", "/" + verify + "/tracks/internal")
    require(verified and any(str(record["buildNumber"]) in release.get("versionCodes", [])
                            and release.get("status") == "completed" for release in live.get("releases", [])),
            "Internal delivery not yet confirmed; inspect Google before retrying")
    api.call("DELETE", "/" + verify)
    save("delivered", edit)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--prior-receipts", type=Path, required=True)
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    require(os.environ.get("GOOGLE_PLAY_UPLOAD_READY") == "true", "Google upload readiness is disabled")
    record, bundle = candidate(args.directory)
    receipt = deliver(PlayAPI(authenticate()), record, bundle, args.output, args.prior_receipts, args.check_only)
    print("Google internal testing: " + receipt["status"] + "; build " + str(record["buildNumber"]))


if __name__ == "__main__":
    main()
