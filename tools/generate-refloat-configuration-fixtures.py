#!/usr/bin/env python3
"""Regenerate immutable Refloat ConfigParams fixtures from audited vendor tags."""

from __future__ import annotations

import base64
import hashlib
import json
import re
import shutil
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REFLOAT = ROOT / "vendor" / "refloat"
OUTPUT = ROOT / "backends" / "fixtures" / "refloat-configuration"
VERSIONS = (
    "1.0.0",
    "1.0.1",
    "1.1.0",
    "1.1.1",
    "1.1.2",
    "1.2.0",
    "1.2.1",
    "1.2.2",
    "1.2.3",
    "1.3.0",
)
SOURCE_PATH = "src/conf/settings.xml"
MAX_COMPRESSED_BYTES = 32 * 1024
MAX_EXPANDED_BYTES = 384 * 1024


def run(*args: str) -> bytes:
    return subprocess.check_output(args, cwd=ROOT)


def crc32c(data: bytes) -> int:
    crc = 0xFFFFFFFF
    for value in data:
        crc ^= value
        for _ in range(8):
            crc = (crc >> 1) ^ (0x82F63B78 if crc & 1 else 0)
    return (~crc) & 0xFFFFFFFF


def text(element: ET.Element, name: str, default: str = "") -> str:
    child = element.find(name)
    return default if child is None or child.text is None else child.text


def integer(element: ET.Element, name: str, default: int = 0) -> int:
    return int(text(element, name, str(default)))


def decimal(element: ET.Element, name: str, default: float = 0.0) -> float:
    return float(text(element, name, str(default)))


def parse_schema(xml: bytes) -> dict:
    root = ET.fromstring(xml)
    if root.tag != "ConfigParams":
        raise ValueError("unexpected root")
    params_element = root.find("Params")
    order_element = root.find("SerOrder")
    grouping_element = root.find("Grouping")
    if params_element is None or order_element is None or grouping_element is None:
        raise ValueError("missing schema section")

    params: dict[str, dict] = {}
    for element in params_element:
        kind = integer(element, "type")
        item = {
            "name": element.tag,
            "longName": text(element, "longName"),
            "type": kind,
            "transmittable": integer(element, "transmittable"),
            "description": text(element, "description"),
            "cDefine": text(element, "cDefine"),
        }
        if kind == 0:
            pass
        elif kind == 1:
            item.update(
                editorDecimals=integer(element, "editorDecimalsDouble"),
                editorScale=decimal(element, "editorScale", 1),
                editAsPercentage=integer(element, "editAsPercentage"),
                maximum=decimal(element, "maxDouble"),
                minimum=decimal(element, "minDouble"),
                step=decimal(element, "stepDouble"),
                default=decimal(element, "valDouble"),
                wireScale=decimal(element, "vTxDoubleScale"),
                suffix=text(element, "suffix"),
                wireType=integer(element, "vTx"),
            )
        elif kind == 2:
            item.update(
                editorScale=decimal(element, "editorScale", 1),
                editAsPercentage=integer(element, "editAsPercentage"),
                maximum=integer(element, "maxInt"),
                minimum=integer(element, "minInt"),
                step=integer(element, "stepInt"),
                default=integer(element, "valInt"),
                suffix=text(element, "suffix"),
                wireType=integer(element, "vTx"),
            )
        elif kind == 3:
            item.update(default=text(element, "valString"), maximumLength=integer(element, "maxLen"))
        elif kind in (4, 6):
            item.update(default=integer(element, "valInt"), choices=[value.text or "" for value in element.findall("enumNames")])
        elif kind == 5:
            item.update(default=integer(element, "valInt"))
        else:
            raise ValueError(f"unsupported type {kind} for {element.tag}")
        params[element.tag] = item

    order = [element.text or "" for element in order_element.findall("ser")]
    if len(order) != len(set(order)) or any(name not in params for name in order):
        raise ValueError("invalid serialization order")
    groups = []
    for group in grouping_element.findall("group"):
        subgroups = []
        for subgroup in group.findall("subgroup"):
            entries = [element.text or "" for element in subgroup.findall("subgroupParams/param")]
            subgroups.append({"name": text(subgroup, "subgroupName"), "entries": entries})
        groups.append({"name": text(group, "groupName"), "subgroups": subgroups})
    fields = [params[name] for name in order]
    canonical = {"parameters": list(params.values()), "serializationOrder": order, "groups": groups}
    canonical_bytes = json.dumps(canonical, ensure_ascii=False, separators=(",", ":")).encode()
    return {
        "params": params,
        "fields": fields,
        "order": order,
        "groups": groups,
        "canonicalSha256": hashlib.sha256(canonical_bytes).hexdigest(),
    }


def signature(schema: dict) -> int:
    value = ""
    for field in schema["fields"]:
        value += field["name"] + str(field["type"]) + str(field.get("wireType", 0))
        value += "".join(field.get("choices", []))
    return crc32c(value.encode())


def encode_values(schema: dict, values: dict[str, object], wire_signature: int) -> bytes:
    output = bytearray(struct.pack(">I", wire_signature))
    for field in schema["fields"]:
        value = values[field["name"]]
        kind = field["type"]
        if kind == 1:
            scaled = round(float(value) * field["wireScale"])
            output.extend(struct.pack(">h" if field["wireType"] == 7 else ">i", scaled))
        elif kind == 2:
            formats = {1: ">B", 2: ">b", 3: ">H", 4: ">h", 5: ">I", 6: ">i"}
            output.extend(struct.pack(formats[field["wireType"]], int(value)))
        elif kind == 3:
            output.extend(str(value).encode() + b"\0")
        elif kind in (4, 5, 6):
            output.extend(struct.pack(">b", int(value)))
        else:
            raise ValueError(f"unsupported type {kind}")
    return bytes(output)


def generated_schema(xml: bytes) -> bytes:
    tool = shutil.which("vesc_tool")
    if tool is None:
        raise RuntimeError("vesc_tool is required to reproduce Qt qCompress fixtures")
    with tempfile.TemporaryDirectory(prefix="refloat-config-") as directory:
        path = Path(directory) / "settings.xml"
        path.write_bytes(xml)
        subprocess.run(
            [tool, "--offscreen", "--xmlConfToCode", str(path)],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        source = (path.parent / "confxml.c").read_text()
    values = bytes(int(value, 16) for value in re.findall(r"0x([0-9a-fA-F]{2})", source))
    if not values or len(values) > MAX_COMPRESSED_BYTES:
        raise ValueError("invalid generated compressed schema size")
    if int.from_bytes(values[:4], "big") > MAX_EXPANDED_BYTES:
        raise ValueError("invalid generated expanded schema size")
    return values


def hex_bytes(*values: int) -> str:
    return bytes(values).hex()


def main() -> None:
    schemas = OUTPUT / "schemas"
    values_dir = OUTPUT / "values"
    schemas.mkdir(parents=True, exist_ok=True)
    values_dir.mkdir(parents=True, exist_ok=True)
    releases = []
    for version in VERSIONS:
        tag = f"v{version}"
        xml = run("git", "-C", str(REFLOAT), "show", f"{tag}:{SOURCE_PATH}")
        commit = run("git", "-C", str(REFLOAT), "rev-list", "-n", "1", tag).decode().strip()
        schema = parse_schema(xml)
        compressed = generated_schema(xml)
        canonical_expanded = zlib.decompress(compressed[4:])
        if len(canonical_expanded) != int.from_bytes(compressed[:4], "big"):
            raise ValueError("generated Qt envelope length mismatch")
        wire_signature = signature(schema)
        defaults = {field["name"]: field["default"] for field in schema["fields"]}
        representative = dict(defaults)
        changed = next(field for field in schema["fields"] if field["type"] == 1 and field["name"] != "meta.is_default")
        representative[changed["name"]] = min(changed["maximum"], changed["default"] + changed["step"])
        normalized = dict(representative)
        if version >= "1.1.0" and "meta.is_default" in normalized:
            normalized["meta.is_default"] = 0
        default_payload = encode_values(schema, defaults, wire_signature)
        current_payload = encode_values(schema, defaults, wire_signature)
        write_payload = encode_values(schema, representative, wire_signature)
        readback_payload = encode_values(schema, normalized, wire_signature)
        (schemas / f"{version}.qcompress").write_bytes(compressed)
        (values_dir / f"{version}-default.bin").write_bytes(default_payload)
        (values_dir / f"{version}-current.bin").write_bytes(current_payload)
        (values_dir / f"{version}-write.bin").write_bytes(write_payload)
        (values_dir / f"{version}-readback.bin").write_bytes(readback_payload)
        (values_dir / f"{version}.json").write_text(
            json.dumps(
                {"default": defaults, "representativeCurrent": defaults, "acceptedWrite": representative, "expectedReadback": normalized},
                ensure_ascii=False,
                separators=(",", ":"),
            )
            + "\n"
        )
        releases.append(
            {
                "version": version,
                "tag": tag,
                "commit": commit,
                "sourcePath": SOURCE_PATH,
                "sourceSha256": hashlib.sha256(xml).hexdigest(),
                "compressedBytes": len(compressed),
                "expandedBytes": int.from_bytes(compressed[:4], "big"),
                "compressedSha256": hashlib.sha256(compressed).hexdigest(),
                "schemaFingerprintSha256": hashlib.sha256(canonical_expanded).hexdigest(),
                "normalizedExpectationSha256": schema["canonicalSha256"],
                "wireSignature": wire_signature,
                "wireSignatureHex": f"{wire_signature:08x}",
                "parameterCount": len(schema["params"]),
                "serializedFieldCount": len(schema["fields"]),
                "groupCount": len(schema["groups"]),
                "defaultPayloadBytes": len(default_payload),
                "defaultPayloadSha256": hashlib.sha256(default_payload).hexdigest(),
                "representativeWrite": {"field": changed["name"], "value": representative[changed["name"]]},
                "writeNormalization": [] if version < "1.1.0" else [{"field": "meta.is_default", "value": 0}],
                "files": {
                    "schema": f"schemas/{version}.qcompress",
                    "values": f"values/{version}.json",
                    "default": f"values/{version}-default.bin",
                    "current": f"values/{version}-current.bin",
                    "acceptedWrite": f"values/{version}-write.bin",
                    "expectedReadback": f"values/{version}-readback.bin",
                },
            }
        )

    legacy = releases[:2]
    if legacy[0]["wireSignature"] != legacy[1]["wireSignature"] or legacy[0]["schemaFingerprintSha256"] == legacy[1]["schemaFingerprintSha256"]:
        raise ValueError("legacy identity collision proof failed")
    (OUTPUT / "manifest.json").write_text(
        json.dumps(
            {
                "schemaVersion": 1,
                "releases": releases,
                "identityCases": [
                    {
                        "name": "legacy-info-and-signature-collision",
                        "reportedVersion": "1.0",
                        "versions": ["1.0.0", "1.0.1"],
                        "wireSignatureHex": legacy[0]["wireSignatureHex"],
                        "distinctSchemaFingerprints": [item["schemaFingerprintSha256"] for item in legacy],
                    },
                    {
                        "name": "reported-patch-requires-audited-fingerprint",
                        "versions": list(VERSIONS[2:]),
                        "rule": "reported exact version and its audited schema fingerprint must both match",
                    },
                ],
            },
            indent=2,
        )
        + "\n"
    )
    malformed = {
        "schemaVersion": 1,
        "cases": [
            {"name": "truncated-qt-envelope", "payloadBase64": base64.b64encode(b"\0\0").decode(), "error": "truncatedEnvelope"},
            {"name": "expansion-excess", "payloadBase64": base64.b64encode(struct.pack(">I", MAX_EXPANDED_BYTES + 1) + b"x").decode(), "error": "expandedTooLarge"},
            {"name": "invalid-zlib", "payloadBase64": base64.b64encode(struct.pack(">I", 8) + b"not-zlib").decode(), "error": "decompressionFailed"},
            {"name": "expanded-length-mismatch", "payloadBase64": base64.b64encode(struct.pack(">I", 99) + b"x\x9c\x03\x00\x00\x00\x00\x01").decode(), "error": "expandedLengthMismatch"},
            {"name": "dtd", "xml": "<?xml version=\"1.0\"?><!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><ConfigParams>&e;</ConfigParams>", "error": "prohibitedXml"},
            {"name": "excessive-nesting", "xml": "<ConfigParams>" + "<x>" * 40 + "</x>" * 40 + "</ConfigParams>", "error": "structuralLimit"},
            {"name": "excessive-field-count", "xmlTemplate": "ConfigParams with 513 distinct Params children", "error": "fieldLimit"},
            {"name": "excessive-string", "xmlTemplate": "ConfigParams field name or metadata string with 4097 UTF-8 bytes", "error": "stringLimit"},
            {"name": "duplicate-field", "xml": "<ConfigParams><Params><x/><x/></Params><SerOrder/><Grouping/></ConfigParams>", "error": "duplicateField"},
            {"name": "duplicate-order", "xml": "<ConfigParams><Params><x><type>5</type></x></Params><SerOrder><ser>x</ser><ser>x</ser></SerOrder><Grouping/></ConfigParams>", "error": "duplicateOrder"},
            {"name": "missing-reference", "xml": "<ConfigParams><Params/><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>", "error": "missingReference"},
            {"name": "unsupported-type", "xml": "<ConfigParams><Params><x><type>99</type></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>", "error": "unsupportedType"},
            {"name": "unsupported-encoding", "xml": "<ConfigParams><Params><x><type>1</type><vTx>99</vTx></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>", "error": "unsupportedEncoding"},
            {"name": "invalid-number", "xml": "<ConfigParams><Params><x><type>1</type><valDouble>NaN</valDouble></x></Params><SerOrder><ser>x</ser></SerOrder><Grouping/></ConfigParams>", "error": "invalidNumber"},
            {"name": "unaudited-fingerprint", "mutation": "change one validated description byte", "error": "unauditedSchema"},
            {"name": "truncated-values", "payloadHex": "000000", "error": "truncatedValues"},
            {"name": "signature-mismatch", "payloadHex": "00000000", "error": "signatureMismatch"},
            {"name": "remainder-byte", "suffixHex": "00", "error": "remainder"},
        ],
    }
    (OUTPUT / "adversarial.json").write_text(json.dumps(malformed, indent=2) + "\n")

    latest = releases[-1]
    signature_bytes = bytes.fromhex(latest["wireSignatureHex"])
    routes = {
        "schemaVersion": 1,
        "commands": {"schema": 92, "current": 93, "default": 94, "write": 95},
        "cases": [
            {"name": "schema-direct", "route": "direct", "requestHex": hex_bytes(92, 0, 0, 0, 1, 144, 0, 0, 0, 0)},
            {"name": "schema-can-42", "route": {"canId": 42}, "innerRequestHex": hex_bytes(92, 0, 0, 0, 1, 144, 0, 0, 0, 0)},
            {"name": "schema-chunk-response", "responseLayout": ["command=92", "index=0", "totalLength:i32", "offset:i32", "chunk<=400"]},
            {"name": "current-direct", "route": "direct", "requestHex": hex_bytes(93, 0)},
            {"name": "default-direct", "route": "direct", "requestHex": hex_bytes(94, 0)},
            {"name": "write-prefix", "route": "direct", "requestPrefixHex": hex_bytes(95, 0) + signature_bytes.hex()},
            {"name": "acknowledgement", "responseHex": hex_bytes(95)},
            {"name": "write-rejected", "acknowledged": True, "readback": "unchanged", "result": "conflict"},
            {"name": "write-readback-match", "acknowledged": True, "readbackFile": "values/1.3.0-readback.bin", "result": "success"},
            {"name": "write-readback-mismatch", "acknowledged": True, "readbackFile": "values/1.3.0-current.bin", "result": "conflict"},
            {"name": "wrong-index", "responsePrefixHex": hex_bytes(92, 1), "error": "unexpectedIndex"},
            {"name": "wrong-offset", "responsePrefixHex": hex_bytes(92, 0, 0, 0, 0, 1, 0, 0, 0, 1), "error": "unexpectedOffset"},
            {"name": "wrong-route", "expectedRoute": {"canId": 42}, "actualRoute": "direct", "error": "unexpectedRoute"},
            {"name": "stale-generation", "expectedGeneration": 2, "actualGeneration": 1, "error": "staleGeneration"},
            {"name": "flywheel-current-withheld", "command": 93, "mode": "flywheel", "submitted": False},
            {"name": "post-flywheel-current", "command": 93, "mode": "normal", "requiresFreshRevision": True},
        ],
    }
    (OUTPUT / "routes.json").write_text(json.dumps(routes, indent=2) + "\n")


if __name__ == "__main__":
    main()
