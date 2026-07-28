package fail.acheron.floatview

import android.app.Application

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
        ).also { it.start() }
    }

    override fun onTerminate() {
        backend.stop()
        super.onTerminate()
    }
}
