package fail.acheron.floatview.core

internal object NativeCore {
    init {
        System.loadLibrary("host_android_jni")
    }

    external fun abiVersion(): Int
    external fun buildIdentity(output: ByteArray): Int
    external fun create(): Long
    external fun start(handle: Long, requestedPort: Int, generation: LongArray): Int
    external fun submit(handle: Long, generation: Long, kind: Int, payload: ByteArray): Int
    external fun submitBle(
        handle: Long,
        generation: Long,
        connectionGeneration: Long,
        adapterOrder: Long,
        payload: ByteArray,
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
        check(abiVersion() == EXPECTED_ABI_VERSION) {
            "Rust core ABI mismatch at bounded stage: expected=$EXPECTED_ABI_VERSION actual=${abiVersion()}"
        }
        val identity = ByteArray(EXPECTED_BUILD_ID.size)
        check(buildIdentity(identity) == STATUS_OK && identity.contentEquals(EXPECTED_BUILD_ID)) {
            "Rust core build identity mismatch at bounded stage"
        }
    }

    private const val EXPECTED_ABI_VERSION = 2
    private const val STATUS_OK = 0
    private val EXPECTED_BUILD_ID = "float-view-0.1.0".encodeToByteArray()
}
