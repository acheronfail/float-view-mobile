package fail.acheron.floatview.files

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.util.Log
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebView
import fail.acheron.floatview.lifecycle.FloatViewApplication
import fail.acheron.floatview.transport.MAX_SESSION_IMPORT_BYTES
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

class SessionFileImportController(
    private val activity: Activity,
    private val application: FloatViewApplication,
    private val webView: () -> WebView?,
) : AutoCloseable {
    private var fileChooserCallback: ValueCallback<Array<Uri>>? = null
    private val importScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    fun showFileChooser(
        filePathCallback: ValueCallback<Array<Uri>>?,
        fileChooserParams: WebChromeClient.FileChooserParams?,
    ): Boolean {
        Log.d("FloatViewFileChooser", "show")
        fileChooserCallback?.onReceiveValue(null)
        fileChooserCallback = filePathCallback
        val intent = fileChooserParams?.createIntent()
        if (intent == null) {
            fileChooserCallback = null
            filePathCallback?.onReceiveValue(null)
            return false
        }
        return try {
            activity.startActivityForResult(intent, FILE_CHOOSER_REQUEST_CODE)
            true
        } catch (_: Exception) {
            Log.d("FloatViewFileChooser", "launch_failed")
            fileChooserCallback = null
            filePathCallback?.onReceiveValue(null)
            false
        }
    }

    fun handleActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != FILE_CHOOSER_REQUEST_CODE) return false
        Log.d(
            "FloatViewFileChooser",
            "result code=$resultCode hasData=${data?.data != null} " +
                "clipCount=${data?.clipData?.itemCount ?: 0} callback=${fileChooserCallback != null}",
        )
        val result = WebChromeClient.FileChooserParams.parseResult(resultCode, data)
        // Galaxy S9 WebView 148 returns the URI to the host but does not
        // populate the DOM FileList. Import the URI natively, then notify the
        // frontend so the same archive importer is used on-device.
        fileChooserCallback?.onReceiveValue(null)
        fileChooserCallback = null
        result?.singleOrNull()?.let(::importSelectedSession)
        return true
    }

    override fun close() {
        fileChooserCallback?.onReceiveValue(null)
        fileChooserCallback = null
        importScope.cancel()
    }

    private fun importSelectedSession(uri: Uri) {
        dispatchFrontendImport("floatview:session-import-started", JSONObject())
        importScope.launch {
            try {
                val sessionId = withContext(Dispatchers.IO) {
                    application.runtime.applicationService.importSessionArchive(readImportUri(uri))
                }
                application.backend.publishSessionCatalogChanged()
                Log.d("FloatViewImport", "complete")
                dispatchFrontendImport(
                    "floatview:session-imported",
                    JSONObject().put("sessionId", sessionId),
                )
            } catch (error: Exception) {
                Log.e("FloatViewImport", "session import failed: ${error::class.simpleName}")
                dispatchFrontendImport(
                    "floatview:session-import-failed",
                    JSONObject().put("message", "Session import failed"),
                )
            }
        }
    }

    private fun readImportUri(uri: Uri): ByteArray {
        val input = activity.contentResolver.openInputStream(uri)
            ?: error("Selected file cannot be opened")
        input.use { stream ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(64 * 1024)
            var total = 0
            while (true) {
                val count = stream.read(buffer)
                if (count < 0) break
                total += count
                check(total <= MAX_SESSION_IMPORT_BYTES) { "Selected file is too large" }
                output.write(buffer, 0, count)
            }
            return output.toByteArray()
        }
    }

    private fun dispatchFrontendImport(event: String, detail: JSONObject) {
        val script =
            "window.dispatchEvent(new CustomEvent(${JSONObject.quote(event)}, { detail: $detail }));"
        webView()?.post { webView()?.evaluateJavascript(script, null) }
    }

    private companion object {
        const val FILE_CHOOSER_REQUEST_CODE = 0xF10A
    }
}
