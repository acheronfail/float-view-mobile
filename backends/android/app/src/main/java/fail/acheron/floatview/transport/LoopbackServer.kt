package fail.acheron.floatview.transport

import android.content.res.AssetManager
import fail.acheron.floatview.application.SharedService
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URL
import org.json.JSONObject

internal const val ANDROID_BACKEND_PORT = 11012
internal const val LOOPBACK_MAX_MESSAGE_BYTES = 65_536
internal const val MAX_SESSION_IMPORT_BYTES = 128 * 1024 * 1024

class LoopbackServer(
    assets: AssetManager,
    service: SharedService,
    requestedPort: Int = 0,
    platform: String = "android",
    @Suppress("UNUSED_PARAMETER") performanceDiagnostics: Boolean = false,
) : AutoCloseable {
    private val host: RustLoopbackHost
    init {
        require(platform == "android" && requestedPort in setOf(0, ANDROID_BACKEND_PORT))
        host = RustLoopbackHost(assets.open("index.html").use { it.readBytes() }, service)
    }
    val port: Int get() = host.port
    val origin: String get() = "http://127.0.0.1:$port"
    fun start() = host.start()
    fun stop() = host.stop()
    override fun close() = host.close()

    /** Android's scoped URI stream enters the same bounded authenticated importer. */
    fun importStream(input: InputStream, checkCancelled: () -> Unit): String {
        val connection = URL("$origin/api/session-import").openConnection() as HttpURLConnection
        try {
            connection.requestMethod = "POST"
            connection.connectTimeout = 10000; connection.readTimeout = 30000
            connection.doOutput = true
            connection.setChunkedStreamingMode(65536)
            connection.setRequestProperty("Origin", origin)
            connection.setRequestProperty("X-Float-View-Token", host.token)
            connection.setRequestProperty("Content-Type", "application/octet-stream")
            connection.outputStream.use { output ->
                val bytes = ByteArray(65536)
                var total = 0L
                while (true) {
                    checkCancelled()
                    val count = input.read(bytes)
                    if (count < 0) break
                    total += count
                    check(total <= MAX_SESSION_IMPORT_BYTES)
                    output.write(bytes, 0, count)
                }
            }
            check(connection.responseCode == 200)
            val response = ByteArray(65537)
            var size = 0
            connection.inputStream.use { stream ->
                while (size < response.size) {
                    val count = stream.read(response, size, response.size - size)
                    if (count < 0) break
                    size += count
                }
            }
            check(size <= 65536)
            return JSONObject(String(response, 0, size, Charsets.UTF_8)).getString("sessionId")
        } finally { connection.disconnect() }
    }
}
