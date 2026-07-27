package fail.acheron.floatview

import android.content.res.AssetManager
import android.util.Base64
import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.Executors
import org.json.JSONArray
import org.json.JSONObject

class LoopbackServer(private val assets: AssetManager) {
    private val executor = Executors.newCachedThreadPool()
    private val token = ByteArray(32).also(SecureRandom()::nextBytes)
        .let { Base64.encodeToString(it, Base64.URL_SAFE or Base64.NO_WRAP or Base64.NO_PADDING) }
    private var server: ServerSocket? = null
    val port: Int get() = requireNotNull(server).localPort
    val origin: String get() = "http://127.0.0.1:$port"

    fun start() {
        if (server != null) return
        val socket = ServerSocket(0, 16, InetAddress.getByName("127.0.0.1"))
        server = socket
        executor.execute {
            while (!socket.isClosed) {
                try {
                    val client = socket.accept()
                    executor.execute { handle(client) }
                } catch (_: Exception) {
                    if (!socket.isClosed) continue
                }
            }
        }
    }

    fun stop() {
        server?.close()
        server = null
        executor.shutdownNow()
    }

    private fun handle(socket: Socket) = socket.use {
        val input = BufferedInputStream(it.getInputStream())
        val output = BufferedOutputStream(it.getOutputStream())
        val headerBytes = ArrayList<Byte>()
        var tail = ""
        while (!tail.endsWith("\r\n\r\n") && headerBytes.size <= 65_536) {
            val value = input.read()
            if (value < 0) return
            headerBytes += value.toByte()
            tail = (tail + value.toChar()).takeLast(4)
        }
        val lines = headerBytes.toByteArray().decodeToString().split("\r\n")
        val target = lines.firstOrNull()?.split(' ')?.getOrNull(1) ?: return
        val headers = lines.drop(1).mapNotNull { line ->
            val index = line.indexOf(':')
            if (index < 0) null else line.substring(0, index).lowercase() to
                line.substring(index + 1).trim()
        }.toMap()

        if (target.startsWith("/api/v1/ws")) {
            val queryToken = target.substringAfter("token=", "").substringBefore('&')
            if (
                queryToken != token ||
                headers["origin"] != origin ||
                headers["upgrade"]?.lowercase() != "websocket"
            ) {
                http(output, "403 Forbidden", "text/plain", ByteArray(0))
                return
            }
            val key = headers["sec-websocket-key"] ?: return
            val accept = Base64.encodeToString(
                MessageDigest.getInstance("SHA-1")
                    .digest((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").toByteArray()),
                Base64.NO_WRAP,
            )
            output.write(
                (
                    "HTTP/1.1 101 Switching Protocols\r\n" +
                        "Upgrade: websocket\r\nConnection: Upgrade\r\n" +
                        "Sec-WebSocket-Accept: $accept\r\n\r\n"
                    ).toByteArray(),
            )
            output.flush()
            websocket(input, output)
            return
        }
        if (target == "/api/bootstrap") {
            val body = JSONObject()
                .put("apiVersion", JSONObject().put("major", 1).put("minor", 0))
                .put("webSocketUrl", "ws://127.0.0.1:$port/api/v1/ws")
                .put("sessionToken", token)
                .toString().toByteArray()
            http(output, "200 OK", "application/json", body)
            return
        }
        val relative = if (target == "/") "index.html" else target.substringBefore('?').drop(1)
        if (relative.split('/').any { segment -> segment == ".." }) {
            http(output, "403 Forbidden", "text/plain", ByteArray(0))
            return
        }
        val body = try {
            assets.open("web/$relative").use { stream -> stream.readBytes() }
        } catch (_: Exception) {
            http(output, "404 Not Found", "text/plain", ByteArray(0))
            return
        }
        val type = when (relative.substringAfterLast('.', "")) {
            "html" -> "text/html"
            "js" -> "text/javascript"
            "css" -> "text/css"
            "svg" -> "image/svg+xml"
            else -> "application/octet-stream"
        }
        http(output, "200 OK", type, body)
    }

    private fun websocket(input: BufferedInputStream, output: BufferedOutputStream) {
        var initialized = false
        while (true) {
            val first = input.read()
            val second = input.read()
            if (first < 0 || second < 0) return
            val opcode = first and 0x0f
            var length = second and 0x7f
            if (length == 126) length = (input.read() shl 8) or input.read()
            if (length == 127 || length > 65_536) return
            val mask = if ((second and 0x80) != 0) input.readNBytes(4) else ByteArray(0)
            val payload = input.readNBytes(length)
            if (payload.size != length) return
            if (mask.isNotEmpty()) payload.indices.forEach { payload[it] =
                (payload[it].toInt() xor mask[it % 4].toInt()).toByte()
            }
            when (opcode) {
                1 -> {
                    val request = try { JSONObject(payload.decodeToString()) } catch (_: Exception) { return }
                    val response = handleRequest(request, initialized)
                    if (request.optString("method") == "system.hello" && !response.has("error")) {
                        initialized = true
                    }
                    frame(output, 1, response.toString().toByteArray())
                }
                8 -> return
                9 -> frame(output, 10, payload)
            }
        }
    }

    private fun handleRequest(request: JSONObject, initialized: Boolean): JSONObject {
        val id = request.opt("id") ?: JSONObject.NULL
        val method = request.optString("method")
        fun success(result: Any) = JSONObject().put("jsonrpc", "2.0").put("id", id).put("result", result)
        fun failure(code: Int, kind: String, message: String) = JSONObject()
            .put("jsonrpc", "2.0").put("id", id)
            .put(
                "error",
                JSONObject().put("code", code).put("message", message)
                    .put("data", JSONObject().put("kind", kind).put("recoverable", false)),
            )
        if (method == "system.hello") {
            val api = request.optJSONObject("params")?.optJSONObject("api")
            if (api?.optInt("major") != 1 || api.optInt("minMinor") > 0) {
                return failure(-32001, "unsupported", "Unsupported API version")
            }
            return success(
                JSONObject()
                    .put("api", JSONObject().put("major", 1).put("minor", 0))
                    .put(
                        "backend",
                        JSONObject().put("name", "float-view-android").put("version", "0.1.0")
                            .put("platform", "android"),
                    )
                    .put(
                        "capabilities",
                        JSONArray().put(JSONObject().put("name", "system.snapshot").put("available", true)),
                    )
                    .put(
                        "limits",
                        JSONObject().put("minTelemetryIntervalMs", 50).put("maxMessageBytes", 65_536)
                            .put("maxReconnectDurationMs", 3_600_000),
                    )
                    .put("snapshot", snapshot()),
            )
        }
        if (!initialized) return failure(-32000, "invalid_state", "system.hello required")
        if (method == "system.getSnapshot") return success(snapshot())
        return failure(-32601, "unsupported", "Unknown method: $method")
    }

    private fun snapshot() = JSONObject()
        .put("revision", 0)
        .put(
            "bluetooth",
            JSONObject().put("radio", "powered_on").put("scanPermission", "granted")
                .put("connectPermission", "granted"),
        )
        .put("knownDevices", JSONArray())
        .put("activeConnection", JSONObject.NULL)
        .put("telemetrySubscriptions", JSONArray())
        .put("loggingJobs", JSONArray())

    private fun frame(output: BufferedOutputStream, opcode: Int, payload: ByteArray) {
        output.write(0x80 or opcode)
        when {
            payload.size < 126 -> output.write(payload.size)
            payload.size <= 65_535 -> {
                output.write(126)
                output.write(payload.size shr 8)
                output.write(payload.size)
            }
            else -> return
        }
        output.write(payload)
        output.flush()
    }

    private fun http(output: BufferedOutputStream, status: String, type: String, body: ByteArray) {
        output.write(
            (
                "HTTP/1.1 $status\r\nContent-Type: $type\r\nContent-Length: ${body.size}\r\n" +
                    "Cache-Control: no-store\r\nConnection: close\r\n\r\n"
                ).toByteArray(),
        )
        output.write(body)
        output.flush()
    }
}
