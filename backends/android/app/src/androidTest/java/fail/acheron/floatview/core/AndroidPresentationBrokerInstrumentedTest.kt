package fail.acheron.floatview.core

import android.content.Context
import android.net.Uri
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AndroidPresentationBrokerInstrumentedTest {
    @Test
    fun boundedContentUriCopySucceedsAndCleansFailedPartialFiles() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val broker = AndroidPresentationBroker(context)
        val root = File(context.filesDir, "float-view/document-copy-${System.nanoTime()}")
        val source = File(root, "selected.floatview")
        val destination = File(root, "staging/import.tmp")
        source.parentFile?.mkdirs()
        source.writeBytes(byteArrayOf(1, 2, 3, 4))
        try {
            assertEquals(true, broker.copyDocument(Uri.fromFile(source), destination, 4))
            assertEquals(listOf<Byte>(1, 2, 3, 4), destination.readBytes().toList())

            assertEquals(false, broker.copyDocument(Uri.fromFile(source), destination, 3))
            assertEquals(false, destination.exists())

            assertEquals(
                false,
                broker.copyDocument(Uri.fromFile(File(root, "missing.floatview")), destination, 4),
            )
            assertEquals(false, destination.exists())
        } finally {
            root.deleteRecursively()
        }
    }

    @Test
    fun documentsRejectInvalidInputsAndReportUnavailableWithoutAnActivity() {
        val context = ApplicationProvider.getApplicationContext<Context>()
        val broker = AndroidPresentationBroker(context)
        val root = File(context.filesDir, "float-view")
        val importPath = File(root, "staging/import.tmp")
        val exportPath = File(root, "exports/ride.csv")
        exportPath.parentFile?.mkdirs()
        exportPath.writeBytes(byteArrayOf(1, 2, 3))
        try {
            assertEquals(
                DocumentResult.FAILED,
                importResult(broker, "relative/import.tmp", 1024),
            )
            assertEquals(
                DocumentResult.FAILED,
                importResult(broker, importPath.path, 128L * 1024 * 1024 + 1),
            )
            assertEquals(
                DocumentResult.UNAVAILABLE,
                importResult(broker, importPath.path, 1024),
            )
            assertEquals(
                DocumentResult.FAILED,
                exportResult(broker, exportPath.path, "../ride.csv"),
            )
            assertEquals(
                DocumentResult.UNAVAILABLE,
                exportResult(broker, exportPath.path, "ride.csv"),
            )
            broker.cancelPending()
        } finally {
            exportPath.delete()
        }
    }

    private fun importResult(
        broker: AndroidPresentationBroker,
        path: String,
        maximumBytes: Long,
    ): DocumentResult {
        var result: DocumentResult? = null
        broker.presentImport(path, listOf("application/octet-stream"), maximumBytes) { result = it }
        return requireNotNull(result)
    }

    private fun exportResult(
        broker: AndroidPresentationBroker,
        path: String,
        displayName: String,
    ): DocumentResult {
        var result: DocumentResult? = null
        broker.presentExport(path, displayName, "text/csv") { result = it }
        return requireNotNull(result)
    }
}
