package fail.acheron.floatview.api.v1

import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Test

class GeneratedModelsTest {
    private val json = Json {
        prettyPrint = false
    }

    @Test
    fun helloResponseExampleRoundTrips() {
        val resource =
            requireNotNull(javaClass.classLoader?.getResource("system.hello.response.json")) {
                "Missing API hello response example"
            }
        val decoded = json.decodeFromString<HelloResponse>(resource.readText())

        assertEquals(1, decoded.result.api.major)
        assertEquals(0, decoded.result.api.minor)
        assertEquals(BackendPlatform.MOCK, decoded.result.backend.platform)
        assertEquals(BluetoothRadioState.POWERED_ON, decoded.result.snapshot.bluetooth.radio)

        val roundTripped = json.decodeFromString<HelloResponse>(json.encodeToString(decoded))
        assertEquals(decoded, roundTripped)
    }
}
