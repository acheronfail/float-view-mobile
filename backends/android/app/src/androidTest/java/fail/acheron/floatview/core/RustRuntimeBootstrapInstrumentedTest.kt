package fail.acheron.floatview.core

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import fail.acheron.floatview.FloatViewApplication
import java.net.HttpURLConnection
import java.net.URL
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class RustRuntimeBootstrapInstrumentedTest {
    @Test
    fun productionRuntimeBootstrapsSuspendsAndResumesWithAndroidAdapters() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val application = context as FloatViewApplication
        assertTrue(application.usesRustBackend)
        application.rustRuntime.updateLifecycle(foreground = true)
        val first = waitForReadiness(application, previousRevision = 0)
        assertTrue(first.third.isNotEmpty())
        assertBootstrap(first.second)

        application.rustRuntime.updateLifecycle(foreground = false)
        application.rustRuntime.updateLifecycle(foreground = true)
        val resumed = waitForReadiness(application, previousRevision = first.first)
        assertEquals(first.third, resumed.third)
        assertBootstrap(resumed.second)
    }

    private fun assertBootstrap(endpoint: String) {
        val connection = URL(endpoint).openConnection() as HttpURLConnection
        connection.connectTimeout = 5_000
        connection.readTimeout = 5_000
        assertEquals(200, connection.responseCode)
        assertTrue(connection.inputStream.use { it.readBytes() }.isNotEmpty())
        connection.disconnect()
    }

    private fun waitForReadiness(
        application: FloatViewApplication,
        previousRevision: Long,
    ): Triple<Long, String, String> {
        val deadline = System.currentTimeMillis() + 10_000
        while (System.currentTimeMillis() < deadline) {
            val ready = application.rustRuntime.readinessForTest()
            if (ready != null && ready.first > previousRevision) return ready
            Thread.sleep(20)
        }
        error("Rust runtime readiness timed out")
    }
}
