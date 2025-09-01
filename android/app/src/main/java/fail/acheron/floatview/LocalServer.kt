package fail.acheron.floatview

import android.app.Activity
import android.util.Log
import android.webkit.WebView
import java.io.*
import java.lang.ref.WeakReference
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import kotlinx.serialization.json.JsonObject

typealias WsHandler = (req: JsonObject) -> Unit

object LocalServer {
  private const val TAG = "FloatView_LocalServer"

  val webSocket = LocalWebSocket()
  private var serverSocket: ServerSocket? = null
  private val executor = Executors.newCachedThreadPool()
  private var webViewRef: WeakReference<WebView>? = null
  private var activityRef: WeakReference<Activity>? = null
  private var started = false
  private val routes = ConcurrentHashMap<String, (Context) -> Unit>()
  private val wsHandlers = mutableMapOf<String, WsHandler>()

  fun addRoute(path: String, handler: (Context) -> Unit) {
    routes[path] = handler
  }

  fun addHandler(commandName: String, handler: WsHandler) {
    wsHandlers[commandName] = handler
  }

  @Synchronized
  fun start(activity: Activity, webView: WebView) {
    if (started) {
      Log.d(TAG, "LocalServer already started; ignoring start() call")
      return
    }

    Log.d(TAG, "LocalServer starting...")
    webViewRef = WeakReference(webView)
    activityRef = WeakReference(activity)
    executor.execute {
      try {
        serverSocket = ServerSocket(0)
        val port = serverSocket!!.localPort
        Log.d(TAG, "LocalServer listening on port $port")
        val url = "http://127.0.0.1:$port"
        activity.runOnUiThread {
          Log.d(TAG, "LocalServer loading URL: $url")
          webView.loadUrl(url)
        }
        started = true
        acceptLoop()
      } catch (e: IOException) {
        Log.e(TAG, "LocalServer failed to start: ${e.message}", e)
      }
    }
  }

  private fun acceptLoop() {
    while (true) {
      val ss = serverSocket ?: break
      if (ss.isClosed) break
      try {
        val client = ss.accept()
        executor.execute { handleRequest(client) }
      } catch (e: IOException) {
        if (serverSocket?.isClosed == true) break

        Log.e(TAG, "Accept error: ${e.message}", e)
      }
    }
  }

  private fun handleRequest(socket: Socket) {
    var isWebSocketUpgrade = false
    var handler: ((Context) -> Unit)? = null
    try {
      val ctx = Context(socket)

      if (webSocket.isWebsocketUpgrade(ctx)) {
        isWebSocketUpgrade = true
        webSocket.handleWebSocketUpgrade(socket, ctx.requestHeaders, executor, wsHandlers)
        return
      }

      handler = routes[ctx.path]
      if (handler != null) {
        Log.d(TAG, "Running handler!")
        handler(ctx)
        return
      }

      when (ctx.path) {
        "/" -> {
          val port = serverSocket?.localPort ?: 0
          ctx.setHtml(getMainHtml(port))
          ctx.send()
        }
        else -> {
          val staticFileResponse = tryServeStaticFile(ctx.path)
          if (staticFileResponse != null) {
            ctx.setText(staticFileResponse.contentType, staticFileResponse.content)
          } else {
            ctx.setHtml(get404Html())
          }
          ctx.send()
        }
      }
    } catch (e: Exception) {
      Log.e(TAG, "Request error: ${e.message}", e)
    } finally {
      if (!isWebSocketUpgrade && handler == null) {
        try {
          Log.d(TAG, "Closing request!")
          socket.close()
        } catch (_: Exception) {}
      }
    }
  }

  private data class StaticFileResponse(val content: String, val contentType: String)

  private fun tryServeStaticFile(path: String): StaticFileResponse? {
    val activity = activityRef?.get() ?: return null

    // Clean up the path and remove leading slash, query params, and fragments
    var cleanPath = path.removePrefix("/")

    // Remove query parameters and fragments
    val queryIndex = cleanPath.indexOf('?')
    if (queryIndex != -1) {
      cleanPath = cleanPath.substring(0, queryIndex)
    }
    val fragmentIndex = cleanPath.indexOf('#')
    if (fragmentIndex != -1) {
      cleanPath = cleanPath.substring(0, fragmentIndex)
    }

    // Skip empty paths
    if (cleanPath.isEmpty()) {
      return null
    }

    val webPath = "web/$cleanPath"
    Log.d(TAG, "Trying to serve static file: $webPath")

    try {
      val content = activity.assets.open(webPath).bufferedReader().use { it.readText() }
      val contentType = getMimeType(cleanPath)
      Log.d(TAG, "Successfully served static file: $webPath as $contentType")
      return StaticFileResponse(content, contentType)
    } catch (e: IOException) {
      Log.d(TAG, "Static file not found: $webPath (${e.message})")
      return null
    }
  }

  private fun getMimeType(filePath: String): String {
    val extension = filePath.substringAfterLast('.', "").lowercase()
    return when (extension) {
      "html" -> "text/html"
      "css" -> "text/css"
      "js" -> "application/javascript"
      "json" -> "application/json"
      "png" -> "image/png"
      "jpg", "jpeg" -> "image/jpeg"
      "gif" -> "image/gif"
      "svg" -> "image/svg+xml"
      "ico" -> "image/x-icon"
      "txt" -> "text/plain"
      "xml" -> "application/xml"
      "pdf" -> "application/pdf"
      "woff" -> "font/woff"
      "woff2" -> "font/woff2"
      "ttf" -> "font/ttf"
      "eot" -> "application/vnd.ms-fontobject"
      else -> "application/octet-stream"
    }
  }

  private fun getMainHtml(port: Int): String {
    val activity = activityRef?.get() ?: return getFallbackMainHtml(port)
    return try {
      activity.assets.open("web/index.html").bufferedReader().use { it.readText() }
    } catch (e: IOException) {
      Log.e(TAG, "index.html read error: ${e.message}", e)
      getFallbackMainHtml(port)
    }
  }

  private fun getFallbackMainHtml(port: Int): String =
          """
        <!DOCTYPE html><html><head><title>Float View Mobile</title></head>
        <body><h1>Float View Mobile</h1><p>Server running on port $port</p></body></html>
    """.trimIndent()

  private fun get404Html(): String {
    return """<!DOCTYPE html><html><body><h1>404 - Not Found</h1></body></html>""".trimIndent()
  }

  @Synchronized
  fun stop() {
    if (!started) return
    Log.d(TAG, "LocalServer stopping...")
    // Close WS clients
    webSocket.webSocketClients.values.forEach { c ->
      try {
        c.socket.close()
      } catch (_: Exception) {}
    }
    webSocket.webSocketClients.clear()
    try {
      serverSocket?.close()
    } catch (_: Exception) {}
    serverSocket = null
    started = false
    try {
      executor.shutdownNow()
    } catch (_: Exception) {}
  }
}
