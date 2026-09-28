package fail.acheron.floatview.files

import android.app.Activity
import android.content.Intent
import fail.acheron.floatview.application.NativeFilePresentation
import fail.acheron.floatview.lifecycle.FloatViewApplication
import java.lang.ref.WeakReference
import java.util.concurrent.Executors

/** Holds the shared lease until the scoped destination copy has actually closed. */
class ExportPresentationBroker(private val application: FloatViewApplication) {
    private var activity = WeakReference<Activity>(null)
    private var pending: NativeFilePresentation? = null
    private var copying = false
    private val copier = Executors.newSingleThreadExecutor()
    @Synchronized fun attach(value: Activity) { activity = WeakReference(value) }
    @Synchronized fun detach(value: Activity) { if (activity.get() === value) activity.clear() }
    @Synchronized fun present(value: NativeFilePresentation): Boolean {
        val current = activity.get() ?: return false
        if (pending != null || !value.file.isFile) return false
        pending = value
        current.runOnUiThread {
            try {
                current.startActivityForResult(Intent(Intent.ACTION_CREATE_DOCUMENT)
                    .addCategory(Intent.CATEGORY_OPENABLE)
                    .setType(if (value.fileName.endsWith(".csv")) "text/csv" else "application/octet-stream")
                    .putExtra(Intent.EXTRA_TITLE, value.fileName), REQUEST_CODE)
            } catch (_: Exception) { finishPending() }
        }
        return true
    }
    @Synchronized private fun finishPending() { pending?.finish(); pending = null; copying = false }
    @Synchronized fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != REQUEST_CODE) return false
        if (copying) return true
        val value = pending ?: return true
        val uri = data?.data
        if (resultCode != Activity.RESULT_OK || uri == null) { finishPending(); return true }
        copying = true
        copier.execute {
            try {
                application.contentResolver.openOutputStream(uri, "wt")?.use { output ->
                    value.file.inputStream().use { input -> input.copyTo(output, 65536) }
                }
            } catch (_: Exception) {
                // The scoped destination may be revoked or removed while copying.
            } finally { synchronized(this) { finishPending() } }
        }
        return true
    }
    companion object { private const val REQUEST_CODE = 0xF10B }
}
