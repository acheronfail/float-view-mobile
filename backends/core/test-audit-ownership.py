"""Exercise ownership gates in disposable repositories, including macOS Bash 3.2."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


AUDIT = Path(__file__).with_name("audit-ownership.sh")
ANDROID = "backends/android/app/src/main/java/fail/acheron/floatview/application"


class OwnershipAuditTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="float-view-audit-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "--quiet", str(self.root)], check=True)
        self.write("backends/core/audit-ownership.sh", AUDIT.read_text())
        self.write("backends/core/Cargo.toml", '[workspace]\nmembers = [\n"runtime",\n]\n')
        self.write("backends/core/Cargo.lock", "")
        for crate in [
            "application-runtime-host",
            "application-runtime-apple-ffi",
            "application-runtime-android-jni",
        ]:
            self.write(f"backends/core/{crate}/Cargo.toml", "")
        self.write("backends/core/application-runtime-host/src/lib.rs", "")
        self.write("backends/apple/Sources/Runtime.swift", "")
        self.write("backends/apple/ios/float-view/application/AppBackend.swift", "SharedService()")
        self.write("backends/apple/macos/application/MacBackendRuntime.swift", "SharedService()")
        self.write(f"{ANDROID}/BackendRuntime.kt", "SharedService()")

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        return path

    def audit(self, expected=0, env=None):
        result = subprocess.run(
            ["/bin/bash", str(self.root / "backends/core/audit-ownership.sh")],
            cwd=self.root,
            capture_output=True,
            text=True,
            env=env,
        )
        if expected == 0:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("Ownership audit:", result.stderr)
        return result

    def test_valid_tree_has_no_migration_size_ceiling(self):
        self.write(
            "backends/core/Cargo.toml",
            '[workspace]\nmembers = [\n' + '"member",\n' * 29 + ']\n',
        )
        self.write("backends/core/runtime.rs", "// source\n" * 50000)
        result = self.audit()
        self.assertIn("workspace_members=29", result.stdout)

    def test_missing_binding_is_rejected(self):
        (self.root / "backends/core/application-runtime-apple-ffi/Cargo.toml").unlink()
        self.audit(expected=1)

    def test_extra_protocol_binding_is_rejected(self):
        self.write("backends/core/vesc-apple-ffi/Cargo.toml", "")
        self.audit(expected=1)

    def test_native_product_ownership_is_rejected(self):
        self.write(f"{ANDROID}/Catalog.kt", "class ApplicationService {}")
        self.audit(expected=1)

    def test_superseded_paths_are_rejected(self):
        self.write("backends/core/loopback-server/src/exchange.rs", "")
        self.audit(expected=1)

    def test_missing_shared_owner_is_rejected(self):
        self.write(f"{ANDROID}/BackendRuntime.kt", "OtherRuntime()")
        self.audit(expected=1)

    def test_tracked_native_artifact_is_rejected(self):
        self.write("runtime.so", "binary")
        subprocess.run(["git", "add", "runtime.so"], cwd=self.root, check=True)
        self.audit(expected=1)

    def test_search_errors_are_rejected(self):
        rg = self.write("bin/rg", "#!/bin/sh\nexit 2\n")
        rg.chmod(0o755)
        env = {**os.environ, "PATH": str(rg.parent) + os.pathsep + os.environ["PATH"]}
        result = self.audit(expected=1, env=env)
        self.assertIn("Source search failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
