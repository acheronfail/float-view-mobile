# Public baseline 100 fixtures

Synthetic fixtures generated on 2026-10-17 using the real product store and
session writer. These contain no user data, real device identifiers or real routes.
`catalog.sqlite` contains a synthetic saved board and non-default settings;
`recording.floatview` is an archive-format-4 recording with a synthetic location
(2°, 3°) and one ride marker. UUIDs are randomly generated fixture identities.

Keep these frozen: tests must continue to open/import them after supported app
updates. Do not regenerate them just to make an incompatible change pass. The
owned-storage marker is 100; SQLite schema/user_version and portable archive
format 4 are independently numbered. Add separate fixtures for later published
formats and an explicit tested upgrade when storage changes incompatibly.

To reproduce in a **new disposable output directory**, from the repository root:

```sh
cargo run --manifest-path backends/core/Cargo.toml --locked --offline \
  -p float-view-product-store --example baseline-fixtures -- \
  .local/ios-store/baseline-fixture-reproduction
```

The example refuses existing output directories. It never opens device storage.
Compare the semantic contents; random UUIDs mean byte hashes can differ between
reproductions. Exported files outside app-private staging remain user-owned and
are not erased by a development-storage reset.
