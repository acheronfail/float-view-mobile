package fail.acheron.floatview.storage

import android.content.Context
import android.database.sqlite.SQLiteDatabase
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import fail.acheron.floatview.backend.DeviceId
import fail.acheron.floatview.backend.ProductStoreException
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
        val session = store.create(DeviceId("platform-private-id"), Instant.EPOCH)
        val saved = store.savedDevices().single()
        assertFalse(saved.savedDeviceId.contains("platform-private-id"))

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
            assertEquals("{}", cursor.getString(0))
            assertEquals(0, cursor.getInt(1))
            assertEquals(1, cursor.getInt(2))
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

}
