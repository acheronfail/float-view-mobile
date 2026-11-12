package fail.acheron.floatview.lifecycle

import android.app.Application
import android.content.pm.ApplicationInfo
import fail.acheron.floatview.application.BackendRuntime
import fail.acheron.floatview.transport.LoopbackServer
import fail.acheron.floatview.application.StartupRecovery

class FloatViewApplication : Application() {
    private val startup by lazy {
        StartupRecovery(
            createRuntime = { BackendRuntime(this) },
            createServer = { runtime: BackendRuntime ->
                LoopbackServer(
                    assets, runtime.service,
                    performanceDiagnostics = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0,
                )
            },
            startServer = {
                it.start()
                if (applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0) {
                    java.io.File(filesDir, "backend-port").writeText(it.port.toString())
                }
            },
        )
    }
    val runtime: BackendRuntime get() = requireNotNull(startup.runtime)
    val runtimeOrNull: BackendRuntime? get() = startup.runtime
    val backend: LoopbackServer get() = requireNotNull(startup.server)
    val startupFailure get() = startup.failure

    override fun onCreate() {
        super.onCreate()
        java.io.File(filesDir, "backend-port").delete()
        retryStartup()
    }

    fun retryStartup() = startup.retry()

    override fun onTerminate() {
        startup.server?.close()
        startup.runtime?.service?.close()
        super.onTerminate()
    }
}
