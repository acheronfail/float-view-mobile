package fail.acheron.floatview

import android.util.Base64
import android.util.Log
import java.io.*
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ExecutorService
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

class LocalWebSocket() {
  data class WebSocketClient(
          val socket: Socket,
          val output: OutputStream,
          val input: InputStream,
          val id: String,
          val handlers: MutableMap<String, WsHandler>
  )

  val logTag = "FloatView_LocalWebSocket"
  val webSocketClients = ConcurrentHashMap<String, WebSocketClient>()

  fun isWebsocketUpgrade(ctx: Context): Boolean {
    return ctx.path == "/ws" &&
            ctx.requestHeaders["upgrade"]?.lowercase() == "websocket" &&
            ctx.requestHeaders["connection"]?.lowercase()?.contains("upgrade") == true
  }

  fun handleWebSocketUpgrade(
          socket: Socket,
          headers: Map<String, String>,
          executor: ExecutorService,
          handlers: MutableMap<String, WsHandler>
  ) {
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
      val client = WebSocketClient(socket, output, socket.getInputStream(), clientId, handlers)
      webSocketClients[clientId] = client
      sendWebSocketMessage(client, wsMessage("socket", "welcome", buildJsonObject {}))
      executor.execute { handleWebSocketMessages(client) }
    } catch (e: Exception) {
      Log.e(logTag, "WebSocket upgrade error: ${e.message}", e)
      try {
        socket.close()
      } catch (_: Exception) {}
    }
  }

  private fun handleWebSocketMessages(client: WebSocketClient) {
    val topic = "socket"
    try {
      val buffer = ByteArray(1024)
      while (!client.socket.isClosed) {
        val bytesRead = client.input.read(buffer)
        if (bytesRead <= 0) break
        val frame = parseWebSocketFrame(buffer, bytesRead) ?: continue

        val payload = Json.decodeFromString<WebSocketCommand>(frame.payload)
        val handler = client.handlers[payload.cmd]
        if (handler == null) {
          sendWebSocketMessage(
                  client,
                  wsMessage(
                          topic,
                          "error",
                          buildJsonObject { put("message", "Unknown command: ${payload.cmd}") }
                  )
          )
          continue
        }

        handler(payload.data)
      }
    } catch (e: Exception) {
      Log.d(logTag, "WebSocket client ${client.id} disconnected: ${e.message}")
    } finally {
      webSocketClients.remove(client.id)
      try {
        client.socket.close()
      } catch (_: Exception) {}
    }
  }

  private fun sendWebSocketMessage(client: WebSocketClient, message: JsonObject) {
    try {
      val bytes = message.toString().toByteArray(StandardCharsets.UTF_8)
      val frame = createWebSocketFrame(bytes)
      client.output.write(frame)
      client.output.flush()
    } catch (e: Exception) {
      Log.e(logTag, "Send WS error: ${e.message}", e)
    }
  }

  fun broadcastMessage(message: JsonObject) {
    Thread() { webSocketClients.values.forEach { sendWebSocketMessage(it, message) } }.start()
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
      Log.w(logTag, "WS payload too large")
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
}
