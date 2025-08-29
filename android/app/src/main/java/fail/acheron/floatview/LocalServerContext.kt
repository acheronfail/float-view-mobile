package fail.acheron.floatview

import android.util.Log
import java.io.PrintWriter
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap

class LocalServerContext(private val socket: Socket, private val output: PrintWriter) {
    private val TAG = "FloatView_LocalServerContext"

    var status = 404
    private var isSent = false
    private val headers = ConcurrentHashMap<String, String>()
    private var body = ""

    fun isSent(): Boolean {
        return isSent
    }

    fun setStatus(value: Int): LocalServerContext {
        status = value
        return this
    }

    fun setHtml(value: String): LocalServerContext {
        body = value
        status = 200
        headers.put("Content-Type", "text/html")
        return this
    }

    fun setJson(value: String): LocalServerContext {
        body = value
        status = 200
        headers.put("Content-Type", "application/json")
        return this
    }

    fun send() {
        if (isSent) {
            Log.e(TAG, "Cannot send already sent response!")
            return
        }

        isSent = true
        headers.put("Connection", "close")
        headers.put("Content-Length", "${body.toByteArray().size}")

        output.println("HTTP/1.1 $status")
        headers.forEach { output.println("${it.key}: ${it.value}") }
        output.println()
        output.print(body)
        output.flush()
        socket.close()

        Log.d(TAG, "Sent response!")
      }
}