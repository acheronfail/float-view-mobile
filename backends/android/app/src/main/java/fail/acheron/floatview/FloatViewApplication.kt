package fail.acheron.floatview

import android.app.Application
import android.content.pm.ApplicationInfo

class FloatViewApplication : Application() {
    lateinit var runtime: BackendRuntime
        private set
    lateinit var backend: LoopbackServer
        private set

    override fun onCreate() {
        super.onCreate()
        runtime = BackendRuntime(this)
        backend = try {
            LoopbackServer(
                assets,
                runtime.applicationService,
                runtime.exportPresentationBroker::present,
                runtime.events,
                requestedPort = ANDROID_BACKEND_PORT,
                performanceDiagnostics = applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0,
            ).also { it.start() }
        } catch (error: Exception) {
            throw backendStartupFailure(ANDROID_BACKEND_PORT, error)
        }
    }

    override fun onTerminate() {
        backend.stop()
        super.onTerminate()
    }
}

internal fun backendStartupFailure(port: Int, cause: Exception) =
    IllegalStateException("Failed to bind Float View backend to 127.0.0.1:$port", cause)
