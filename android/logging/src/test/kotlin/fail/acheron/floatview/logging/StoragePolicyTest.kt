package fail.acheron.floatview.logging

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Files

class StoragePolicyTest {
    @Test
    fun `thresholds match the contract`() {
        assertEquals(StorageLevel.NORMAL, StoragePolicy.level(StoragePolicy.WARNING_BYTES))
        assertEquals(StorageLevel.WARNING, StoragePolicy.level(StoragePolicy.CRITICAL_BYTES))
        assertEquals(StorageLevel.CRITICAL, StoragePolicy.level(StoragePolicy.STOP_BYTES))
        assertEquals(StorageLevel.STOP, StoragePolicy.level(StoragePolicy.STOP_BYTES - 1))
    }

    @Test
    fun `outdated storage is deleted and recreated`() {
        val root = Files.createTempDirectory("float-view-storage-reset").toFile()
        try {
            val obsolete = root.resolve("obsolete.sqlite").apply { writeText("old") }
            root.resolve(".schema-version").writeText("0")

            prepareStorageRoot(root)

            assertFalse(obsolete.exists())
            assertTrue(root.isDirectory)
            assertEquals(LogSchema.STORAGE_VERSION, root.resolve(".schema-version").readText())
        } finally {
            root.deleteRecursively()
        }
    }
}
