package fail.acheron.floatview.transport

import org.junit.Assert.assertEquals
import org.junit.Test

class LoopbackServerTest {
    @Test
    fun webSocketControlMessagesAreLimitedTo65536Utf8Bytes() {
        assertEquals(65_536, LOOPBACK_MAX_MESSAGE_BYTES)
    }
}
