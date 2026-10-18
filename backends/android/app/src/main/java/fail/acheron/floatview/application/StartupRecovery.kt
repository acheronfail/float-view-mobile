package fail.acheron.floatview.application

/** Host startup stages; no storage reset is a recovery action. */
enum class StartupFailure(val title: String, val recovery: String) {
    DATA(
        "Unable to open Float View data",
        "Float View could not open its saved rides and settings. Check available storage in Settings, then try again. If you used a newer version of Float View, install that version or later. Do not uninstall the app or clear its storage, as this can erase your saved rides and settings.",
    ),
    CONNECTION(
        "Unable to start Float View’s local connection",
        "Float View could not start the local connection used by its interface. Try again. If the problem continues, restart your device and reopen Float View.",
    ),
    INTERFACE(
        "Unable to load Float View",
        "Float View could not load its interface. Try again. If the problem continues, close and reopen the app.",
    ),
}

/** Main-thread owner. Activity recreation and listener retries retain the process runtime. */
internal class StartupRecovery<R, S : AutoCloseable>(
    private val createRuntime: () -> R,
    private val createServer: (R) -> S,
    private val startServer: (S) -> Unit,
) {
    var runtime: R? = null; private set
    var server: S? = null; private set
    var failure: StartupFailure? = null; private set

    fun retry() {
        if (server != null) return
        var stage = StartupFailure.DATA
        var preparedServer: S? = null
        try {
            val owner = runtime ?: createRuntime().also { runtime = it }
            stage = StartupFailure.INTERFACE
            preparedServer = createServer(owner)
            stage = StartupFailure.CONNECTION
            startServer(preparedServer)
            server = preparedServer
            failure = null
        } catch (_: Exception) {
            // A failed listener must release its handle before another attempt.
            runCatching { preparedServer?.close() }
            failure = stage
        }
    }
}
