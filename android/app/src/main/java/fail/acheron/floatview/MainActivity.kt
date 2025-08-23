package fail.acheron.floatview

import android.os.Bundle
import android.util.Base64
import android.util.Log
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import java.io.*
import java.net.ServerSocket
import java.net.Socket
import java.nio.charset.StandardCharsets
import java.security.MessageDigest
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {

    private var serverSocket: ServerSocket? = null
    private val executor = Executors.newCachedThreadPool()
    private lateinit var webView: WebView
    private val TAG = "FloatViewApp"
    private val webSocketClients = ConcurrentHashMap<String, WebSocketClient>()

    data class WebSocketClient(
        val socket: Socket,
        val output: OutputStream,
        val input: InputStream,
        val id: String
    )

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

        // Start the local server
        startLocalServer()
    }

    private fun startLocalServer() {
        Log.d(TAG, "Starting local server...")
        executor.execute {
            try {
                // Use port 0 to let the system assign an available port
                serverSocket = ServerSocket(0)
                val actualPort = serverSocket!!.localPort
                Log.d(TAG, "Server started on port $actualPort")
                val serverUrl = "http://127.0.0.1:$actualPort"

                // Load the local server URL in WebView on main thread
                runOnUiThread {
                    Log.d(TAG, "Loading URL: $serverUrl")
                    webView.loadUrl(serverUrl)
                }

                // Accept connections
                while (!serverSocket!!.isClosed) {
                    val clientSocket = serverSocket!!.accept()
                    Log.d(TAG, "New client connection accepted")
                    executor.execute { handleRequest(clientSocket) }
                }

            } catch (e: IOException) {
                if (!e.message?.contains("Socket closed")!!) {
                    Log.e(TAG, "Server error: ${e.message}", e)
                }
            }
        }
    }

    private fun handleRequest(socket: Socket) {
        var isWebSocketUpgrade = false

        try {
            val input = BufferedReader(InputStreamReader(socket.getInputStream()))
            val output = PrintWriter(socket.getOutputStream(), true)

            // Read HTTP request
            val requestLine = input.readLine() ?: return
            val parts = requestLine.split(" ")
            if (parts.size < 2) return

            val method = parts[0]
            val path = parts[1]
            Log.d(TAG, "Handling request: $method $path")

            // Read headers
            val headers = mutableMapOf<String, String>()
            var line: String?
            do {
                line = input.readLine()
                if (line != null && line.isNotEmpty()) {
                    val colonIndex = line.indexOf(":")
                    if (colonIndex > 0) {
                        val key = line.substring(0, colonIndex).trim().lowercase()
                        val value = line.substring(colonIndex + 1).trim()
                        headers[key] = value
                    }
                }
            } while (line != null && line.isNotEmpty())

            // Check if this is a WebSocket upgrade request
            if (path == "/ws" && headers["upgrade"]?.lowercase() == "websocket" &&
                headers["connection"]?.lowercase()?.contains("upgrade") == true) {
                Log.d(TAG, "WebSocket upgrade request detected for path: $path")
                Log.d(TAG, "Headers: $headers")
                isWebSocketUpgrade = true
                handleWebSocketUpgrade(socket, headers)
                return // Don't close socket, it's handled by WebSocket
            }

            // Handle regular HTTP requests
            Log.d(TAG, "Processing regular HTTP request for path: $path")
            when {
                path == "/" -> {
                    Log.d(TAG, "Serving main page")
                    val port = serverSocket?.localPort ?: 0
                    val html = getMainHtml(port)
                    sendResponse(output, "200 OK", "text/html", html)
                }

                path == "/api/status" -> {
                    Log.d(TAG, "Serving API status")
                    val json = """{"status":"ok","message":"Local server is running"}"""
                    sendResponse(output, "200 OK", "application/json", json)
                }

                else -> {
                    Log.d(TAG, "Serving 404 for path: $path")
                    val html = get404Html()
                    sendResponse(output, "404 Not Found", "text/html", html)
                }
            }

        } catch (e: IOException) {
            Log.e(TAG, "Error handling request: ${e.message}", e)
        } finally {
            // Only close socket for regular HTTP requests, not WebSocket upgrades
            if (!isWebSocketUpgrade) {
                try {
                    socket.close()
                } catch (e: IOException) {
                    Log.e(TAG, "Error closing socket: ${e.message}", e)
                }
            }
        }
    }

    private fun handleWebSocketUpgrade(socket: Socket, headers: Map<String, String>) {
        try {
            Log.d(TAG, "Starting WebSocket handshake")
            val webSocketKey = headers["sec-websocket-key"] ?: run {
                Log.e(TAG, "Missing sec-websocket-key header")
                return
            }
            val acceptKey = generateWebSocketAcceptKey(webSocketKey)
            Log.d(TAG, "Generated WebSocket accept key: $acceptKey")

            // Send WebSocket handshake response
            val output = socket.getOutputStream()
            val response = """
                HTTP/1.1 101 Switching Protocols
                Upgrade: websocket
                Connection: Upgrade
                Sec-WebSocket-Accept: $acceptKey


            """.trimIndent().replace("\n", "\r\n")

            Log.d(TAG, "Sending WebSocket handshake response")
            output.write(response.toByteArray())
            output.flush()

            val clientId = "client_${System.currentTimeMillis()}"
            val client = WebSocketClient(socket, output, socket.getInputStream(), clientId)
            webSocketClients[clientId] = client

            Log.d(TAG, "WebSocket client connected: $clientId")

            // Send welcome message
            val welcomeMessage = createJsonMessage("welcome",
                "message" to "Connected to Float View Mobile",
                "clientId" to clientId
            )
            sendWebSocketMessage(client, welcomeMessage)

            // Handle WebSocket messages in a separate thread
            executor.execute { handleWebSocketMessages(client) }

        } catch (e: Exception) {
            Log.e(TAG, "Error handling WebSocket upgrade: ${e.message}", e)
            try {
                socket.close()
            } catch (closeException: Exception) {
                Log.e(TAG, "Error closing socket after WebSocket upgrade failure", closeException)
            }
        }
    }

    private fun generateWebSocketAcceptKey(key: String): String {
        val webSocketMagicString = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
        val combined = key + webSocketMagicString
        val digest = MessageDigest.getInstance("SHA-1").digest(combined.toByteArray(StandardCharsets.UTF_8))
        return Base64.encodeToString(digest, Base64.NO_WRAP)
    }

    private fun handleWebSocketMessages(client: WebSocketClient) {
        try {
            val buffer = ByteArray(1024)
            while (!client.socket.isClosed) {
                val bytesRead = client.input.read(buffer)
                if (bytesRead <= 0) break

                val frame = parseWebSocketFrame(buffer, bytesRead)
                if (frame != null) {
                    Log.d(TAG, "Received WebSocket JSON message from ${client.id}: ${frame.payload}")

                    // Validate JSON format
                    if (!isValidJson(frame.payload)) {
                        Log.w(TAG, "Invalid JSON received from ${client.id}, sending error response")
                        val errorMessage = createJsonMessage("error", "message" to "Invalid JSON format")
                        sendWebSocketMessage(client, errorMessage)
                        continue
                    }

                    // Echo the message back with additional info
                    val echoMessage = """{"type":"echo","original":${frame.payload},"timestamp":${System.currentTimeMillis()},"from":"server"}"""
                    sendWebSocketMessage(client, echoMessage)

                    // Broadcast to all other clients
                    val broadcastMessage = """{"type":"broadcast","message":${frame.payload},"from":"${client.id}"}"""
                    broadcastToOtherClients(client.id, broadcastMessage)
                }
            }
        } catch (e: Exception) {
            Log.d(TAG, "WebSocket client ${client.id} disconnected: ${e.message}")
        } finally {
            webSocketClients.remove(client.id)
            try {
                client.socket.close()
            } catch (e: Exception) {
                Log.e(TAG, "Error closing WebSocket client socket", e)
            }
            Log.d(TAG, "WebSocket client ${client.id} cleaned up")
        }
    }

    // Simple JSON validation - checks basic structure
    private fun isValidJson(jsonString: String): Boolean {
        val trimmed = jsonString.trim()
        if (trimmed.isEmpty()) return false

        // Basic validation for JSON objects and arrays
        return (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
               (trimmed.startsWith("[") && trimmed.endsWith("]"))
    }

    // Helper methods for creating JSON responses
    private fun createJsonMessage(type: String, vararg pairs: Pair<String, Any>): String {
        val jsonBuilder = StringBuilder()
        jsonBuilder.append("{\"type\":\"$type\"")

        pairs.forEach { (key, value) ->
            jsonBuilder.append(",\"$key\":")
            when (value) {
                is String -> jsonBuilder.append("\"$value\"")
                is Number -> jsonBuilder.append(value)
                else -> jsonBuilder.append("\"$value\"")
            }
        }

        jsonBuilder.append("}")
        return jsonBuilder.toString()
    }

    private data class WebSocketFrame(val opcode: Int, val payload: String)

    private fun parseWebSocketFrame(buffer: ByteArray, length: Int): WebSocketFrame? {
        if (length < 2) return null

        val firstByte = buffer[0].toInt() and 0xFF
        val secondByte = buffer[1].toInt() and 0xFF

        val opcode = firstByte and 0x0F

        // Only handle text frames (opcode 1) for JSON messages
        if (opcode != 1) {
            Log.d(TAG, "Ignoring non-text WebSocket frame with opcode: $opcode")
            return null
        }

        val masked = (secondByte and 0x80) != 0
        var payloadLength = (secondByte and 0x7F)

        var offset = 2

        // Handle extended payload length (simplified - only support up to 65535 bytes)
        if (payloadLength == 126) {
            if (length < offset + 2) return null
            payloadLength = ((buffer[offset].toInt() and 0xFF) shl 8) or (buffer[offset + 1].toInt() and 0xFF)
            offset += 2
        } else if (payloadLength == 127) {
            // Don't handle large payloads for simplicity
            Log.w(TAG, "WebSocket payload too large, ignoring")
            return null
        }

        // Extract masking key (required for client-to-server messages)
        val maskingKey = if (masked) {
            if (length < offset + 4) return null
            val key = ByteArray(4)
            System.arraycopy(buffer, offset, key, 0, 4)
            offset += 4
            key
        } else null

        // Extract payload
        if (length < offset + payloadLength) return null
        val payload = ByteArray(payloadLength)
        System.arraycopy(buffer, offset, payload, 0, payloadLength)

        // Unmask payload (always required for client messages)
        if (masked && maskingKey != null) {
            for (i in payload.indices) {
                payload[i] = (payload[i].toInt() xor maskingKey[i % 4].toInt()).toByte()
            }
        }

        val payloadString = String(payload, StandardCharsets.UTF_8)

        // Validate that it's valid JSON (basic check)
        if (!payloadString.trim().startsWith("{") && !payloadString.trim().startsWith("[")) {
            Log.w(TAG, "WebSocket message is not JSON, ignoring: $payloadString")
            return null
        }

        return WebSocketFrame(opcode, payloadString)
    }

    private fun sendWebSocketMessage(client: WebSocketClient, message: String) {
        try {
            val messageBytes = message.toByteArray(StandardCharsets.UTF_8)
            val frame = createWebSocketFrame(messageBytes)
            client.output.write(frame)
            client.output.flush()
        } catch (e: Exception) {
            Log.e(TAG, "Error sending WebSocket message to ${client.id}: ${e.message}", e)
        }
    }

    private fun broadcastToOtherClients(excludeClientId: String, message: String) {
        webSocketClients.values.forEach { client ->
            if (client.id != excludeClientId) {
                sendWebSocketMessage(client, message)
            }
        }
    }

    private fun createWebSocketFrame(payload: ByteArray): ByteArray {
        val payloadLength = payload.size

        // For simplicity, only support payloads up to 65535 bytes
        val headerSize = when {
            payloadLength < 126 -> 2
            payloadLength < 65536 -> 4
            else -> {
                Log.w(TAG, "WebSocket payload too large: $payloadLength bytes")
                return ByteArray(0) // Return empty frame for oversized payloads
            }
        }

        val frame = ByteArray(headerSize + payloadLength)
        var offset = 0

        // First byte: FIN=1, RSV=000, Opcode=0001 (text frame)
        frame[offset++] = 0x81.toByte()

        // Second byte and payload length (server frames are never masked)
        when {
            payloadLength < 126 -> {
                frame[offset++] = payloadLength.toByte()
            }
            else -> {
                frame[offset++] = 126.toByte()
                frame[offset++] = (payloadLength shr 8).toByte()
                frame[offset++] = (payloadLength and 0xFF).toByte()
            }
        }

        // Copy payload (no masking for server-to-client frames)
        System.arraycopy(payload, 0, frame, offset, payloadLength)

        return frame
    }

    private fun sendResponse(output: PrintWriter, status: String, contentType: String, body: String) {
        output.println("HTTP/1.1 $status")
        output.println("Content-Type: $contentType")
        output.println("Content-Length: ${body.toByteArray().size}")
        output.println("Connection: close")
        output.println()
        output.print(body)
        output.flush()
    }

    private fun getMainHtml(port: Int): String {
        return try {
            val inputStream = assets.open("web/index.html")
            val html = inputStream.bufferedReader().use { it.readText() }
            // Simple template replacement for port
            html.replace("{{PORT}}", port.toString())
        } catch (e: IOException) {
            Log.e(TAG, "Error reading index.html: ${e.message}", e)
            getFallbackMainHtml(port)
        }
    }

    private fun getFallbackMainHtml(port: Int): String {
        return """
            <!DOCTYPE html>
            <html>
            <head><title>Float View Mobile</title></head>
            <body>
                <h1>Hello</h1>
                <p>Welcome to your local server!</p>
                <p>Server is running on port $port</p>
                <p><em>Error loading main template</em></p>
            </body>
            </html>
        """.trimIndent()
    }

    private fun get404Html(): String {
        return try {
            val inputStream = assets.open("web/404.html")
            inputStream.bufferedReader().use { it.readText() }
        } catch (e: IOException) {
            Log.e(TAG, "Error reading 404.html: ${e.message}", e)
            """
                <!DOCTYPE html>
                <html>
                <head><title>Not Found</title></head>
                <body><h1>404 - Not Found</h1><p>The requested resource was not found.</p></body>
                </html>
            """.trimIndent()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            // Close all WebSocket connections
            webSocketClients.values.forEach { client ->
                try {
                    client.socket.close()
                } catch (e: Exception) {
                    Log.e(TAG, "Error closing WebSocket client", e)
                }
            }
            webSocketClients.clear()

            serverSocket?.close()
            executor.shutdown()
        } catch (e: IOException) {
            e.printStackTrace()
        }
    }
}
