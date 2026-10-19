package fail.acheron.floatview.ui

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.os.Message
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import java.io.ByteArrayInputStream
import java.util.concurrent.atomic.AtomicBoolean

/** A single short-lived, non-rendered popup only hands its destination to the OS. */
internal class ExternalLinkRouter(private val activity: Activity, private val origin: String) {
    private val handler = Handler(Looper.getMainLooper())
    private var popup: WebView? = null
    private var expiry: Runnable? = null

    fun open(url: String) {
        if (webNavigation(url, origin) != WebNavigation.EXTERNAL) return
        runCatching {
            activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)).addCategory(Intent.CATEGORY_BROWSABLE))
        }
    }

    fun createPopup(result: Message): Boolean {
        val transport = result.obj as? WebView.WebViewTransport ?: return false
        if (popup != null) return false
        val child = WebView(activity)
        popup = child
        val consumed = AtomicBoolean(false)
        fun consume(url: String) {
            if (url == "about:blank") return
            if (!consumed.compareAndSet(false, true)) return
            handler.post { if (popup === child) { open(url); close() } }
        }
        child.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                if (request.isForMainFrame) consume(request.url.toString())
                return true
            }
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse {
                if (request.isForMainFrame) consume(request.url.toString())
                // Never download or render external content, even if navigation wasn't intercepted.
                return WebResourceResponse("text/plain", "UTF-8", ByteArrayInputStream(byteArrayOf()))
            }
        }
        val timeout = Runnable { if (popup === child) close() }
        expiry = timeout
        handler.postDelayed(timeout, 5_000)
        transport.webView = child
        result.sendToTarget()
        return true
    }

    fun close() {
        expiry?.let(handler::removeCallbacks)
        expiry = null
        val child = popup
        popup = null
        child?.stopLoading()
        child?.destroy()
    }
}
