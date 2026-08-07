package fail.acheron.floatview.core

import android.content.Context
import android.net.Uri
import android.os.SystemClock
import java.io.File
import java.io.FileOutputStream
import java.io.ByteArrayOutputStream
import java.security.SecureRandom

internal enum class AndroidPlatformHostError { INVALID_PATH, UNAVAILABLE, PAYLOAD_TOO_LARGE, IO }

internal interface AndroidPlatformHost {
    fun setBackgroundLogging(enabled: Boolean): Boolean
    fun readResource(namespace: ResourceNamespace, logicalName: String, maximumBytes: Long): ByteArray
    fun stageDocument(
        destination: File,
        maximumBytes: Long,
        completion: (AndroidPlatformHostError?) -> Unit,
    )
    fun storageRoot(kind: StorageRootKind): File
    fun availableSpace(root: File): Long
    fun monotonicMilliseconds(): Long
    fun wallMilliseconds(): Long
    fun secureRandom(count: Int): ByteArray
    fun diagnostic(stage: String, code: String)
    fun presentNotificationsPermission(): Boolean
    fun presentTheme(theme: PlatformTheme): Boolean
    fun presentServiceState(active: Boolean): Boolean
    fun presentExport(stagedFile: File, completion: (AndroidPlatformHostError?) -> Unit)
    fun runtimeReady(endpoint: String, launchToken: String): Boolean
    fun cancelPendingPresentation()
}

/** Android-API owner with bounded byte/file behavior and injectable UI presentation. */
internal class AndroidPlatformHostServices(
    context: Context,
    private val backgroundLogging: (Boolean) -> Boolean = { false },
    private val documentPresenter: (((Uri?) -> Unit) -> Boolean)? = null,
    private val notificationPresenter: () -> Boolean = { false },
    private val themePresenter: (PlatformTheme) -> Boolean = { false },
    private val servicePresenter: (Boolean) -> Boolean = { false },
    private val exportPresenter: ((File, (Boolean) -> Unit) -> Unit)? = null,
    private val readyPresenter: (String, String) -> Boolean = { _, _ -> false },
    private val presentationCanceller: () -> Unit = {},
    private val diagnosticHandler: (String, String) -> Unit = { _, _ -> },
) : AndroidPlatformHost {
    private val applicationContext = context.applicationContext
    private val random = SecureRandom()
    private val roots = mapOf(
        StorageRootKind.DURABLE to File(applicationContext.filesDir, "core"),
        StorageRootKind.CACHE to File(applicationContext.cacheDir, "core"),
        StorageRootKind.TEMPORARY to File(applicationContext.cacheDir, "core/tmp"),
    )

    override fun setBackgroundLogging(enabled: Boolean): Boolean = backgroundLogging(enabled)

    override fun readResource(
        namespace: ResourceNamespace,
        logicalName: String,
        maximumBytes: Long,
    ): ByteArray {
        require(maximumBytes in 1..MAX_COMPLETION_BYTES) { AndroidPlatformHostError.PAYLOAD_TOO_LARGE.name }
        require(validLogicalName(logicalName)) { AndroidPlatformHostError.INVALID_PATH.name }
        val assetName = when (namespace) {
            ResourceNamespace.FRONTEND -> logicalName
            ResourceNamespace.SQL -> logicalName
            ResourceNamespace.SOFTWARE_FIXTURE -> logicalName
        }
        return applicationContext.assets.open(assetName).use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(64 * 1024)
            var total = 0L
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                total += count
                require(total <= maximumBytes) { AndroidPlatformHostError.PAYLOAD_TOO_LARGE.name }
                output.write(buffer, 0, count)
            }
            output.toByteArray()
        }
    }

    override fun stageDocument(
        destination: File,
        maximumBytes: Long,
        completion: (AndroidPlatformHostError?) -> Unit,
    ) {
        if (
            maximumBytes !in 1..MAX_DOCUMENT_BYTES || !isOwnedPath(destination) ||
            documentPresenter == null
        ) {
            completion(AndroidPlatformHostError.INVALID_PATH)
            return
        }
        if (!documentPresenter.invoke { uri ->
                if (uri == null) {
                    completion(AndroidPlatformHostError.UNAVAILABLE)
                } else {
                    completion(copyContentUri(applicationContext, uri, destination, maximumBytes))
                }
            }
        ) {
            completion(AndroidPlatformHostError.UNAVAILABLE)
        }
    }

    override fun storageRoot(kind: StorageRootKind): File =
        requireNotNull(roots[kind]).canonicalFile.also { root ->
            check(root.mkdirs() || root.isDirectory)
        }

    override fun availableSpace(root: File): Long {
        val canonical = root.canonicalFile
        require(isOwnedPath(canonical)) { AndroidPlatformHostError.INVALID_PATH.name }
        return canonical.usableSpace.also { require(it >= 0) }
    }

    override fun monotonicMilliseconds(): Long = SystemClock.elapsedRealtime()
    override fun wallMilliseconds(): Long = System.currentTimeMillis()

    override fun secureRandom(count: Int): ByteArray {
        require(count in 1..256)
        return ByteArray(count).also(random::nextBytes)
    }

    override fun diagnostic(stage: String, code: String) = diagnosticHandler(stage, code)
    override fun presentNotificationsPermission(): Boolean = notificationPresenter()
    override fun presentTheme(theme: PlatformTheme): Boolean = themePresenter(theme)
    override fun presentServiceState(active: Boolean): Boolean = servicePresenter(active)

    override fun presentExport(stagedFile: File, completion: (AndroidPlatformHostError?) -> Unit) {
        val presenter = exportPresenter
        if (!isOwnedPath(stagedFile) || !stagedFile.isFile || presenter == null) {
            completion(AndroidPlatformHostError.INVALID_PATH)
            return
        }
        presenter(stagedFile.canonicalFile) { success ->
            completion(if (success) null else AndroidPlatformHostError.UNAVAILABLE)
        }
    }

    override fun runtimeReady(endpoint: String, launchToken: String): Boolean =
        readyPresenter(endpoint, launchToken)

    override fun cancelPendingPresentation() = presentationCanceller()

    private fun isOwnedPath(file: File): Boolean {
        val candidate = file.canonicalFile.path
        return roots.values.any { root ->
            val path = root.canonicalFile.path
            candidate == path || candidate.startsWith("$path${File.separator}")
        }
    }

    private companion object {
        const val MAX_COMPLETION_BYTES = 8L * 1024 * 1024
        const val MAX_DOCUMENT_BYTES = 512L * 1024 * 1024

        fun validLogicalName(value: String): Boolean =
            value.isNotEmpty() && value.encodeToByteArray().size <= 128 &&
                !value.startsWith('/') && !value.startsWith('\\') &&
                !value.contains("..") && !value.contains(':') &&
                value.split('/').all { it.isNotEmpty() && it != "." } &&
                value.all { it.isLetterOrDigit() && it.code < 128 || it in "-_./" }

        fun copyContentUri(
            context: Context,
            source: Uri,
            destination: File,
            maximumBytes: Long,
        ): AndroidPlatformHostError? =
            try {
                require(maximumBytes in 1..MAX_DOCUMENT_BYTES)
                check(destination.parentFile?.mkdirs() != false || destination.parentFile?.isDirectory == true)
                val input = context.contentResolver.openInputStream(source)
                    ?: return AndroidPlatformHostError.UNAVAILABLE
                input.use { stream ->
                    FileOutputStream(destination, false).use { output ->
                        val buffer = ByteArray(64 * 1024)
                        var total = 0L
                        while (true) {
                            val count = stream.read(buffer)
                            if (count < 0) break
                            total += count
                            if (total > maximumBytes) {
                                destination.delete()
                                return AndroidPlatformHostError.PAYLOAD_TOO_LARGE
                            }
                            output.write(buffer, 0, count)
                        }
                    }
                }
                null
            } catch (_: Exception) {
                destination.delete()
                AndroidPlatformHostError.IO
            }
    }
}
