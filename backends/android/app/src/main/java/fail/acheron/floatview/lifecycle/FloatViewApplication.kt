package fail.acheron.floatview.lifecycle

import android.app.Application
import android.content.pm.ApplicationInfo
import fail.acheron.floatview.application.BackendRuntime
import fail.acheron.floatview.transport.ANDROID_BACKEND_PORT
import fail.acheron.floatview.transport.LoopbackServer
import fail.acheron.floatview.transport.backendStartupFailure

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
