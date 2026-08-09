package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import fail.acheron.floatview.application.DeviceId
import fail.acheron.floatview.application.ControllerRoutingHint
import fail.acheron.floatview.application.ProductStoreException
import fail.acheron.floatview.application.RefloatConfigurationIdentityRecord
import fail.acheron.floatview.application.RefloatSchemaCacheIdentity
import fail.acheron.floatview.application.RideState
import fail.acheron.floatview.application.TelemetrySample
import java.io.File
import java.time.Instant
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SQLiteBoardProfileInstrumentedTest {
    private lateinit var context: Context
    private lateinit var root: File

    @Before
    fun setUp() {
        context = InstrumentationRegistry.getInstrumentation().targetContext
        root = File(context.filesDir, "float-view/logs")
        root.deleteRecursively()
    }

    @After
    fun tearDown() {
        root.deleteRecursively()
    }

    @Test
    fun profileIdentityMergeRevisionAndRestartAreDurable() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val session = store.create(DeviceId("platform-private-id"), "Advertised", Instant.EPOCH)
        val saved = store.savedDevices().single()
        assertFalse(saved.savedDeviceId.contains("platform-private-id"))
        assertEquals("Advertised", saved.displayName)
        assertEquals("platform-private-id", saved.bluetoothIdentifier)
        val renamed = store.updateSavedDevice(
            saved.savedDeviceId,
            0,
            "Trail board",
            Instant.EPOCH,
        )
        assertEquals("Trail board", renamed.displayName)
        assertEquals(1, renamed.revision)
        assertThrows(ProductStoreException.SavedDeviceRevisionConflict::class.java) {
            store.updateSavedDevice(saved.savedDeviceId, 0, null, Instant.EPOCH)
        }

        val initial = store.boardProfile(saved.savedDeviceId)
        assertEquals(0, initial.revision)
        assertEquals(
            listOf("seriesCount", "parallelCount", "motor"),
            initial.missingFields,
        )
        assertThrows(ProductStoreException.InvalidProfilePatch::class.java) {
            store.updateBoardProfile(
                saved.savedDeviceId,
                0,
                """{"seriesCount":0}""",
                Instant.ofEpochSecond(1),
            )
        }
        val updated = store.updateBoardProfile(
            saved.savedDeviceId,
            0,
            """{"seriesCount":18,"motor":"hypercore_xr"}""",
            Instant.ofEpochSecond(1),
        )
        assertEquals(1, updated.revision)
        assertEquals(listOf("parallelCount"), updated.missingFields)
        assertThrows(ProductStoreException.ProfileRevisionConflict::class.java) {
            store.updateBoardProfile(
                saved.savedDeviceId,
                0,
                """{"seriesCount":20}""",
                Instant.ofEpochSecond(2),
            )
        }
        store.append(
            session.jobId,
            listOf(
                TelemetrySample(
                    sequence = 1,
                    observedAt = Instant.EPOCH,
                    elapsedMs = 0,
                    rideState = RideState.RIDING,
                    speedKph = 18.0,
                ),
                TelemetrySample(
                    sequence = 2,
                    observedAt = Instant.ofEpochSecond(20),
                    elapsedMs = 20_000,
                    rideState = RideState.RIDING,
                    speedKph = 18.0,
                ),
            ),
        )
        store.close(session.jobId, Instant.ofEpochSecond(3), "test")

        val catalog = SQLiteDatabase.openDatabase(
            File(root, "catalog.sqlite").path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        val sessionFile = catalog.rawQuery(
            """
            SELECT profile_snapshot_json, profile_revision, event_count, file_name
            FROM session_catalog
            """.trimIndent(),
            null,
        ).use { cursor ->
            cursor.moveToFirst()
            assertEquals("""{"preferredBmsCanId":null}""", cursor.getString(0))
            assertEquals(0, cursor.getInt(1))
            assertEquals(3, cursor.getInt(2))
            cursor.getString(3)
        }
        catalog.close()
        val sessionDatabase = SQLiteDatabase.openDatabase(
            File(root, sessionFile).path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        sessionDatabase.rawQuery(
            "SELECT kind, details_json FROM session_event",
            null,
        ).use { cursor ->
            cursor.moveToFirst()
            assertEquals("board_profile_changed", cursor.getString(0))
            assertTrue(cursor.getString(1).contains("\"revision\":1"))
        }
        sessionDatabase.close()

        val reopened = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertEquals(updated, reopened.boardProfile(saved.savedDeviceId))
    }

    @Test
    fun controllerPreferenceAndPrivateHintStayAtomicAndPrivate() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val saved = store.ensureSavedDevice(DeviceId("private-platform-id"), "Board", Instant.EPOCH)
        assertEquals(null, store.active())
        assertThrows(ProductStoreException.InvalidProfilePatch::class.java) {
            store.updateBoardProfile(
                saved.savedDeviceId,
                0,
                """{"preferredCanId":254}""",
                Instant.EPOCH,
            )
        }
        val uuid = ByteArray(12) { (it + 1).toByte() }
        store.storeControllerRoutingHint(
            ControllerRoutingHint(saved.savedDeviceId, 1, uuid),
            Instant.EPOCH,
        )
        val configured = store.updateBoardProfile(
            saved.savedDeviceId,
            0,
            """{"preferredCanId":42}""",
            Instant.EPOCH,
        )
        assertEquals(listOf("seriesCount", "parallelCount", "motor"), configured.missingFields)
        assertEquals(null, store.controllerRoutingHint(saved.savedDeviceId))

        assertThrows(ProductStoreException.InvalidProfilePatch::class.java) {
            store.updateBoardProfile(
                saved.savedDeviceId,
                configured.revision,
                """{"preferredBmsCanId":254}""",
                Instant.EPOCH,
            )
        }
        val bmsConfigured = store.updateBoardProfile(
            saved.savedDeviceId,
            configured.revision,
            """{"preferredBmsCanId":42}""",
            Instant.EPOCH,
        )
        assertTrue(bmsConfigured.valuesJson.contains("\"preferredBmsCanId\":42"))

        store.storeControllerRoutingHint(
            ControllerRoutingHint(saved.savedDeviceId, 42, uuid),
            Instant.EPOCH,
        )
        val editor = store.updateBoardProfile(
            saved.savedDeviceId,
            bmsConfigured.revision,
            """{"seriesCount":20}""",
            Instant.EPOCH,
        )
        val selected = store.selectController(saved.savedDeviceId, 7, uuid, Instant.EPOCH)
        assertEquals(editor.revision + 1, selected.revision)
        assertTrue(selected.valuesJson.contains("\"seriesCount\":20"))
        assertTrue(selected.valuesJson.contains("\"preferredCanId\":7"))
        val hint = store.controllerRoutingHint(saved.savedDeviceId)
        assertEquals(7, hint?.canId)
        assertTrue(requireNotNull(hint?.controllerUuid).contentEquals(uuid))

        store.updateBoardProfile(
            saved.savedDeviceId,
            selected.revision,
            """{"preferredCanId":null}""",
            Instant.EPOCH,
        )
        assertEquals(null, store.controllerRoutingHint(saved.savedDeviceId))
    }

    @Test
    fun refloatConfigurationsAreExactVersionPagedConflictSafeAndDurable() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val stable = RefloatConfigurationIdentityRecord("refloat", "1.0.1", "b".repeat(64), 123)
        val colliding = RefloatConfigurationIdentityRecord("refloat", "1.0.0", "a".repeat(64), 123)
        val values = """{"kp":{"type":"double","value":20}}"""
        val first = store.createRefloatConfiguration("Trail", null, stable, values, Instant.ofEpochSecond(1))
        val second = store.createRefloatConfiguration("Street", "calm", stable, values, Instant.ofEpochSecond(2))
        store.createRefloatConfiguration("Legacy", null, colliding, values, Instant.ofEpochSecond(3))
        val page = store.refloatConfigurations(stable, 1, null)
        assertEquals(listOf(second.configurationId), page.items.map { it.configurationId })
        assertEquals(
            listOf(first.configurationId),
            store.refloatConfigurations(stable, 1, requireNotNull(page.nextCursor)).items.map { it.configurationId },
        )
        assertEquals(1, store.refloatConfigurations(colliding, 50, null).items.size)
        val updated = store.updateRefloatConfiguration(
            first.configurationId, 1, "Trail 2", null, values, Instant.ofEpochSecond(4),
        )
        assertEquals(2, updated.revision)
        assertThrows(ProductStoreException.RefloatConfigurationRevisionConflict::class.java) {
            store.updateRefloatConfiguration(first.configurationId, 1, "stale", null, values, Instant.MAX)
        }
        val reopened = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertEquals("Trail 2", reopened.refloatConfiguration(first.configurationId).name)
        reopened.deleteRefloatConfiguration(first.configurationId, 2)
        assertThrows(ProductStoreException.RefloatConfigurationNotFound::class.java) {
            reopened.refloatConfiguration(first.configurationId)
        }
    }

    @Test
    fun refloatSchemaCacheRequiresExactNonzeroIdentityAndDeletesCorruption() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val identity = RefloatSchemaCacheIdentity("6.6", "THOR300", "1.2.3", 0, 123)
        val data = ByteArray(1_024) { it.toByte() }
        assertEquals(null, store.cachedRefloatSchema(identity))
        store.storeRefloatSchema(data, identity)
        assertTrue(requireNotNull(store.cachedRefloatSchema(identity)).contentEquals(data))
        assertEquals(null, store.cachedRefloatSchema(identity.copy(hardwareConfigurationCrc = 124)))
        val reopened = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertTrue(requireNotNull(reopened.cachedRefloatSchema(identity)).contentEquals(data))
        val cacheFile = requireNotNull(File(root, "refloat-schema-cache").listFiles()).single()
        cacheFile.writeText("corrupt")
        assertEquals(null, reopened.cachedRefloatSchema(identity))
        assertFalse(cacheFile.exists())
    }

    @Test
    fun forgettingBoardRemovesAllOwnedDataAcrossRestart() {
        val store = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        val session = store.create(DeviceId("private-platform-id"), "Board", Instant.EPOCH)
        val saved = store.savedDevices().single()
        store.storeControllerRoutingHint(
            ControllerRoutingHint(saved.savedDeviceId, 42, ByteArray(12) { (it + 1).toByte() }),
            Instant.EPOCH,
        )
        store.close(session.jobId, Instant.ofEpochSecond(1), "test")

        val catalog = SQLiteDatabase.openDatabase(
            File(root, "catalog.sqlite").path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        val sessionFile = catalog.rawQuery("SELECT file_name FROM session_catalog", null).use { cursor ->
            assertTrue(cursor.moveToFirst())
            cursor.getString(0)
        }
        catalog.close()

        store.deleteSavedDevice(saved.savedDeviceId)
        assertTrue(store.savedDevices().isEmpty())
        assertFalse(File(root, sessionFile).exists())

        val remaining = SQLiteDatabase.openDatabase(
            File(root, "catalog.sqlite").path,
            null,
            SQLiteDatabase.OPEN_READONLY,
        )
        fun count(table: String): Int = remaining.rawQuery("SELECT COUNT(*) FROM $table", null).use { cursor ->
            assertTrue(cursor.moveToFirst())
            cursor.getInt(0)
        }
        assertEquals(0, count("board_profile"))
        assertEquals(0, count("controller_route_hint"))
        assertEquals(0, count("session_catalog"))
        remaining.close()

        val reopened = SQLiteLogStore(context, availableBytes = { 2_000_000_000 })
        assertTrue(reopened.savedDevices().isEmpty())
    }

}
