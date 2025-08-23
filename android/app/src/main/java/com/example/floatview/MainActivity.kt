package com.example.floatview

import android.os.Bundle
import android.util.Log
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import java.io.*
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {

    private var serverSocket: ServerSocket? = null
    private val executor = Executors.newCachedThreadPool()
    private lateinit var webView: WebView
    private val TAG = "FloatViewApp"

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

            // Skip remaining headers
            var line: String?
            do {
                line = input.readLine()
            } while (line != null && line.isNotEmpty())

            // Handle different paths
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
            try {
                socket.close()
            } catch (e: IOException) {
                Log.e(TAG, "Error closing socket: ${e.message}", e)
            }
        }
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
            serverSocket?.close()
            executor.shutdown()
        } catch (e: IOException) {
            e.printStackTrace()
        }
    }
}
