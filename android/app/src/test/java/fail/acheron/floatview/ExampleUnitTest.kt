package fail.acheron.floatview

import fail.acheron.floatview.proto.scanDevice
import fail.acheron.floatview.proto.wsMessage
import org.junit.Test

import org.junit.Assert.*

/**
 * Example local unit test, which will execute on the development machine (host).
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
class ExampleUnitTest {
    @Test
    fun addition_isCorrect() {
        assertEquals(4, 2 + 2)
    }

    @Test
    fun checkProto() {
        val msg = wsMessage {
            scanDevice = scanDevice {
                address = "AAAC"
                name = "BBBB"
            }
        }

        println("data: ${msg.toByteArray().contentToString()}")
    }
}
