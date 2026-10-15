package fail.acheron.floatview.speech

import org.junit.Assert.*
import org.junit.Test

class SpeechLeaseTest {
    @Test fun terminalReleaseStopsAndAbandonsFocusExactlyOnce() {
        val lease = SpeechLease()
        var stopped = 0
        var abandoned = 0
        assertTrue(lease.begin(1) { stopped++; abandoned++ })
        assertTrue(lease.finish(1))
        assertFalse(lease.finish(1))
        assertEquals(1, stopped)
        assertEquals(1, abandoned)
        assertNull(lease.active)
    }
    @Test fun obsoleteCallbacksCannotReleaseReplacementAudio() {
        val lease = SpeechLease()
        var first = 0
        var second = 0
        assertTrue(lease.begin(1) { first++ })
        assertFalse(lease.begin(2) { second++ })
        assertEquals(0, second)
        assertTrue(lease.finish(1))
        assertTrue(lease.begin(2) { second++ })
        assertFalse(lease.finish(1))
        assertEquals(0, second)
        assertEquals(2L, lease.active)
        assertTrue(lease.finish(2))
        assertEquals(1, first)
        assertEquals(1, second)
    }
}
