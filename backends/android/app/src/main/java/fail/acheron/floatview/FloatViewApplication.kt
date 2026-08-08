package fail.acheron.floatview

import android.app.Application
import fail.acheron.floatview.host.FloatViewRuntimeBridge

class FloatViewApplication : Application() {
    internal lateinit var rustRuntime: FloatViewRuntimeBridge
        private set

    override fun onCreate() {
        super.onCreate()
        rustRuntime = FloatViewRuntimeBridge(this)
    }

    override fun onTerminate() {
        rustRuntime.close()
        super.onTerminate()
    }
}
