package fail.acheron.floatview

import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import android.webkit.WebViewClient
import android.bluetooth.BluetoothDevice
import androidx.appcompat.app.AppCompatActivity
import java.util.UUID

class MainActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private val TAG = "FloatViewApp"
    // Minimal integration of BluetoothModule
    private lateinit var bluetoothModule: BluetoothModule


    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        Log.d(TAG, "MainActivity onCreate started")

        // Create and configure WebView
        webView = WebView(this)
        webView.settings.javaScriptEnabled = true
        webView.settings.domStorageEnabled = true
        webView.settings.allowFileAccess = false
        webView.settings.allowContentAccess = false
        webView.webViewClient = object : WebViewClient() {
            override fun onReceivedError(view: WebView?, errorCode: Int, description: String?, failingUrl: String?) {
                Log.e(TAG, "WebView error: $errorCode - $description for URL: $failingUrl")
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                Log.d(TAG, "Page finished loading: $url")
            }
        }

        setContentView(webView)

    // Start the singleton local server
    LocalServer.start(this, webView)

    // Initialize Bluetooth module (permissions may prompt user)
    bluetoothModule = BluetoothModule(this)
    // If permissions already granted, start scan immediately; otherwise request then scan in callback
    if (bluetoothModule.requestPermissionsIfNeeded(this, 1001)) {
        startBleScan()
    }
    }

    private val targetServiceUuid: UUID = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")

    private fun startBleScan() {
        bluetoothModule.startScan(listOf(targetServiceUuid), 10_000L, object : BluetoothModule.ScanListener {
            override fun onDeviceFound(device: BluetoothDevice) {
                Log.d(TAG, "BLE device with target service found: ${'$'}{device.address} (${ '$'}{device.name})")
            }

            override fun onScanStopped(devices: List<BluetoothDevice>) {
                Log.d(TAG, "BLE scan complete. Devices count: ${'$'}{devices.size}")
            }

            override fun onError(message: String) {
                Log.e(TAG, "BLE scan error: ${'$'}message")
            }
        })
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == 1001) {
            val allGranted = grantResults.isNotEmpty() && grantResults.all { it == android.content.pm.PackageManager.PERMISSION_GRANTED }
            if (allGranted) {
                Log.d(TAG, "All BLE permissions granted; starting scan")
                startBleScan()
            } else {
                Log.w(TAG, "BLE permissions denied; cannot start scan")
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
    LocalServer.stop()
    }
}
