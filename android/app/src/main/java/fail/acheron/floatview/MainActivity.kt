package fail.acheron.floatview

import android.annotation.SuppressLint
import android.os.Bundle
import android.webkit.WebView
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {
    private var webView: WebView? = null

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as FloatViewApplication
        app.runtime.permissionBroker.attach(this)
        app.runtime.exportPresentationBroker.attach(this)
        val backend = app.backend
        webView = WebView(this).also {
            it.settings.javaScriptEnabled = true
            it.loadUrl(backend.origin)
            setContentView(it)
        }
    }

    override fun onDestroy() {
        val runtime = (application as FloatViewApplication).runtime
        runtime.permissionBroker.detach(this)
        runtime.exportPresentationBroker.detach(this)
        webView?.destroy()
        webView = null
        super.onDestroy()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == BluetoothPermissionBroker.BLUETOOTH_PERMISSION_REQUEST) {
            val runtime = (application as FloatViewApplication).runtime
            runtime.permissionBroker.recordResult(permissions)
            runtime.applicationService.onBluetoothEvent(
                fail.acheron.floatview.core.BluetoothEvent.StateChanged(
                    runtime.bluetooth.currentState(),
                ),
            )
        }
    }
}
