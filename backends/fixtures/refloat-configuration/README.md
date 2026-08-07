# Refloat configuration fixtures

These immutable fixtures are regenerated from the offline `vendor/refloat`
repository by `tools/generate-refloat-configuration-fixtures.py`. The manifest
pins each stable tag, commit, source path, source digest, exact VESC Tool
Qt-`qCompress` bytes, canonical normalized-schema fingerprint, CRC32C wire
signature, field counts, payload digests, and version-specific readback
normalization.

`schemas/` contains the exact generated command-92 byte stream. `values/`
contains complete signed command-93/94 bodies and representative command-95
write/readback bodies. `adversarial.json` and `routes.json` define shared
negative and command/routing vectors without raw device identity.

Regenerate deliberately after auditing a new stable source tag:

```sh
python3 tools/generate-refloat-configuration-fixtures.py
```

Generation requires the installed VESC Tool CLI because its XML writer plus
`qCompress(..., 9)` output is the firmware source of truth.

For a newly supported release, first fetch its tag into the offline
`vendor/refloat` mirror and audit the exact `src/conf/settings.xml` plus the
firmware serialization code. Add the tag/commit to the generator allowlist,
regenerate, then review the manifest diff: source and compressed digests,
normalized fingerprint, CRC32C signature, field counts, complete defaults,
representative write, and expected normalization/readback must all be explained.
Update the mirrored Swift/Kotlin audited catalogs and run their full valid and
adversarial fixture tables. Never reuse a nearby patch's fingerprint, infer a
release from INFO alone, or hand-edit generated schema/value bytes.

`Package.swift` and `FixtureResources.swift` expose these same immutable bytes
to the Apple software conformance peripheral as a local resource package.
Android packages the directory as read-only test assets. Neither mechanism is a
production schema trust shortcut: native decoders still fingerprint and audit
the complete schema before exposing configuration.
