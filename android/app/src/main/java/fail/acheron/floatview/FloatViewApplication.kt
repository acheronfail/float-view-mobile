package fail.acheron.floatview

import android.app.Application

class FloatViewApplication : Application() {
    lateinit var backend: LoopbackServer
        private set

    override fun onCreate() {
        super.onCreate()
        backend = LoopbackServer(assets).also { it.start() }
    }

    override fun onTerminate() {
        backend.stop()
        super.onTerminate()
    }
}
