# Physical checkpoint guidance

Keep reusable procedures and brief source/build-linked outcomes here. Historical
summaries identify the original checkpoint record revision where an exact tested
binary was not retained; do not treat that revision as a binary attestation.
Keep failures, unavailable hardware and deferred coverage explicit.

Detailed device/OS/firmware inventories, identifiers, transcripts, timings,
screenshots and raw run reports belong under ignored `.local/physical-testing/`.
The documentation cleanup preserved originals there before shortening public
records. Keep a private backup when needed; Git does not back up ignored files.
Do not force-add local reports. Existing Git history remains unchanged.

Use `refloat-configuration-template.md` as a local result template. General BLE,
BMS, location and cleanup procedures remain in `../README.md`; announcement
procedures remain in the archived announcement change's `physical-checks.md`.
Preserve configuration safety, complete restoration and fixture RESET/READY
checks. Record actual tested build/source, target and relevant OS/firmware locally;
publish only the scoped outcome and sanitized actionable findings. Software and
fixture simulation do not establish untested real hardware or OS behavior.

## Permission recovery procedure

For Android API 29, run the existing
`conformance/src/permissions-physical-android.ts <device-serial>` checkpoint from
`conformance/` using `node --strip-types src/permissions-physical-android.ts`.
It requires initially granted background permission so restoration is defined.
If OS history blocks prompting, record Settings recovery instead of fabricating
a fresh prompt. Do not reset OS request/denial history or unrelated storage.

On iOS use `FloatViewLifecycleTests.testPermissionsRecovery` for prompt/denial,
Settings navigation, unchanged return and granting Always. Verify the restored
authorization after the test. For macOS use the normal app window with disposable
storage, exercise real OS grant/revocation and Settings-return behavior, then
restore the original access choice. Accessory/headless hosts do not establish
normal window activation or permission presentation.
