package fail.acheron.floatview.logging

import org.junit.Assert.assertEquals
import org.junit.Test

class StoragePolicyTest {
    @Test
    fun `thresholds match the contract`() {
        assertEquals(StorageLevel.NORMAL, StoragePolicy.level(StoragePolicy.WARNING_BYTES))
        assertEquals(StorageLevel.WARNING, StoragePolicy.level(StoragePolicy.CRITICAL_BYTES))
        assertEquals(StorageLevel.CRITICAL, StoragePolicy.level(StoragePolicy.STOP_BYTES))
        assertEquals(StorageLevel.STOP, StoragePolicy.level(StoragePolicy.STOP_BYTES - 1))
    }
}
