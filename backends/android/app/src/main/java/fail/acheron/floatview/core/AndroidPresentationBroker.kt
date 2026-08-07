package fail.acheron.floatview.core

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import androidx.core.content.FileProvider
import fail.acheron.floatview.LoggingForegroundService
import java.io.File
import java.lang.ref.WeakReference

/** Replaceable Activity/WebView presentation seam; owns no backend or archive semantics. */
internal class AndroidPresentationBroker(
    private val applicationContext: android.content.Context,
) {
    private var activity = WeakReference<Activity>(null)
    private var readyHandler: ((String, String) -> Unit)? = null
    private var themeHandler: ((PlatformTheme) -> Unit)? = null
    private var pendingDocument: ((Uri?) -> Unit)? = null
    private var ready: Pair<String, String>? = null
    private var readyRevision = 0L

    @Synchronized
    fun attach(
        value: Activity,
        onReady: (String, String) -> Unit,
        onTheme: (PlatformTheme) -> Unit,
    ) {
        activity = WeakReference(value)
        readyHandler = onReady
        themeHandler = onTheme
        ready?.let { (endpoint, token) -> value.runOnUiThread { onReady(endpoint, token) } }
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) {
            activity.clear()
            readyHandler = null
            themeHandler = null
        }
    }

    @Synchronized
    fun selectDocument(completion: (Uri?) -> Unit): Boolean {
        val current = activity.get() ?: return false
        if (pendingDocument != null) return false
        pendingDocument = completion
        return try {
            current.startActivityForResult(
                Intent(Intent.ACTION_OPEN_DOCUMENT)
                    .addCategory(Intent.CATEGORY_OPENABLE)
                    .setType("application/octet-stream"),
                DOCUMENT_REQUEST,
            )
            true
        } catch (_: Exception) {
            pendingDocument = null
            false
        }
    }

    @Synchronized
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != DOCUMENT_REQUEST) return false
        val completion = pendingDocument.also { pendingDocument = null }
        completion?.invoke(data?.data.takeIf { resultCode == Activity.RESULT_OK })
        return true
    }

    @Synchronized
    fun presentNotificationsPermission(): Boolean {
        if (Build.VERSION.SDK_INT < 33) return true
        val current = activity.get() ?: return false
        if (current.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return true
        }
        current.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), NOTIFICATION_REQUEST)
        return true
    }

    @Synchronized
    fun presentTheme(theme: PlatformTheme): Boolean {
        val current = activity.get() ?: return false
        val handler = themeHandler ?: return false
        current.runOnUiThread { handler(theme) }
        return true
    }

    fun presentServiceState(active: Boolean): Boolean {
        if (active) LoggingForegroundService.start(applicationContext)
        else LoggingForegroundService.stop(applicationContext)
        return true
    }

    @Synchronized
    fun presentExport(file: File, completion: (Boolean) -> Unit) {
        val current = activity.get()
        if (current == null || !file.isFile) {
            completion(false)
            return
        }
        val uri = FileProvider.getUriForFile(
            applicationContext,
            "${applicationContext.packageName}.exports",
            file,
        )
        current.runOnUiThread {
            try {
                current.startActivity(
                    Intent.createChooser(
                        Intent(Intent.ACTION_SEND)
                            .setType("application/octet-stream")
                            .putExtra(Intent.EXTRA_STREAM, uri)
                            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
                        "Save or share session",
                    ),
                )
                completion(true)
            } catch (_: Exception) {
                completion(false)
            }
        }
    }

    @Synchronized
    fun runtimeReady(endpoint: String, launchToken: String): Boolean {
        ready = endpoint to launchToken
        readyRevision = Math.addExact(readyRevision, 1)
        val current = activity.get()
        val handler = readyHandler
        if (current != null && handler != null) {
            current.runOnUiThread { handler(endpoint, launchToken) }
        }
        return true
    }

    @Synchronized
    fun readinessForTest(): Triple<Long, String, String>? =
        ready?.let { Triple(readyRevision, it.first, it.second) }

    @Synchronized
    fun cancelPending() {
        pendingDocument?.invoke(null)
        pendingDocument = null
    }

    private companion object {
        const val DOCUMENT_REQUEST = 0xC0DE
        const val NOTIFICATION_REQUEST = 0xC0DF
    }
}
