package fail.acheron.floatview

import android.util.Log
import kotlinx.serialization.json.JsonObject
import java.io.*
import java.io.PrintWriter
import java.net.Socket

class LocalServerContext(private val socket: Socket) {
    private val TAG = "FloatView_LocalServerContext"

    var status = 404
    val requestHeaders = mutableMapOf<String, String>()
    val responseHeaders = mutableMapOf<String, String>()
    private var isSent = false
    private var body = ""
    private var requestBody: String? = null

    private val input = BufferedReader(InputStreamReader(socket.getInputStream()))
    private val output = PrintWriter(socket.getOutputStream(), true)

    val method: String
    val path: String

    init {
        val requestLine = input.readLine()
        require(requestLine != null) { "Invalid request line" }

        val parts = requestLine.split(" ")
        require(parts.size >= 2) { "Invalid request line parts: $parts" }

        method = parts[0]
        path = parts[1]
        Log.d(TAG, "Request: $method $path")

        // Headers
        var line: String?
        do {
            line = input.readLine()
            if (line != null && line.isNotEmpty()) {
                val idx = line.indexOf(":")
                if (idx > 0) {
                    val k = line.substring(0, idx).trim().lowercase()
                    val v = line.substring(idx + 1).trim()
                    requestHeaders[k] = v
                }
            }
        } while (line != null && line.isNotEmpty())
    }

    fun isSent(): Boolean {
        return isSent
    }

    fun getBody(): String {
        if (requestBody != null) {
            return requestBody!!
        }

        // Parse body if content type is application/json
        val contentType = requestHeaders["content-type"]
        if (contentType != null && contentType.contains("application/json", ignoreCase = true)) {
            val contentLength = requestHeaders["content-length"]?.toIntOrNull() ?: 0
            if (contentLength > 0) {
                val buffer = CharArray(contentLength)
                var totalRead = 0
                while (totalRead < contentLength) {
                    val read = input.read(buffer, totalRead, contentLength - totalRead)
                    if (read == -1) break
                    totalRead += read
                }

                requestBody = String(buffer, 0, totalRead)
                return requestBody!!
            }
        }

        requestBody = ""
        return requestBody!!
    }

    fun setStatus(value: Int): LocalServerContext {
        status = value
        return this
    }

    fun setText(contentType: String, value: String): LocalServerContext {
        body = value
        status = 200
        responseHeaders.put("Content-Type", contentType)
        return this
    }

    fun setHtml(value: String): LocalServerContext {
        body = value
        status = 200
        responseHeaders.put("Content-Type", "text/html")
        return this
    }

    fun setJson(value: JsonObject): LocalServerContext {
        body = value.toString()
        status = 200
        responseHeaders.put("Content-Type", "application/json")
        return this
    }

    fun send() {
        if (isSent) {
            Log.e(TAG, "Cannot send already sent response!")
            return
        }

        isSent = true
        responseHeaders.put("Connection", "close")
        responseHeaders.put("Content-Length", "${body.toByteArray().size}")

        output.println("HTTP/1.1 $status")
        responseHeaders.forEach { output.println("${it.key}: ${it.value}") }
        responseHeaders.forEach { Log.d(TAG, "${it.key}: ${it.value}") }
        output.println()
        output.print(body)
        output.flush()
        socket.close()

        Log.d(TAG, "Sent response!")
    }
}
