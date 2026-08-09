package fail.acheron.floatview.transport

import org.junit.Assert.assertEquals
import org.junit.Test

class LoopbackPortTest {
    @Test
    fun `Android backend port is stable`() {
        assertEquals(11012, ANDROID_BACKEND_PORT)
    }

    @Test
    fun `bind failure identifies the unavailable address`() {
        val failure = backendStartupFailure(ANDROID_BACKEND_PORT, IllegalStateException("occupied"))
        assertEquals(
            "Failed to bind Float View backend to 127.0.0.1:11012",
            failure.message,
        )
    }
}
