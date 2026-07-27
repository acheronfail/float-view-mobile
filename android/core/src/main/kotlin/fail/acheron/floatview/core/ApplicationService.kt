package fail.acheron.floatview.core

class ApplicationService(
    private val bluetooth: BluetoothPort,
    private val clock: ClockPort,
    private val logStore: LogStorePort,
    samplingSettings: SamplingSettings = SamplingSettings(),
    private val events: EventSink,
) {
    private val sampling = AdaptiveSamplingPolicy(samplingSettings)
    private var explicitDisconnect = false
    var activeDeviceId: DeviceId? = null
        private set
    var connectionState: ConnectionState = ConnectionState.DISCONNECTED
        private set

    fun connect(deviceId: DeviceId) {
        require(activeDeviceId == null || activeDeviceId == deviceId) {
            "Only one board can be active"
        }
        activeDeviceId = deviceId
        explicitDisconnect = false
        transition(ConnectionState.CONNECTING)
        bluetooth.connect(deviceId)
    }

    fun onBluetoothEvent(event: BluetoothEvent) {
        when (event) {
            is BluetoothEvent.ConnectionChanged -> {
                if (event.deviceId != activeDeviceId) return
                if (event.state == ConnectionState.READY && logStore.active() == null) {
                    events.publish(logStore.create(event.deviceId, clock.now()))
                }
                if (
                    event.state == ConnectionState.DISCONNECTED &&
                    !explicitDisconnect &&
                    logStore.active() != null
                ) {
                    transition(ConnectionState.RECONNECTING)
                    bluetooth.connect(event.deviceId)
                } else {
                    transition(event.state)
                    if (event.state == ConnectionState.DISCONNECTED) activeDeviceId = null
                }
            }
            else -> events.publish(event)
        }
    }

    fun disconnect() {
        val deviceId = activeDeviceId ?: return
        transition(ConnectionState.DISCONNECTING)
        explicitDisconnect = true
        logStore.active()?.let {
            events.publish(logStore.close(it.jobId, clock.now(), "explicit_disconnect"))
        }
        bluetooth.disconnect(deviceId)
    }

    fun ingest(sample: TelemetrySample) {
        val active = logStore.active() ?: return
        if (connectionState != ConnectionState.READY) return
        if (sampling.shouldPersist(sample.rideState, sample.elapsedMs)) {
            logStore.append(active.jobId, listOf(sample))
        }
        events.publish(sample)
    }

    private fun transition(state: ConnectionState) {
        connectionState = state
        events.publish(state)
    }
}
