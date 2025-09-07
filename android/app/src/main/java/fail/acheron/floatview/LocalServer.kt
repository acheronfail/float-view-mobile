package fail.acheron.floatview

import android.app.Activity
import android.webkit.WebView
import java.io.*
import java.lang.ref.WeakReference
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors
import kotlinx.serialization.json.JsonObject

object LocalServer {
  private val webSocket = LocalWebSocket()
  private var serverSocket: ServerSocket? = null
  private val executor = Executors.newCachedThreadPool()
  private var webViewRef: WeakReference<WebView>? = null
  private var activityRef: WeakReference<Activity>? = null
  private var started = false
  private val routes = ConcurrentHashMap<String, (LocalWebContext) -> Unit>()

  fun setHandler(handler: (cmd: BackendCommand) -> Unit) {
    webSocket.setHandler(handler)
  }

  fun broadcastMessage(message: JsonObject) {
    webSocket.broadcastMessage(message)
  }

  @Synchronized
  fun start(activity: Activity, webView: WebView) {
    if (started) {
      printError("LocalServer already started; ignoring start() call")
      return
    }

    webViewRef = WeakReference(webView)
    activityRef = WeakReference(activity)
    executor.execute {
      try {
        serverSocket = ServerSocket(0)
        val port = serverSocket!!.localPort
        print("LocalServer listening on port $port")
        val url = "http://127.0.0.1:$port"
        activity.runOnUiThread {
          print("LocalServer loading URL: $url")
          webView.loadUrl(url)
        }
        started = true
        acceptLoop()
      } catch (e: IOException) {
        printError("LocalServer failed to start: ${e.message}", e)
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

        printError("Accept error: ${e.message}", e)
      }
    }
  }

  private fun handleRequest(socket: Socket) {
    var isWebSocketUpgrade = false
    var handler: ((LocalWebContext) -> Unit)? = null
    try {
      val ctx = LocalWebContext(socket)

      if (webSocket.isWebsocketUpgrade(ctx)) {
        isWebSocketUpgrade = true
        webSocket.handleWebSocketUpgrade(socket, ctx.requestHeaders, executor)
        return
      }

      handler = routes[ctx.path]
      if (handler != null) {
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
      printError("Request error: ${e.message}", e)
    } finally {
      if (!isWebSocketUpgrade && handler == null) {
        try {
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
    try {
      val content = activity.assets.open(webPath).bufferedReader().use { it.readText() }
      val contentType = getMimeType(cleanPath)
      print("Successfully served static file: $webPath as $contentType")
      return StaticFileResponse(content, contentType)
    } catch (e: IOException) {
      printError("Static file not found: $webPath (${e.message})")
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
      printError("index.html read error: ${e.message}", e)
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
    print("LocalServer stopping...")
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
