package fail.acheron.floatview.files

import android.app.Activity
import android.content.Intent
import androidx.core.content.FileProvider
import fail.acheron.floatview.application.SessionExportRecord
import fail.acheron.floatview.lifecycle.FloatViewApplication
import java.io.File
import java.lang.ref.WeakReference

class ExportPresentationBroker(private val application: FloatViewApplication) {
    private var activity = WeakReference<Activity>(null)

    @Synchronized
    fun attach(value: Activity) {
        activity = WeakReference(value)
    }

    @Synchronized
    fun detach(value: Activity) {
        if (activity.get() === value) activity.clear()
    }

    @Synchronized
    fun present(value: SessionExportRecord): Boolean {
        val current = activity.get() ?: return false
        val file = File(value.filePath)
        if (!file.isFile) return false
        val uri = FileProvider.getUriForFile(
            application,
            "${application.packageName}.exports",
            file,
        )
        val intent = Intent(Intent.ACTION_SEND)
            .setType(if (value.fileName.endsWith(".csv")) "text/csv" else "application/octet-stream")
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        current.runOnUiThread {
            current.startActivity(Intent.createChooser(intent, "Save or share session"))
        }
        return true
    }
}
