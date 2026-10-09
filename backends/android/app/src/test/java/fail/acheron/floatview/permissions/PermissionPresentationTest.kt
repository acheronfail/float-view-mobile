package fail.acheron.floatview.permissions

import org.junit.Assert.*
import org.junit.Test

class PermissionPresentationTest {
    @Test fun `absent activity fails immediately and nothing is deferred`() {
        val slot = PermissionPresentation()
        val results = mutableListOf<Boolean>()
        assertNull(slot.begin(false, results::add))
        assertEquals(listOf(false), results)
        assertNotNull(slot.begin(true, results::add))
        assertEquals(listOf(false), results)
    }
    @Test fun `timeout and late system completion cannot finish a newer presentation`() {
        val slot = PermissionPresentation()
        val results = mutableListOf<Boolean>()
        assertFalse(slot.busy())
        val old = slot.begin(true, results::add)!!
        assertTrue(slot.busy())
        assertTrue(slot.startSystem(old))
        slot.finish(old, false)
        assertTrue(slot.busy())
        assertNull(slot.begin(true, results::add))
        slot.resolveSystem(true)
        assertFalse(slot.busy())
        val current = slot.begin(true, results::add)!!
        slot.finish(old, true)
        assertTrue(slot.active(current))
        slot.finish(current, true)
        slot.finish(current, true)
        assertEquals(listOf(false, false, true), results)
    }
}
