package fail.acheron.floatview.application

import java.nio.file.Files
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ServiceBoundaryInstrumentedTest {
    private fun withService(action: (Long) -> Unit) {
        RustApplicationRuntime.ensureLoaded()
        val root = Files.createTempDirectory(androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().targetContext.cacheDir.toPath(), "float-view-device-jni").toFile().canonicalFile
        val handle = RustApplicationRuntime.serviceCreate(root.path.toByteArray())
        assertTrue(handle > 0)
        try { action(handle) } finally {
            assertEquals(0, RustApplicationRuntime.serviceDestroy(handle))
            root.deleteRecursively()
        }
    }
    @Test fun retryableCopyAndClosedEnvelope() = withService { handle ->
        val command = "{\"command\":\"drain\"}".toByteArray()
        val required = IntArray(1)
        assertEquals(0, RustApplicationRuntime.serviceApply(handle, command, required))
        assertTrue(required[0] > 0)
        assertEquals(5, RustApplicationRuntime.serviceApply(handle, command, IntArray(1)))
        val written = IntArray(1)
        assertEquals(6, RustApplicationRuntime.serviceCopyResult(handle, ByteArray(0), written))
        assertEquals(required[0], written[0])
        val output = ByteArray(written[0])
        assertEquals(0, RustApplicationRuntime.serviceCopyResult(handle, output, written))
        val result = JSONObject(String(output))
        assertTrue(result.getLong("lifetime") > 0)
        assertEquals("host_state", result.getJSONArray("effects").getJSONObject(0).getString("effect"))
        assertEquals(3, RustApplicationRuntime.serviceApply(handle, "{\"command\":\"drain\",\"extra\":1}".toByteArray(), required))
        assertEquals(1, RustApplicationRuntime.serviceApply(handle, null, required))
        assertEquals(1, RustApplicationRuntime.serviceApply(handle, command, IntArray(2)))
        assertEquals(1, RustApplicationRuntime.serviceApply(handle, ByteArray(1048577), required))
        assertEquals(2, RustApplicationRuntime.serviceApply(Long.MAX_VALUE, command, required))
    }
    @Test fun listenerBorrowsServiceAndDestructionInvalidatesHandle() = withService { handle ->
        val server = RustApplicationRuntime.serviceServerCreate(handle, 2, "<html/>".toByteArray())
        assertTrue(server > 0)
        val input = "{\"version\":1,\"action\":{\"command\":\"stop\"}}".toByteArray()
        val output = ByteArray(1048576)
        val written = IntArray(1)
        assertEquals(1, RustApplicationRuntime.serverCommand(server, input, ByteArray(1), written))
        assertEquals(0, RustApplicationRuntime.serverCommand(server, input, output, written))
        assertEquals(0, RustApplicationRuntime.serverDestroy(server))
        assertEquals(0, RustApplicationRuntime.serverDestroy(server))
        assertEquals(2, RustApplicationRuntime.serverCommand(server, input, output, written))
        assertEquals(0, RustApplicationRuntime.serviceApply(handle, "{\"command\":\"drain\"}".toByteArray(), written))
        assertEquals(0, RustApplicationRuntime.serviceCopyResult(handle, output, written))
    }
}
