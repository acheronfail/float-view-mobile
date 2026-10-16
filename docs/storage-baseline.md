# Supported storage baseline

The v1.0.0 candidate establishes owned-storage marker **100**, before external
TestFlight testing. A marketing version, Git tag or installation channel does not
authorize deletion. From baseline 100 onward, incompatible storage changes need
an incremented marker and an explicit, tested, crash-safe upgrade for supported
data. Do not add a migration framework until there is an actual supported
transition. Portable archive format remains **4**; SQLite PRAGMA user_version and
individual SQL schemas are independently numbered.

## Owned state and reset boundary

The shared product store owns one root:

- Apple: Application Support/FloatView/Sessions inside the current installation's
  container (or direct Mac Application Support directory).
- Android: files/float-view/logs inside the current app's private files directory.

Within that root, the reset covers catalog.sqlite and its sidecars (settings,
saved boards/associations, battery profiles, auto-connect selection, tunes and
editor drafts), issued session-UUID.sqlite files and sidecars, issued unpublished
session/import files, the bounded refloat-schema-cache and Exports staging files.
The shared root's .service.lock is coordination only and is retained. Exports
staging contains temporary files presented by the OS; copies saved/shared outside
that root are user-owned and are not reset. Unknown databases/directories or
unrecognised files inside the two owned staging/cache directories refuse reset.
Unrelated regular files alongside known storage are preserved.

The only recognised earlier shared markers found in Git history are 12, 13, 14,
15, 17, 18, 19, 20, 21, 22, 23, 24, 25 and 26. This exact allowlist permits a
one-time reset of that development state. Marker 16, arbitrary lower numbers,
unknown/newer markers, malformed/oversized/non-UTF-8 markers and a missing marker
on a nonempty root do not authorize erasure. Earlier unrelated native layouts
are not migrated or treated as a known shared-store format. Changing iOS app IDs
creates a different container; the app does not reach into the former container.

All owned paths are checked before deleting, with no symlink/hardlink following,
bounded directory enumeration and an exclusive cross-process root owner. The
old recognised marker remains until owned deletions finish. A failed/interrupted
deletion can repeat; the new marker is written with fsync/atomic rename only
afterward. Interrupted empty marker initialization can retry. Already-supported
data is not reset; a missing catalog beside recordings is treated as corruption.
SQL/data corruption, protected storage and other operational failures preserve
files and fail rather than recreate the store.

The frontend's only durable WebView hints are theme/accent. The first canonical
baseline settings snapshot overwrites their values; they never restore product
data into the new service. Apple has no project-authored UserDefaults store.
Android PermissionBroker preferences record permission-resolution history and
must remain intact, as must operating-system permission decisions on all hosts.
Do not clear all WebView/OS preferences or revoke grants to simulate a data reset.

## Frozen regression evidence

[The synthetic baseline fixtures](../backends/fixtures/public-baseline/100/README.md)
contain a minimal catalog with a board/non-default settings and an independently
exported archive-format-4 recording with synthetic location and a ride marker.
Regression tests open/import these frozen files, preserve settings across reopen,
reject unsupported archives without catalog changes, and exercise pre-baseline
reset and unsupported-marker byte preservation. Never replace the fixtures just
to make an incompatible change pass.

Apple startup now presents a native retry/recovery message instead of terminating
on a fallible database/service initialization. Retrying never offers a destructive
reset or changes the storage policy. This needs scoped UI failure/retry evidence;
the Rust storage tests do not establish native OS presentation. Android startup
was audited and still throws before activity creation on a service failure; its
nonfatal presentation remains a separate Android readiness item.
