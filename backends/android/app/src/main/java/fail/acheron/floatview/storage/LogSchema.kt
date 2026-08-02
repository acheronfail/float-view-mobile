package fail.acheron.floatview.storage

import java.io.File

object LogSchema {
    const val STORAGE_VERSION = "8"
}

internal fun prepareStorageRoot(root: File): File {
    val marker = File(root, ".schema-version")
    val storedVersion = marker.takeIf(File::isFile)?.readText()?.trim()
    if (root.exists() && storedVersion != LogSchema.STORAGE_VERSION) {
        check(root.deleteRecursively()) { "Unable to reset outdated logging storage" }
    }
    check(root.mkdirs() || root.isDirectory) { "Unable to create logging storage" }
    marker.writeText(LogSchema.STORAGE_VERSION)
    return root
}
