package fail.acheron.floatview

import android.util.Base64
import android.util.Log
import java.io.*
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ExecutorService

class LocalWebSocket() {
  data class WebSocketClient(
          val socket: Socket,
          val output: OutputStream,
          val input: InputStream,
          val id: String
  )

    val TAG = "FloatView_LocalWebSocket"
  val webSocketClients = ConcurrentHashMap<String, WebSocketClient>()

    fun isWebsocketUpgrade(ctx: Context): Boolean {
        return ctx.path == "/ws" &&
                      ctx.requestHeaders["upgrade"]?.lowercase() == "websocket" &&
                      ctx.requestHeaders["connection"]?.lowercase()?.contains("upgrade") == true

    }

  fun handleWebSocketUpgrade(socket: Socket, headers: Map<String, String>, executor: ExecutorService) {
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

  fun handleWebSocketMessages(client: WebSocketClient) {
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

  fun sendWebSocketMessage(client: WebSocketClient, message: String) {
    try {
      val bytes = message.toByteArray(StandardCharsets.UTF_8)
      val frame = createWebSocketFrame(bytes)
      client.output.write(frame)
      client.output.flush()
    } catch (e: Exception) {
      Log.e(TAG, "Send WS error: ${e.message}", e)
    }
  }

  fun broadcastToOtherClients(excludeId: String, message: String) {
    webSocketClients.values.forEach { if (it.id != excludeId) sendWebSocketMessage(it, message) }
  }

  data class WebSocketFrame(val opcode: Int, val payload: String)

  fun parseWebSocketFrame(buffer: ByteArray, length: Int): WebSocketFrame? {
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

  fun createWebSocketFrame(payload: ByteArray): ByteArray {
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

  fun generateWebSocketAcceptKey(key: String): String {
    val magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    val digest =
            MessageDigest.getInstance("SHA-1")
                    .digest((key + magic).toByteArray(StandardCharsets.UTF_8))
    return Base64.encodeToString(digest, Base64.NO_WRAP)
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

    private fun isValidJson(s: String): Boolean {
    val t = s.trim()
    return t.isNotEmpty() &&
            ((t.startsWith("{") && t.endsWith("}")) || (t.startsWith("[") && t.endsWith("]")))
  }
}
