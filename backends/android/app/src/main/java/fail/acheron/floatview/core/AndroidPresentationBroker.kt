package fail.acheron.floatview.core

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.content.FileProvider
import fail.acheron.floatview.BuildConfig
import fail.acheron.floatview.LoggingForegroundService
import java.io.File
import java.lang.ref.WeakReference

/** Replaceable Activity/WebView presentation seam; owns no backend or archive semantics. */
internal class AndroidPresentationBroker(
    private val applicationContext: android.content.Context,
) {
    private val shareRoot = File(applicationContext.filesDir, "float-view/share")
    private var activity = WeakReference<Activity>(null)
    private var readyHandler: ((String, String) -> Unit)? = null
    private var pendingDocument: ((Uri?) -> Unit)? = null
    private var ready: Pair<String, String>? = null
    private var readyRevision = 0L

    init {
        shareRoot.listFiles()?.take(MAX_STALE_SHARE_FILES)?.forEach(File::delete)
    }

    @Synchronized
    fun attach(
        value: Activity,
        onReady: (String, String) -> Unit,
    ) {
        activity = WeakReference(value)
        readyHandler = onReady
        ready?.let { (endpoint, token) -> value.runOnUiThread { onReady(endpoint, token) } }
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) {
            activity.clear()
            readyHandler = null
        }
    }

    @Synchronized
    private fun selectDocument(contentTypes: List<String>, completion: (Uri?) -> Unit): Boolean {
        val current = activity.get() ?: return false
        if (pendingDocument != null) return false
        pendingDocument = completion
        return try {
            current.startActivityForResult(
                Intent(Intent.ACTION_OPEN_DOCUMENT)
                    .addCategory(Intent.CATEGORY_OPENABLE)
                    .setType(contentTypes.first())
                    .putExtra(Intent.EXTRA_MIME_TYPES, contentTypes.toTypedArray()),
                DOCUMENT_REQUEST,
            )
            true
        } catch (_: Exception) {
            pendingDocument = null
            false
        }
    }

    fun presentImport(
        stagingPath: String,
        allowedContentTypes: List<String>,
        maximumBytes: Long,
        completion: (DocumentResult) -> Unit,
    ) {
        val destination = approvedFile(stagingPath, mustExist = false)
        if (destination == null || maximumBytes !in 1..MAX_IMPORT_BYTES || allowedContentTypes.isEmpty()) {
            completion(DocumentResult.FAILED)
            return
        }
        if (!selectDocument(allowedContentTypes) { uri ->
                if (uri == null) {
                    completion(DocumentResult.CANCELLED)
                    return@selectDocument
                }
                Thread {
                    val result = copyDocument(uri, destination, maximumBytes)
                    completion(if (result) DocumentResult.SUCCESS else DocumentResult.FAILED)
                }.start()
            }
        ) completion(DocumentResult.UNAVAILABLE)
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

    fun presentServiceState(active: Boolean): Boolean {
        if (active) LoggingForegroundService.start(applicationContext)
        else LoggingForegroundService.stop(applicationContext)
        return true
    }

    @Synchronized
    fun presentExport(
        stagingPath: String,
        displayName: String,
        contentType: String,
        completion: (DocumentResult) -> Unit,
    ) {
        val file = approvedFile(stagingPath, mustExist = true)
        if (
            file == null || !file.isFile || displayName.isBlank() ||
            displayName.contains('/') || displayName.contains('\\')
        ) {
            completion(DocumentResult.FAILED)
            return
        }
        val current = activity.get()
        if (current == null) {
            completion(DocumentResult.UNAVAILABLE)
            return
        }
        val shared = File(shareRoot, displayName)
        val uri = try {
            shared.parentFile?.mkdirs()
            file.copyTo(shared, overwrite = true)
            FileProvider.getUriForFile(
                applicationContext,
                "${applicationContext.packageName}.exports",
                shared,
            )
        } catch (_: Exception) {
            shared.delete()
            completion(DocumentResult.FAILED)
            return
        }
        current.runOnUiThread {
            try {
                current.startActivity(
                    Intent.createChooser(
                        Intent(Intent.ACTION_SEND)
                            .setType(contentType)
                            .putExtra(Intent.EXTRA_TITLE, displayName)
                            .putExtra(Intent.EXTRA_STREAM, uri)
                            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
                        "Save or share session",
                    ),
                )
                completion(DocumentResult.SUCCESS)
                Handler(Looper.getMainLooper()).postDelayed({ shared.delete() }, SHARE_EXPIRY_MILLIS)
            } catch (_: Exception) {
                shared.delete()
                completion(DocumentResult.FAILED)
            }
        }
    }

    private fun approvedFile(path: String, mustExist: Boolean): File? = runCatching {
        require(path.toByteArray().size <= 4096)
        val root = File(applicationContext.filesDir, "float-view").canonicalFile
        val candidate = File(path)
        val canonicalParent = candidate.parentFile?.canonicalFile ?: return null
        require(canonicalParent.path == root.path || canonicalParent.path.startsWith(root.path + File.separator))
        require(!mustExist || candidate.isFile)
        candidate
    }.getOrNull()

    /** Focused test seam for Android's content-URI, bounded-copy responsibility. */
    internal fun copyDocument(source: Uri, destination: File, maximumBytes: Long): Boolean = try {
        destination.parentFile?.mkdirs()
        destination.delete()
        applicationContext.contentResolver.openInputStream(source).use { input ->
            requireNotNull(input)
            destination.outputStream().use { output ->
                val buffer = ByteArray(64 * 1024)
                var total = 0L
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    total = Math.addExact(total, count.toLong())
                    require(total <= maximumBytes)
                    output.write(buffer, 0, count)
                }
                require(total > 0)
            }
        }
        true
    } catch (_: Exception) {
        destination.delete()
        false
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
    fun readinessForTest(): Triple<Long, String, String>? {
        check(BuildConfig.DEBUG) { "test-only readiness inspection is unavailable" }
        return ready?.let { Triple(readyRevision, it.first, it.second) }
    }

    @Synchronized
    fun cancelPending() {
        pendingDocument?.invoke(null)
        pendingDocument = null
    }

    private companion object {
        const val DOCUMENT_REQUEST = 0xC0DE
        const val NOTIFICATION_REQUEST = 0xC0DF
        const val MAX_IMPORT_BYTES = 128L * 1024 * 1024
        const val SHARE_EXPIRY_MILLIS = 10L * 60 * 1000
        const val MAX_STALE_SHARE_FILES = 32
    }
}
