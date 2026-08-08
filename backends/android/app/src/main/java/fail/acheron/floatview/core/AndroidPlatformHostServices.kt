package fail.acheron.floatview.core

import android.content.Context

internal interface AndroidPlatformHost {
    fun setBackgroundLogging(enabled: Boolean): Boolean
    fun diagnostic(stage: String, code: String)
    fun presentNotificationsPermission(): Boolean
    fun runtimeReady(endpoint: String, launchToken: String): Boolean
}

/** Android-API owner for the small set of presentation hooks retained by the Rust core. */
internal class AndroidPlatformHostServices(
    @Suppress("UNUSED_PARAMETER") context: Context,
    private val backgroundLogging: (Boolean) -> Boolean = { false },
    private val notificationPresenter: () -> Boolean = { false },
    private val readyPresenter: (String, String) -> Boolean = { _, _ -> false },
    private val diagnosticHandler: (String, String) -> Unit = { _, _ -> },
) : AndroidPlatformHost {
    override fun setBackgroundLogging(enabled: Boolean): Boolean = backgroundLogging(enabled)
    override fun diagnostic(stage: String, code: String) = diagnosticHandler(stage, code)
    override fun presentNotificationsPermission(): Boolean = notificationPresenter()
    override fun runtimeReady(endpoint: String, launchToken: String): Boolean =
        readyPresenter(endpoint, launchToken)
}
