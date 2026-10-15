package fail.acheron.floatview.bluetooth

import fail.acheron.floatview.application.*
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URI
import java.util.Base64
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Authenticated test transport. No device/protocol model lives in this host. */
class FixtureBluetoothAdapter(config: JSONObject, private val emit: (BluetoothEvent) -> Unit) : BluetoothPort {
    private val endpoint = URI(config.getString("endpoint")).also {
        require(it.scheme == "http" && it.host == "127.0.0.1" && it.port in 1..65535 &&
            it.path in listOf("", "/") && it.userInfo == null && it.query == null && it.fragment == null)
    }.toURL()
    private val token = config.getString("token").also { require(it.length == 48) }
    private val run = config.getString("run").also { require(it.length == 48) }
    private val session = config.getString("session").also { require(it.length == 48) }
    private val worker = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS, ArrayBlockingQueue(63))
    private val lock = Any()
    private var failed = false
    private var pendingBytes = 0
    private var serviceGeneration = BluetoothConnection(0, 0)
    private var generation = 0
    private var operation = 0L
    private var sequence = 0L
    private var device: DeviceId? = null
    private var active: HttpURLConnection? = null
    private var activeDeadline = 0L
    private val state = BluetoothState(BluetoothRadioState.POWERED_ON,
        BluetoothPermissionState.GRANTED, BluetoothPermissionState.GRANTED)

    init {
        Thread({
            while (!synchronized(lock) { failed }) {
                Thread.sleep(250)
                if (synchronized(lock) { active != null && System.nanoTime() >= activeDeadline }) fail()
                else submit("poll")
            }
        }, "fixture-heartbeat").apply { isDaemon = true; start() }
    }
    override fun currentState() = state
    override fun startDiscovery() = submit("scan")
    override fun stopDiscovery() = submit("stopScan")
    override fun connect(deviceId: DeviceId, generation: BluetoothConnection) = submit("connect", deviceId, serviceGeneration = generation)
    override fun disconnect(deviceId: DeviceId, generation: BluetoothConnection) {
        if (synchronized(lock) { serviceGeneration == generation }) submit("disconnect", deviceId, serviceGeneration = generation)
    }
    override fun enableNotifications(generation: BluetoothConnection, operation: Long) = submit("enableNotifications", serviceGeneration = generation, transportOperation = operation)
    override fun write(deviceId: DeviceId, generation: BluetoothConnection, operation: Long, response: Boolean, bytes: ByteArray) {
        if (bytes.size > 8192) { fail(); return }
        submit("write", deviceId, bytes, generation, operation, response)
    }
    private fun submit(command: String, id: DeviceId? = null, bytes: ByteArray? = null, serviceGeneration: BluetoothConnection? = null, transportOperation: Long? = null, response: Boolean = false) {
        synchronized(lock) {
            if (failed) return
            val count = bytes?.size ?: 0
            if (pendingBytes + count > 65536) { fail(); return }
            pendingBytes += count
            val payload = bytes?.copyOf()
            if (command == "connect") generation++
            if (id != null) device = id
            if (serviceGeneration != null) this.serviceGeneration = serviceGeneration
            val capturedGeneration = this.serviceGeneration
            val expectedGeneration = generation
            val deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(4500)
            try {
                worker.execute {
                    if (synchronized(lock) { failed }) return@execute
                    try {
                        require(System.nanoTime() < deadline)
                        val op = ++operation
                        val body = JSONObject().put("run", run).put("session", session).put("operation", op)
                            .put("generation", expectedGeneration).put("command", command)
                            .put("transport_operation", transportOperation).put("response", response)
                        if (id != null) body.put("device", id.value)
                        if (payload != null) body.put("bytes", Base64.getEncoder().encodeToString(payload))
                        val connection = endpoint.openConnection() as HttpURLConnection
                        synchronized(lock) { active = connection; activeDeadline = deadline }
                        val remaining = minOf(4000, TimeUnit.NANOSECONDS.toMillis(deadline - System.nanoTime()).toInt())
                        require(remaining > 0)
                        connection.connectTimeout = remaining; connection.readTimeout = remaining
                        connection.instanceFollowRedirects = false
                        connection.requestMethod = "POST"; connection.doOutput = true
                        connection.setRequestProperty("Authorization", "Bearer $token")
                        connection.setRequestProperty("X-Fixture-Session", session)
                        val encoded = body.toString().toByteArray()
                        connection.setFixedLengthStreamingMode(encoded.size)
                        val result = try {
                            connection.outputStream.use { it.write(encoded) }
                            require(connection.responseCode == 200)
                            val data = connection.inputStream.use { input ->
                                val buffer = ByteArray(65537)
                                var size = 0
                                while (size < buffer.size) {
                                    require(System.nanoTime() < deadline)
                                    val count = input.read(buffer, size, buffer.size - size)
                                    if (count < 0) break
                                    size += count
                                }
                                buffer.copyOf(size)
                            }
                            require(data.size <= 65536 && System.nanoTime() < deadline)
                            JSONObject(String(data, Charsets.UTF_8))
                        } finally { connection.disconnect(); synchronized(lock) { active = null } }
                        require(result.getString("run") == run && result.getString("session") == session &&
                            result.getLong("operation") == op && result.getInt("generation") == expectedGeneration)
                        val events = result.getJSONArray("events")
                        require(events.length() <= 128)
                        for (i in 0 until events.length()) {
                            val event = events.getJSONObject(i)
                            val next = event.getLong("sequence"); require(next > sequence); sequence = next
                            require(event.getLong("operation") in 1..op)
                            if (event.getInt("generation") != synchronized(lock) { generation }) continue
                            val raw = event.getString("device"); require(raw.length <= 256)
                            val device = DeviceId(raw)
                            when (event.getString("event")) {
                                "discovered" -> emit(BluetoothEvent.DeviceFound(device, event.optString("name"), event.getInt("rssi"), event.getBoolean("uart")))
                                "profile" -> {
                                    val p = event.getJSONObject("profile")
                                    emit(BluetoothEvent.Profile(capturedGeneration, BluetoothProfile(p.getBoolean("service"), p.getBoolean("notify"), p.getInt("with_response"), p.getInt("without_response"))))
                                }
                                "notifications" -> emit(BluetoothEvent.Notifications(capturedGeneration, event.getLong("transport_operation"), event.getBoolean("success")))
                                "writeProgress" -> {
                                    val progress = event.getString("progress")
                                    require(progress in listOf("submitted", "acknowledged", "available"))
                                    emit(BluetoothEvent.WriteProgress(capturedGeneration, event.getLong("transport_operation"), progress))
                                }
                                "link" -> emit(BluetoothEvent.ConnectionChanged(device, capturedGeneration, when(event.getString("state")) {
                                    "ready" -> ConnectionState.READY
                                    "disconnected" -> ConnectionState.DISCONNECTED
                                    else -> error("Invalid fixture link")
                                }))
                                "bytes" -> {
                                    val encodedBytes = event.getString("bytes"); require(encodedBytes.length <= 10924)
                                    val decoded = Base64.getDecoder().decode(encodedBytes); require(decoded.size <= 8192)
                                    emit(BluetoothEvent.Notification(device, capturedGeneration, decoded))
                                }
                                else -> error("Invalid fixture event")
                            }
                        }
                    } catch (_: Exception) { fail() }
                    finally { synchronized(lock) { pendingBytes = maxOf(0, pendingBytes - count) } }
                }
            } catch (_: java.util.concurrent.RejectedExecutionException) { fail() }
        }
    }
    private fun fail() {
        val id = synchronized(lock) {
            if (failed) return
            failed = true; pendingBytes = 0; active?.disconnect(); worker.shutdownNow(); device
        }
        emit(BluetoothEvent.ScanFailed("Conformance fixture relay failed"))
        if (id != null) emit(BluetoothEvent.ConnectionChanged(id, synchronized(lock) { serviceGeneration }, ConnectionState.DISCONNECTED))
    }
}
