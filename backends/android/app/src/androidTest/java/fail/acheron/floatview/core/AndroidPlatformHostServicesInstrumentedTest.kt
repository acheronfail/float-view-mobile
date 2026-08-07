package fail.acheron.floatview.core

import android.content.Context
import android.net.Uri
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidPlatformHostServicesInstrumentedTest {
    @Test
    fun assetsStorageRandomAndContentUriStagingRemainBounded() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val source = File(context.cacheDir, "platform-host-source.bin").apply {
            writeBytes(byteArrayOf(1, 2, 3, 4))
        }
        val host = AndroidPlatformHostServices(
            context,
            documentPresenter = { completion ->
                completion(Uri.fromFile(source))
                true
            },
        )
        val durable = host.storageRoot(StorageRootKind.DURABLE)
        val destination = File(durable, "staging/import.bin")
        val finished = CountDownLatch(1)
        var result: AndroidPlatformHostError? = AndroidPlatformHostError.IO
        host.stageDocument(destination, 4) { error ->
            result = error
            finished.countDown()
        }
        assertTrue(finished.await(5, TimeUnit.SECONDS))
        assertEquals(null, result)
        assertArrayEquals(byteArrayOf(1, 2, 3, 4), destination.readBytes())
        assertTrue(host.availableSpace(durable) > 0)
        assertEquals(32, host.secureRandom(32).size)
        assertTrue(
            host.readResource(ResourceNamespace.FRONTEND, "index.html", 8L * 1024 * 1024)
                .isNotEmpty(),
        )
        destination.delete()
        source.delete()
    }
}
