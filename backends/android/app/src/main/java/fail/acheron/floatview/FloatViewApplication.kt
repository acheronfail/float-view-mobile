package fail.acheron.floatview

import android.app.Application
import fail.acheron.floatview.core.RustBackendRuntime

class FloatViewApplication : Application() {
    internal lateinit var rustRuntime: RustBackendRuntime
        private set

    override fun onCreate() {
        super.onCreate()
        rustRuntime = RustBackendRuntime(this)
    }

    override fun onTerminate() {
        rustRuntime.close()
        super.onTerminate()
    }
}
