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
        backend = LoopbackServer(
            assets,
            runtime.applicationService,
            runtime.exportPresentationBroker::present,
            runtime.events,
            requestedPort = if (isEmulator() || isDebuggable()) 8765 else 0,
        ).also { it.start() }
    }

    override fun onTerminate() {
        backend.stop()
        super.onTerminate()
    }

    private fun isDebuggable(): Boolean =
        applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
}
