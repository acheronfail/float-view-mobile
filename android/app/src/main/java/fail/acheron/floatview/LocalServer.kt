package fail.acheron.floatview

import android.app.Activity
import android.util.Base64
import android.util.Log
import android.webkit.WebView
import java.io.*
import java.lang.ref.WeakReference
import java.net.ServerSocket
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

object LocalServer {

  private const val TAG = "FloatView_LocalServer"

  // --- State ---
  private var serverSocket: ServerSocket? = null
  private val executor = Executors.newCachedThreadPool()
  private val webSocketClients = ConcurrentHashMap<String, WebSocketClient>()
  private var webViewRef: WeakReference<WebView>? = null
  private var activityRef: WeakReference<Activity>? = null
  private var started = false
  private val routes = ConcurrentHashMap<String, (LocalServerContext) -> Unit>()

  data class WebSocketClient(
          val socket: Socket,
          val output: OutputStream,
          val input: InputStream,
          val id: String
  )

  fun addRoute(name: String, handler: (LocalServerContext) -> Unit) {
    routes.put(name, handler);
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
    var handler: ((LocalServerContext) -> Unit)? = null
    try {
      val input = BufferedReader(InputStreamReader(socket.getInputStream()))
      val output = PrintWriter(socket.getOutputStream(), true)
      val requestLine = input.readLine() ?: return
      val parts = requestLine.split(" ")
      if (parts.size < 2) return
      val method = parts[0]
      val path = parts[1]
      Log.d(TAG, "Request: $method $path")

      // Headers
      val headers = mutableMapOf<String, String>()
      var line: String?
      do {
        line = input.readLine()
        if (line != null && line.isNotEmpty()) {
          val idx = line.indexOf(":")
          if (idx > 0) {
            val k = line.substring(0, idx).trim().lowercase()
            val v = line.substring(idx + 1).trim()
            headers[k] = v
          }
        }
      } while (line != null && line.isNotEmpty())

      if (path == "/ws" &&
                      headers["upgrade"]?.lowercase() == "websocket" &&
                      headers["connection"]?.lowercase()?.contains("upgrade") == true
      ) {
        isWebSocketUpgrade = true
        handleWebSocketUpgrade(socket, headers)
        return
      }

      val ctx = LocalServerContext(socket, output)
      handler = routes[path]
      if (handler != null) {
        Log.d(TAG, "Running handler!")
        handler(ctx)
        return
      }

      when (path) {
        "/" -> {
          val port = serverSocket?.localPort ?: 0
          val html = getMainHtml(port)
          sendResponse(output, "200 OK", "text/html", html)
        }
        else -> {
          // Try to serve static file from web directory
          val staticFileResponse = tryServeStaticFile(path)
          if (staticFileResponse != null) {
            sendResponse(output, "200 OK", staticFileResponse.contentType, staticFileResponse.content)
          } else {
            sendResponse(output, "404 Not Found", "text/html", get404Html())
          }
        }
      }
    } catch (e: IOException) {
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

  private fun handleWebSocketUpgrade(socket: Socket, headers: Map<String, String>) {
    try {
      val key = headers["sec-websocket-key"] ?: return
      val acceptKey = generateWebSocketAcceptKey(key)
      val output = socket.getOutputStream()
      val response =
              """
                HTTP/1.1 101 Switching Protocols
                Upgrade: websocket
                Connection: Upgrade
                Sec-WebSocket-Accept: $acceptKey


            """
                      .trimIndent()
                      .replace("\n", "\r\n")
      output.write(response.toByteArray())
      output.flush()
      val clientId = "client_${System.currentTimeMillis()}"
      val client = WebSocketClient(socket, output, socket.getInputStream(), clientId)
      webSocketClients[clientId] = client
      sendWebSocketMessage(
              client,
              createJsonMessage("welcome", "clientId" to clientId, "message" to "Connected")
      )
      executor.execute { handleWebSocketMessages(client) }
    } catch (e: Exception) {
      Log.e(TAG, "WebSocket upgrade error: ${e.message}", e)
      try {
        socket.close()
      } catch (_: Exception) {}
    }
  }

  private fun handleWebSocketMessages(client: WebSocketClient) {
    try {
      val buffer = ByteArray(1024)
      while (!client.socket.isClosed) {
        val bytesRead = client.input.read(buffer)
        if (bytesRead <= 0) break
        val frame = parseWebSocketFrame(buffer, bytesRead) ?: continue
        if (!isValidJson(frame.payload)) {
          sendWebSocketMessage(client, createJsonMessage("error", "message" to "Invalid JSON"))
          continue
        }
        val echo =
                """{"type":"echo","original":${frame.payload},"timestamp":${System.currentTimeMillis()}}"""
        sendWebSocketMessage(client, echo)
        val broadcast = """{"type":"broadcast","message":${frame.payload},"from":"${client.id}"}"""
        broadcastToOtherClients(client.id, broadcast)
      }
    } catch (e: Exception) {
      Log.d(TAG, "WebSocket client ${client.id} disconnected: ${e.message}")
    } finally {
      webSocketClients.remove(client.id)
      try {
        client.socket.close()
      } catch (_: Exception) {}
    }
  }

  private fun sendWebSocketMessage(client: WebSocketClient, message: String) {
    try {
      val bytes = message.toByteArray(StandardCharsets.UTF_8)
      val frame = createWebSocketFrame(bytes)
      client.output.write(frame)
      client.output.flush()
    } catch (e: Exception) {
      Log.e(TAG, "Send WS error: ${e.message}", e)
    }
  }

  private fun broadcastToOtherClients(excludeId: String, message: String) {
    webSocketClients.values.forEach { if (it.id != excludeId) sendWebSocketMessage(it, message) }
  }

  private data class WebSocketFrame(val opcode: Int, val payload: String)

  private fun parseWebSocketFrame(buffer: ByteArray, length: Int): WebSocketFrame? {
    if (length < 2) return null
    val first = buffer[0].toInt() and 0xFF
    val second = buffer[1].toInt() and 0xFF
    val opcode = first and 0x0F
    if (opcode != 1) return null // text only
    val masked = (second and 0x80) != 0
    var payloadLength = (second and 0x7F)
    var offset = 2
    if (payloadLength == 126) {
      if (length < offset + 2) return null
      payloadLength =
              ((buffer[offset].toInt() and 0xFF) shl 8) or (buffer[offset + 1].toInt() and 0xFF)
      offset += 2
    } else if (payloadLength == 127) {
      Log.w(TAG, "WS payload too large")
      return null
    }
    val maskingKey =
            if (masked) {
              if (length < offset + 4) return null
              ByteArray(4).also {
                System.arraycopy(buffer, offset, it, 0, 4)
                offset += 4
              }
            } else null
    if (length < offset + payloadLength) return null
    val payload = ByteArray(payloadLength)
    System.arraycopy(buffer, offset, payload, 0, payloadLength)
    if (masked && maskingKey != null) {
      for (i in payload.indices) payload[i] =
              (payload[i].toInt() xor maskingKey[i % 4].toInt()).toByte()
    }
    val str = String(payload, StandardCharsets.UTF_8)
    if (!str.trim().startsWith("{") && !str.trim().startsWith("[")) return null
    return WebSocketFrame(opcode, str)
  }

  private fun createWebSocketFrame(payload: ByteArray): ByteArray {
    val length = payload.size
    val headerSize =
            when {
              length < 126 -> 2
              length < 65536 -> 4
              else -> return ByteArray(0)
            }
    val frame = ByteArray(headerSize + length)
    var o = 0
    frame[o++] = 0x81.toByte()
    when {
      length < 126 -> frame[o++] = length.toByte()
      else -> {
        frame[o++] = 126
        frame[o++] = (length shr 8).toByte()
        frame[o++] = (length and 0xFF).toByte()
      }
    }
    System.arraycopy(payload, 0, frame, o, length)
    return frame
  }

  private fun generateWebSocketAcceptKey(key: String): String {
    val magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    val digest =
            MessageDigest.getInstance("SHA-1")
                    .digest((key + magic).toByteArray(StandardCharsets.UTF_8))
    return Base64.encodeToString(digest, Base64.NO_WRAP)
  }

  private fun isValidJson(s: String): Boolean {
    val t = s.trim()
    return t.isNotEmpty() &&
            ((t.startsWith("{") && t.endsWith("}")) || (t.startsWith("[") && t.endsWith("]")))
  }

  private fun createJsonMessage(type: String, vararg pairs: Pair<String, Any>): String {
    val b = StringBuilder().append("{\"type\":\"$type\"")
    pairs.forEach { (k, v) ->
      b.append(",\"$k\":")
      when (v) {
        is String -> b.append("\"$v\"")
        is Number -> b.append(v)
        else -> b.append("\"$v\"")
      }
    }
    return b.append("}").toString()
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

  private fun sendResponse(out: PrintWriter, status: String, contentType: String, body: String) {
    out.println("HTTP/1.1 $status")
    out.println("Content-Type: $contentType")
    out.println("Content-Length: ${body.toByteArray().size}")
    out.println("Connection: close")
    out.println()
    out.print(body)
    out.flush()
  }

  private fun getMainHtml(port: Int): String {
    val activity = activityRef?.get() ?: return getFallbackMainHtml(port)
    return try {
      // FIXME: port interpolation here needs to be updated for svelte
      val html = activity.assets.open("web/index.html").bufferedReader().use { it.readText() }
      html.replace("{{PORT}}", port.toString())
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
    webSocketClients.values.forEach { c ->
      try {
        c.socket.close()
      } catch (_: Exception) {}
    }
    webSocketClients.clear()
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
