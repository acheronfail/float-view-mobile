package fail.acheron.floatview.core

import android.content.Context

internal interface FloatViewRuntimeHostProtocol {
    fun setBackgroundLogging(enabled: Boolean): Boolean
    fun diagnostic(stage: String, code: String)
    fun presentNotificationsPermission(): Boolean
    fun runtimeReady(endpoint: String, launchToken: String): Boolean
    fun presentImport(
        stagingPath: String,
        allowedContentTypes: List<String>,
        maximumBytes: Long,
        completion: (DocumentResult) -> Unit,
    ) { completion(DocumentResult.UNAVAILABLE) }
    fun presentExport(
        stagingPath: String,
        displayName: String,
        contentType: String,
        completion: (DocumentResult) -> Unit,
    ) { completion(DocumentResult.UNAVAILABLE) }
}

internal enum class DocumentResult { SUCCESS, CANCELLED, UNAVAILABLE, FAILED }

/** Android-API owner for the small set of presentation hooks retained by the Rust core. */
internal class FloatViewRuntimeHost(
    @Suppress("UNUSED_PARAMETER") context: Context,
    private val backgroundLogging: (Boolean) -> Boolean = { false },
    private val notificationPresenter: () -> Boolean = { false },
    private val readyPresenter: (String, String) -> Boolean = { _, _ -> false },
    private val importPresenter: (String, List<String>, Long, (DocumentResult) -> Unit) -> Unit =
        { _, _, _, completion -> completion(DocumentResult.UNAVAILABLE) },
    private val exportPresenter: (String, String, String, (DocumentResult) -> Unit) -> Unit =
        { _, _, _, completion -> completion(DocumentResult.UNAVAILABLE) },
    private val diagnosticHandler: (String, String) -> Unit = { _, _ -> },
) : FloatViewRuntimeHostProtocol {
    override fun setBackgroundLogging(enabled: Boolean): Boolean = backgroundLogging(enabled)
    override fun diagnostic(stage: String, code: String) = diagnosticHandler(stage, code)
    override fun presentNotificationsPermission(): Boolean = notificationPresenter()
    override fun runtimeReady(endpoint: String, launchToken: String): Boolean =
        readyPresenter(endpoint, launchToken)
    override fun presentImport(
        stagingPath: String,
        allowedContentTypes: List<String>,
        maximumBytes: Long,
        completion: (DocumentResult) -> Unit,
    ) = importPresenter(stagingPath, allowedContentTypes, maximumBytes, completion)
    override fun presentExport(
        stagingPath: String,
        displayName: String,
        contentType: String,
        completion: (DocumentResult) -> Unit,
    ) = exportPresenter(stagingPath, displayName, contentType, completion)
}
