package fail.acheron.floatview.transport

import android.content.res.AssetManager
import fail.acheron.floatview.application.ApplicationService
import fail.acheron.floatview.application.EventHub
import fail.acheron.floatview.application.SessionExportRecord

internal const val ANDROID_BACKEND_PORT = 11012
internal const val LOOPBACK_MAX_MESSAGE_BYTES = 65_536
internal const val MAX_SESSION_IMPORT_BYTES = 128 * 1024 * 1024
internal fun backendStartupFailure(port: Int, cause: Exception) =
    IllegalStateException("Failed to bind Float View backend to 127.0.0.1:$port", cause)

/** Application-owned native execution; Rust owns all public transport behavior. */
class LoopbackServer(
    assets: AssetManager,
    applicationService: ApplicationService,
    exportPresenter: ((SessionExportRecord) -> Boolean)?,
    events: EventHub,
    requestedPort: Int = 0,
    platform: String = "android",
    @Suppress("UNUSED_PARAMETER") performanceDiagnostics: Boolean = false,
) : AutoCloseable {
    private val host: RustLoopbackHost
    init {
        require(platform == "android" && requestedPort in setOf(0, ANDROID_BACKEND_PORT))
        host = RustLoopbackHost(assets.open("index.html").use { it.readBytes() }, applicationService, events, exportPresenter)
    }
    val port: Int get() = host.port
    val origin: String get() = "http://127.0.0.1:$port"
    fun publishSessionCatalogChanged() = host.publish(true)
    fun start() = host.start()
    fun stop() = host.stop()
    override fun close() = host.close()
}
