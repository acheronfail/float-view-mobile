package fail.acheron.floatview.application

import java.io.Closeable
import java.io.File
import org.json.JSONArray
import org.json.JSONObject

internal enum class RustApplicationRuntimeError {
    ABI_MISMATCH,
    INVALID_COMMAND,
    INVALID_HANDLE,
    RUNTIME_FAILURE,
    BUSY,
    INTERNAL_FAILURE,
    INVALID_STATE,
    CONFLICT,
    UNAVAILABLE,
    RESULT_TOO_LARGE,
    COPY_MISMATCH,
    MALFORMED_RESULT,
}

internal class RustApplicationRuntimeException(
    val reason: RustApplicationRuntimeError,
) : Exception(reason.name)

internal data class RustApplicationRuntimeResult(
    val generation: Long,
    val effects: List<JSONObject>,
)

internal class RustApplicationRuntime : Closeable {
    private var handle: Long

    init {
        ensureLoaded()
        if (abiVersion() != ABI_VERSION) fail(RustApplicationRuntimeError.ABI_MISMATCH)
        handle = create()
        if (handle == 0L) fail(RustApplicationRuntimeError.INTERNAL_FAILURE)
    }

    @Synchronized
    fun apply(command: JSONObject): RustApplicationRuntimeResult {
        val input = command.toString().toByteArray(Charsets.UTF_8)
        if (input.isEmpty() || input.size > MAXIMUM_COMMAND_BYTES) {
            fail(RustApplicationRuntimeError.INVALID_COMMAND)
        }
        val required = intArrayOf(0)
        val applyStatus = apply(handle, input, required)
        if (applyStatus != STATUS_OK) throw statusError(applyStatus)
        val size = required.single()
        if (size < 0 || size > MAXIMUM_RESULT_BYTES) {
            fail(RustApplicationRuntimeError.RESULT_TOO_LARGE)
        }
        val output = ByteArray(size)
        val written = intArrayOf(0)
        val copyStatus = copyResult(handle, output, written)
        if (copyStatus != STATUS_OK) throw statusError(copyStatus)
        if (written.single() != size) fail(RustApplicationRuntimeError.COPY_MISMATCH)
        val result = runCatching { JSONObject(output.toString(Charsets.UTF_8)) }
            .getOrElse { fail(RustApplicationRuntimeError.MALFORMED_RESULT) }
        if (result.optInt("version", -1) != ABI_VERSION ||
            !result.has("generation") || !result.has("effects")
        ) {
            fail(RustApplicationRuntimeError.MALFORMED_RESULT)
        }
        val effects = result.optJSONArray("effects")
            ?.objects()
            ?: fail(RustApplicationRuntimeError.MALFORMED_RESULT)
        return RustApplicationRuntimeResult(result.getLong("generation"), effects)
    }

    fun interrupt() {
        if (handle != 0L) interrupt(handle)
    }

    @Synchronized
    override fun close() {
        val value = handle
        handle = 0
        if (value != 0L) destroy(value)
    }

    private external fun abiVersion(): Int
    private external fun create(): Long
    private external fun destroy(handle: Long): Int
    private external fun interrupt(handle: Long): Int
    private external fun apply(handle: Long, command: ByteArray?, required: IntArray?): Int
    private external fun copyResult(handle: Long, output: ByteArray?, written: IntArray?): Int

    companion object {
        private const val ABI_VERSION = 1
        private const val MAXIMUM_COMMAND_BYTES = 1_048_576
        private const val MAXIMUM_RESULT_BYTES = 4_194_304
        private const val HOST_LIBRARY_PROPERTY = "floatViewApplicationRuntimeJniLibrary"
        private const val LIBRARY_NAME = "float_view_application_runtime_android_jni"

        private const val STATUS_OK = 0
        private const val STATUS_INVALID_ARGUMENT = 1
        private const val STATUS_INVALID_HANDLE = 2
        private const val STATUS_INVALID_ENVELOPE = 3
        private const val STATUS_RUNTIME_FAILURE = 4
        private const val STATUS_RESULT_PENDING = 5
        private const val STATUS_BUFFER_TOO_SMALL = 6
        private const val STATUS_BUSY = 7
        private const val STATUS_INTERNAL = 8
        private const val STATUS_INVALID_STATE = 9
        private const val STATUS_CONFLICT = 10
        private const val STATUS_UNAVAILABLE = 11

        @Volatile
        private var loaded = false

        @Synchronized
        private fun ensureLoaded() {
            if (loaded) return
            val explicitHostLibrary = System.getProperty(HOST_LIBRARY_PROPERTY)
            if (explicitHostLibrary == null) {
                System.loadLibrary(LIBRARY_NAME)
            } else {
                val library = File(explicitHostLibrary)
                require(library.isAbsolute) { "$HOST_LIBRARY_PROPERTY must be an absolute path" }
                System.load(library.canonicalPath)
            }
            loaded = true
        }

        private fun statusError(status: Int) = RustApplicationRuntimeException(
            when (status) {
                STATUS_INVALID_ARGUMENT, STATUS_INVALID_ENVELOPE,
                STATUS_RESULT_PENDING, STATUS_BUFFER_TOO_SMALL -> RustApplicationRuntimeError.INVALID_COMMAND
                STATUS_INVALID_HANDLE -> RustApplicationRuntimeError.INVALID_HANDLE
                STATUS_RUNTIME_FAILURE -> RustApplicationRuntimeError.RUNTIME_FAILURE
                STATUS_BUSY -> RustApplicationRuntimeError.BUSY
                STATUS_INVALID_STATE -> RustApplicationRuntimeError.INVALID_STATE
                STATUS_CONFLICT -> RustApplicationRuntimeError.CONFLICT
                STATUS_UNAVAILABLE -> RustApplicationRuntimeError.UNAVAILABLE
                STATUS_INTERNAL -> RustApplicationRuntimeError.INTERNAL_FAILURE
                else -> RustApplicationRuntimeError.INTERNAL_FAILURE
            },
        )

        private fun fail(reason: RustApplicationRuntimeError): Nothing =
            throw RustApplicationRuntimeException(reason)
    }
}

private fun JSONArray.objects(): List<JSONObject>? {
    val result = ArrayList<JSONObject>(length())
    for (index in 0 until length()) {
        result += optJSONObject(index) ?: return null
    }
    return result
}
