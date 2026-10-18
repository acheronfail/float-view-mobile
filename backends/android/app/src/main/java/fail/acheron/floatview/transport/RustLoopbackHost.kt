package fail.acheron.floatview.transport

import fail.acheron.floatview.application.SharedService
import fail.acheron.floatview.application.RustApplicationRuntime
import fail.acheron.floatview.application.json
import org.json.JSONObject

/** A listener borrows the existing process owner; it owns no domain dispatcher. */
internal class RustLoopbackHost(html: ByteArray, private val service: SharedService) : AutoCloseable {
    private val handle = RustApplicationRuntime.serviceServerCreate(service.handle, 2, html).also { check(it > 0) }
    private var running = false
    var port = 0; private set
    var token = ""; private set
    private fun command(name: String, parameters: JSONObject? = null): JSONObject {
        val action = json("command" to name)
        if (parameters != null) action.put("parameters", parameters)
        val input = json("version" to 1, "action" to action).toString().toByteArray(Charsets.UTF_8)
        val output = ByteArray(1048576)
        val written = IntArray(1)
        check(RustApplicationRuntime.serverCommand(handle, input, output, written) == 0)
        return JSONObject(String(output, 0, written[0], Charsets.UTF_8)).getJSONObject("data")
    }
    @Synchronized fun start() {
        if (running) return
        val result = command("start", json("platform" to "android", "port" to 11012))
        port = result.getInt("port"); token = result.getString("token"); running = true
    }
    @Synchronized fun stop() { if (running) { command("stop"); running = false } }
    override fun close() {
        try { stop() } finally { check(RustApplicationRuntime.serverDestroy(handle) == 0) }
    }
}
