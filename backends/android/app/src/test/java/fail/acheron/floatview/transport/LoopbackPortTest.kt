package fail.acheron.floatview.transport

import org.junit.Assert.assertEquals
import org.junit.Test

class LoopbackPortTest {
    @Test
    fun `Android backend port is stable`() {
        assertEquals(11012, ANDROID_BACKEND_PORT)
    }

}
