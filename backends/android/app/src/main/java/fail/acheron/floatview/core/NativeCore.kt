package fail.acheron.floatview.core

internal object NativeCore {
    init {
        System.loadLibrary("host_android_jni")
    }

    external fun abiVersion(): Int
    external fun buildIdentity(output: ByteArray): Int
    external fun create(): Long
    external fun start(handle: Long, requestedPort: Int, generation: LongArray): Int
    external fun startConfigured(
        handle: Long,
        requestedPort: Int,
        transport: Int,
        storageRoot: ByteArray,
        frontendIndex: ByteArray,
        generation: LongArray,
    ): Int
    external fun submit(handle: Long, generation: Long, kind: Int, payload: ByteArray): Int
    external fun submitBle(
        handle: Long,
        generation: Long,
        connectionGeneration: Long,
        adapterOrder: Long,
        payload: ByteArray,
    ): Int
    external fun submitBluetoothState(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long,
        radio: Int, authorization: Int,
    ): Int
    external fun submitBluetoothDevice(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long,
        association: ByteArray, name: ByteArray, rssi: Int,
    ): Int
    external fun submitBluetoothConnection(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long, state: Int,
    ): Int
    external fun submitLocationAuthorization(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long,
        authorization: Int,
    ): Int
    external fun submitLocation(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long,
        latitude: Double, longitude: Double, accuracyM: Double, hasAccuracy: Boolean,
    ): Int
    external fun submitLifecycle(
        handle: Long, generation: Long, connectionGeneration: Long, adapterOrder: Long,
        foreground: Boolean, backgroundEligible: Boolean,
    ): Int
    external fun waitCommand(
        handle: Long,
        generation: Long,
        timeoutMs: Int,
        header: LongArray,
        payload: ByteArray,
    ): Int
    external fun complete(
        handle: Long,
        generation: Long,
        operationId: Long,
        resultKind: Int,
        payload: ByteArray,
    ): Int
    external fun stop(handle: Long): Int
    external fun destroy(handle: Long): Int

    fun requireCompatibleLibrary() {
        val identity = ByteArray(EXPECTED_BUILD_ID.size)
        requireCompatibleIdentity(abiVersion(), buildIdentity(identity), identity)
    }

    internal fun requireCompatibleIdentity(
        actualAbiVersion: Int,
        buildIdentityStatus: Int,
        actualBuildIdentity: ByteArray,
    ) {
        check(actualAbiVersion == EXPECTED_ABI_VERSION) {
            "Rust core ABI mismatch at bounded stage: expected=$EXPECTED_ABI_VERSION actual=$actualAbiVersion"
        }
        check(
            buildIdentityStatus == STATUS_OK &&
                actualBuildIdentity.contentEquals(EXPECTED_BUILD_ID),
        ) {
            "Rust core build identity mismatch at bounded stage"
        }
    }

    private const val EXPECTED_ABI_VERSION = 4
    private const val STATUS_OK = 0
    private val EXPECTED_BUILD_ID = "float-view-abi04".encodeToByteArray()
}
