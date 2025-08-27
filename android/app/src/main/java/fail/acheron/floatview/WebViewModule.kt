package fail.acheron.floatview

import android.annotation.SuppressLint
import android.app.Activity
import android.util.Log
import android.webkit.WebView
import android.webkit.WebViewClient

class WebViewModule(private val activity: Activity) {

    companion object {
        private const val TAG = "FloatView_WebViewModule"
    }

    lateinit var webView: WebView
        private set

    @SuppressLint("SetJavaScriptEnabled")
    fun init(): WebView {
        if (this::webView.isInitialized) return webView

        Log.d(TAG, "Initializing WebView")
        webView = WebView(activity)
        webView.settings.javaScriptEnabled = true
        webView.settings.domStorageEnabled = true
        webView.settings.allowFileAccess = false
        webView.settings.allowContentAccess = false
        webView.webViewClient = object : WebViewClient() {
            override fun onReceivedError(
                view: WebView?,
                errorCode: Int,
                description: String?,
                failingUrl: String?
            ) {
                Log.e(TAG, "WebView error: $errorCode - $description for URL: $failingUrl")
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                Log.d(TAG, "Page finished loading: $url")
            }
        }
        return webView
    }

    fun destroy() {
        if (!this::webView.isInitialized) return
        try {
            Log.d(TAG, "Destroying WebView")
            webView.stopLoading()
            webView.webChromeClient = null
            webView.destroy()
        } catch (e: Exception) {
            Log.w(TAG, "Error destroying WebView: ${e.message}", e)
        }
    }
}
