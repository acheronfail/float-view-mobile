package fail.acheron.floatview.core

import java.io.File
import java.io.Closeable
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import java.util.concurrent.atomic.AtomicReference

internal data class NativeHostCommand(
    val kind: Int,
    val runtimeGeneration: Long,
    val operationId: Long,
    val payload: ByteArray,
    val presentationGeneration: Long,
)

internal interface CorePlatformRuntime {
    val runtimeGeneration: Long
    fun submitPlatformEvent(connectionGeneration: Long, adapterOrder: Long, event: PlatformEvent)
    fun submitBluetoothBytes(connectionGeneration: Long, adapterOrder: Long, payload: ByteArray)
    fun complete(command: NativeHostCommand, resultKind: Int = 0, payload: ByteArray = byteArrayOf())
}

/**
 * Process-lifetime owner for the Rust handle and its only blocking JNI waiter.
 * Activities replace presentation generations; they never create this owner.
 */
internal class ProcessCoreBridge(
    requestedPort: Int = 0,
    softwareTransport: Boolean,
    storageRoot: File,
    frontendIndex: ByteArray,
    onCommand: (NativeHostCommand) -> Unit,
) :
    CorePlatformRuntime,
    Closeable {
    private val presentationGeneration = AtomicLong(0)
    private val commandHandler = AtomicReference(onCommand)
    private val closed = AtomicBoolean(false)
    private val handle: Long
    private val generation = LongArray(1)
    private val worker: Thread

    init {
        NativeCore.requireCompatibleLibrary()
        handle = NativeCore.create().also { check(it > 0) }
        check(
            NativeCore.startConfigured(
                handle,
                requestedPort,
                if (softwareTransport) 1 else 0,
                storageRoot.canonicalPath.encodeToByteArray(),
                frontendIndex.copyOf(),
                generation,
            ) == STATUS_OK,
        )
        worker = Thread(::pump, "float-view-core-command-pump").also { it.start() }
    }

    fun replacePresentation(generation: Long, onCommand: (NativeHostCommand) -> Unit) {
        check(!closed.get())
        commandHandler.set(onCommand)
        presentationGeneration.set(generation)
    }

    fun replacePresentation(generation: Long) {
        check(!closed.get())
        presentationGeneration.set(generation)
    }

    fun submitForSmoke(kind: Int, payload: ByteArray) {
        check(NativeCore.submit(handle, generation[0], kind, payload.copyOf()) == STATUS_OK)
    }

    override val runtimeGeneration: Long
        get() = generation[0]

    override fun submitPlatformEvent(
        connectionGeneration: Long,
        adapterOrder: Long,
        event: PlatformEvent,
    ) {
        val status = when (event) {
            is PlatformEvent.BluetoothState -> NativeCore.submitBluetoothState(
                handle, generation[0], connectionGeneration, adapterOrder,
                event.radio.tag, event.authorization.tag,
            )
            is PlatformEvent.BluetoothDevice -> NativeCore.submitBluetoothDevice(
                handle, generation[0], connectionGeneration, adapterOrder,
                event.token.copyOf(), event.name?.encodeToByteArray() ?: byteArrayOf(), event.rssi.toInt(),
            )
            is PlatformEvent.BluetoothConnectionChanged -> NativeCore.submitBluetoothConnection(
                handle, generation[0], event.connectionGeneration, adapterOrder, event.state.tag,
            )
            is PlatformEvent.LocationAuthorizationChanged -> NativeCore.submitLocationAuthorization(
                handle, generation[0], connectionGeneration, adapterOrder, event.authorization.tag,
            )
            is PlatformEvent.Location -> NativeCore.submitLocation(
                handle, generation[0], connectionGeneration, adapterOrder,
                event.latitude, event.longitude, event.accuracyM ?: 0.0, event.accuracyM != null,
            )
            is PlatformEvent.Lifecycle -> NativeCore.submitLifecycle(
                handle, generation[0], connectionGeneration, adapterOrder,
                event.foreground, event.backgroundEligible,
            )
            is PlatformEvent.LoggingService -> NativeCore.submitLoggingService(
                handle, generation[0], connectionGeneration, adapterOrder, event.active,
            )
            is PlatformEvent.Theme -> NativeCore.submitTheme(
                handle, generation[0], connectionGeneration, adapterOrder, event.theme.tag,
            )
        }
        check(status == STATUS_OK)
    }

    override fun submitBluetoothBytes(
        connectionGeneration: Long,
        adapterOrder: Long,
        payload: ByteArray,
    ) {
        check(
            NativeCore.submitBle(
                handle,
                generation[0],
                connectionGeneration,
                adapterOrder,
                payload.copyOf(),
            ) == STATUS_OK,
        )
    }

    override fun complete(
        command: NativeHostCommand,
        resultKind: Int,
        payload: ByteArray,
    ) {
        check(!closed.get())
        check(
            NativeCore.complete(
                handle,
                generation[0],
                command.operationId,
                resultKind,
                payload.copyOf(),
            ) ==
                STATUS_OK,
        )
    }

    fun cancel(command: NativeHostCommand) = complete(command, RESULT_CANCELLED)

    private fun pump() {
        check(Thread.currentThread() !== android.os.Looper.getMainLooper().thread)
        val header = LongArray(7)
        var payload = ByteArray(1024)
        while (!closed.get()) {
            when (val status = NativeCore.waitCommand(handle, generation[0], 100, header, payload)) {
                STATUS_OK -> {
                    check(header[0] == EXPECTED_ABI_VERSION.toLong())
                    check(header[3] == generation[0])
                    check(header[5] in 0..MAX_COMMAND_PAYLOAD.toLong())
                    val command =
                        NativeHostCommand(
                            kind = header[1].toInt(),
                            runtimeGeneration = header[3],
                            operationId = header[4],
                            payload = payload.copyOf(header[5].toInt()),
                            presentationGeneration = presentationGeneration.get(),
                        )
                    commandHandler.get().invoke(command)
                }
                STATUS_BUFFER_TOO_SMALL -> {
                    check(header[6] in 0..MAX_COMMAND_PAYLOAD.toLong())
                    payload = ByteArray(header[6].toInt())
                }
                STATUS_TIMEOUT -> Unit
                STATUS_STOPPED -> return
                else -> error("Native command wait failed at bounded stage: $status")
            }
        }
    }

    override fun close() {
        if (!closed.compareAndSet(false, true)) return
        check(NativeCore.stop(handle) == STATUS_OK)
        worker.join()
        check(NativeCore.destroy(handle) == STATUS_OK)
    }

    private companion object {
        const val STATUS_OK = 0
        const val STATUS_BUFFER_TOO_SMALL = 4
        const val STATUS_TIMEOUT = 5
        const val STATUS_STOPPED = 6
        const val RESULT_SUCCESS = 0
        const val RESULT_CANCELLED = 1
        const val EXPECTED_ABI_VERSION = 3
        const val MAX_COMMAND_PAYLOAD = 64 * 1024
    }
}
