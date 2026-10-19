import importlib.util
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location("android_store", Path(__file__).with_name("android-store.py"))
store = importlib.util.module_from_spec(spec)
spec.loader.exec_module(store)


class BundleValidationTests(unittest.TestCase):
    def elf(self, alignment=16384, machine=183):
        data = bytearray(120)
        data[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<H", data, 18, machine)
        struct.pack_into("<Q", data, 32, 64)
        struct.pack_into("<HH", data, 54, 56, 1)
        struct.pack_into("<I", data, 64, 1)
        struct.pack_into("<Q", data, 112, alignment)
        return data

    def test_load_alignment_and_architecture(self):
        self.assertEqual(store.elf_alignment(self.elf(), "arm64-v8a"), [16384])
        for data in (self.elf(4096), self.elf(0), self.elf(24000), self.elf(machine=62), self.elf()[:70]):
            with self.assertRaises(ValueError):
                store.elf_alignment(data, "arm64-v8a")

    def test_manifest_rejects_debug_wrong_identity_and_version(self):
        xml = '<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="fail.acheron.floatview" android:versionName="1.2.0-rc.1" android:versionCode="120"><uses-sdk android:targetSdkVersion="36"/><application android:debuggable="false"/></manifest>'
        store.verify_manifest(xml, "v1.2.0-rc.1", 120)
        for changed in (xml.replace('debuggable="false"', 'debuggable="true"'), xml.replace('floatview"', 'other"'), xml.replace('versionCode="120"', 'versionCode="1"'), xml.replace('targetSdkVersion="36"', 'targetSdkVersion="29"')):
            with self.assertRaises(ValueError):
                store.verify_manifest(changed, "v1.2.0-rc.1", 120)

    def test_master_and_nightly_rejected_before_signer_use(self):
        for env in ({}, {"FLOAT_VIEW_VERSION": "v1.2.0", "GITHUB_REF": "refs/heads/master"}, {"FLOAT_VIEW_VERSION": "v1.2.1-nightly.1.g1234567"}):
            with self.assertRaises(ValueError):
                store.identity(env)


if __name__ == "__main__":
    unittest.main()
