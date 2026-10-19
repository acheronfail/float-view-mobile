"""Pin the owned-store boundary without silently dropping WAL/marker/session data."""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

RES = Path(__file__).resolve().parents[2] / 'backends/android/app/src/main'


def included(rules, domain, path):
    def matches(row):
        prefix = row.attrib['path'].rstrip('/')
        return row.attrib['domain'] == domain and (path == prefix or path.startswith(prefix + '/'))
    return any(matches(r) for r in rules.findall('include')) and not any(matches(r) for r in rules.findall('exclude'))


class BackupRulesTests(unittest.TestCase):
    def test_all_routes_preserve_complete_store_and_exclude_ephemeral_private_state(self):
        legacy = ET.parse(RES/'res/xml/backup_rules.xml').getroot()
        modern = ET.parse(RES/'res/xml/data_extraction_rules.xml').getroot()
        for rules in [legacy, modern.find('cloud-backup'), modern.find('device-transfer')]:
            self.assertIsNotNone(rules)
            for name in ['.schema-version', 'catalog.sqlite', 'catalog.sqlite-wal', 'catalog.sqlite-shm', 'session-id.sqlite', 'session-id.sqlite-wal']:
                self.assertTrue(included(rules, 'file', 'float-view/logs/' + name), name)
            for name in ['float-view/diagnostics/journal-0.jsonl', 'float-view/logs/Exports/temporary.zip', 'float-view/logs/refloat-schema-cache/schema.json', 'float-view/logs/.service.lock']:
                self.assertFalse(included(rules, 'file', name), name)
            self.assertFalse(included(rules, 'sharedpref', 'permission-history-v1.xml'))
            self.assertFalse(included(rules, 'root', 'app_webview/Default/Cookies'))
        self.assertEqual('true', modern.find('cloud-backup').attrib['disableIfNoEncryptionCapabilities'])
        self.assertEqual('clientSideEncryption|deviceToDeviceTransfer', legacy.find('include').attrib['requireFlags'])

    def test_manifest_enables_backup_without_foreground_interruption_or_downgrade_restore(self):
        app = ET.parse(RES/'AndroidManifest.xml').getroot().find('application')
        android = '{http://schemas.android.com/apk/res/android}'
        self.assertEqual('true', app.attrib[android+'allowBackup'])
        self.assertEqual('false', app.attrib[android+'backupInForeground'])
        self.assertNotEqual('true', app.attrib.get(android+'restoreAnyVersion'))


if __name__ == '__main__':
    unittest.main()
