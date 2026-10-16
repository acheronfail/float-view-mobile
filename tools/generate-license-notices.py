#!/usr/bin/env python3
"""Collect exact dependency notices from installed, locked build inputs.

Run after installing frontend/native dependencies. Android's resolved Release
classpath report is supplied explicitly so this script never starts a Gradle
daemon or downloads dependencies. See docs/licensing.md.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
TARGETS = (
    "aarch64-apple-ios", "aarch64-apple-darwin", "x86_64-apple-darwin",
    "aarch64-linux-android", "armv7-linux-androideabi", "i686-linux-android",
    "x86_64-linux-android",
)
FRONTEND = ("svelte", "@sveltejs/kit", "clsx", "devalue", "esm-env", "leaflet")


def generate(android_report):
    documents = {}
    entries = []

    def document(text):
        text = text.replace("\r\n", "\n").strip()
        key = hashlib.sha256(text.encode()).hexdigest()[:16]
        documents[key] = text
        return key

    def license_files(folder):
        return [p for p in sorted(folder.iterdir()) if p.is_file() and
                p.name.upper().startswith(("LICENSE", "COPYING", "NOTICE", "UNLICENSE"))]

    packages = {}
    for target in TARGETS:
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
             "--filter-platform", target], cwd=ROOT / "backends/core", text=True))
        by_id = {p["id"]: p for p in metadata["packages"]}
        nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
        suffix = "android-jni" if "android" in target else "apple-ffi"
        pending = [p["id"] for p in metadata["packages"]
                   if p["name"] == "float-view-application-runtime-" + suffix]
        if len(pending) != 1:
            raise ValueError(f"Missing native root for {target}")
        seen = set()
        while pending:
            package_id = pending.pop()
            if package_id in seen:
                continue
            seen.add(package_id)
            package = by_id[package_id]
            if package["source"]:
                packages[package_id] = package
            for dep in nodes[package_id]["deps"]:
                if any(kind["kind"] != "dev" for kind in dep["dep_kinds"]):
                    pending.append(dep["pkg"])

    for package in sorted(packages.values(), key=lambda p: (p["name"], p["version"])):
        if not package["license"]:
            raise ValueError(f"Missing licence for {package['name']}")
        files = license_files(Path(package["manifest_path"]).parent)
        refs = [(p.name, document(p.read_text())) for p in files]
        # This proc macro declares the standard dual licence but omits licence
        # files in its published crate. Keep its declaration, not invented credit.
        if not refs and package["name"] != "jni-sys-macros":
            raise ValueError(f"No licence text for {package['name']}")
        entries.append((f"Rust: {package['name']} {package['version']}", package["license"],
                        f"https://crates.io/crates/{package['name']}/{package['version']}", refs))

    for name in FRONTEND:
        folder = ROOT / "frontend/node_modules" / name
        package = json.loads((folder / "package.json").read_text())
        lock = json.loads((ROOT / "frontend/package-lock.json").read_text())
        if package["version"] != lock["packages"]["node_modules/" + name]["version"]:
            raise ValueError(f"Installed package differs from lock: {name}")
        files = license_files(folder)
        if not files:
            raise ValueError(f"No licence text for {name}")
        entries.append((f"Frontend: {name} {package['version']}", package["license"],
                        f"https://www.npmjs.com/package/{name}/v/{package['version']}",
                        [(p.name, document(p.read_text())) for p in files]))

    icons = json.loads((ROOT / "frontend/node_modules/@iconify-json/lucide/package.json").read_text())
    entries.append((f"Generated icons: @iconify-json/lucide {icons['version']}", "ISC; MIT for Feather-derived icons",
                    "https://github.com/lucide-icons/lucide/blob/main/LICENSE",
                    [("Lucide and Feather notices", document((ROOT / "LICENSES/Lucide.txt").read_text()))]))
    entries.append(("Rider artwork: onewheel by Luis Prado, Noun Project 4260841", "CC-BY-3.0",
                    "https://thenounproject.com/icon/onewheel-4260841/",
                    [("Attribution and modifications", document((ROOT / "NOTICE").read_text())),
                     ("CC BY 3.0", document((ROOT / "LICENSES/CC-BY-3.0.txt").read_text()))]))
    sqlite = next(p for p in packages.values() if p["name"] == "libsqlite3-sys")
    amalgamation = Path(sqlite["manifest_path"]).parent / "sqlite3/sqlite3.c"
    sqlite_version = re.search(r"version ([\d.]+)\.", amalgamation.read_text()[:1000]).group(1)
    entries.append((f"Bundled SQLite {sqlite_version} (libsqlite3-sys)", "Public domain",
                    "https://sqlite.org/copyright.html", []))

    cache = Path(os.environ.get("GRADLE_USER_HOME", Path.home() / ".gradle")) / "caches/modules-2/files-2.1"
    coordinates = set()
    for line in android_report.read_text().splitlines():
        match = re.search(r"([\w.\-]+):([\w.\-]+):([\w.\-]+)(?: -> ([\w.\-]+))?", line)
        if match:
            coordinates.add((match[1], match[2], match[4] or match[3]))
    if "releaseRuntimeClasspath" not in android_report.read_text() or not coordinates:
        raise ValueError("Expected a resolved Android releaseRuntimeClasspath report")
    if "FAILED" in android_report.read_text():
        raise ValueError("Android classpath contains unresolved dependencies")
    ns = {"m": "http://maven.apache.org/POM/4.0.0"}

    def pom_licenses(group, artifact, version, depth=0):
        if depth > 8:
            raise ValueError("POM parent depth exceeded")
        files = sorted((cache / group / artifact / version).glob("*/*.pom"))
        if not files:
            raise ValueError(f"Missing cached POM: {group}:{artifact}:{version}")
        pom = ET.parse(files[0]).getroot()
        licenses = pom.findall("m:licenses/m:license", ns)
        if licenses:
            return [p.findtext("m:name", namespaces=ns) for p in licenses]
        parent = pom.find("m:parent", ns)
        if parent is None:
            raise ValueError(f"Missing licence: {group}:{artifact}:{version}")
        return pom_licenses(*(parent.findtext("m:" + key, namespaces=ns)
                              for key in ("groupId", "artifactId", "version")), depth + 1)

    apache = next(key for key, value in documents.items()
                  if "Apache License" in value and "Version 2.0, January 2004" in value)
    for group, artifact, version in sorted(coordinates):
        licenses = pom_licenses(group, artifact, version)
        if not all("Apache" in value and "2.0" in value for value in licenses):
            raise ValueError(f"Review new Android licence: {group}:{artifact}:{version}")
        entries.append((f"Android: {group}:{artifact}:{version}", "Apache-2.0",
                        f"https://repo1.maven.org/maven2/{group.replace('.', '/')}/{artifact}/{version}/",
                        [("Apache License 2.0", apache)]))

    header = """Float View — third-party notices

Generated by tools/generate-license-notices.py from locked/cached inputs.
This conservative inventory includes native build dependencies and procedural
macros. Dependency licences remain independent of Float View's AGPL licence.
Standard MIT/Apache texts below also cover declarations without a separate
licence file. Preserve component-specific and embedded third-party notices.
Android entries include metadata/BOM coordinates; see docs/licensing.md for
scope, source-delivery and final-package verification requirements.

"""
    sections = [header]
    for name, license_name, source, refs in entries:
        sections.append(f"{name}\nLicence: {license_name}\nSource: {source}\n" +
                        "".join(f"  {label}: document {key}\n" for label, key in refs) + "\n")
    for key, value in sorted(documents.items()):
        sections.append(f"{'=' * 72}\nDocument {key}\n{'=' * 72}\n{value}\n\n")
    return "".join(sections).rstrip() + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--android-report", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = generate(args.android_report)
    destination = ROOT / "THIRD_PARTY_NOTICES.txt"
    if args.check:
        if destination.read_text() != result:
            raise SystemExit("Notices differ; regenerate and review the licence changes")
        print("Third-party notices match the installed locked inputs")
    else:
        destination.write_text(result)
        print(f"Wrote {destination.name}")
