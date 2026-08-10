package fail.acheron.floatview.protocol

import java.io.File

internal const val VESC_BMS_VALUES_ABI_VERSION = 1

internal object RustVescBmsValuesBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewVescBmsValuesJniLibrary"
    private const val LIBRARY_NAME = "float_view_vesc_bms_values_android_jni"

    init {
        val explicitHostLibrary = System.getProperty(HOST_LIBRARY_PROPERTY)
        if (explicitHostLibrary == null) {
            System.loadLibrary(LIBRARY_NAME)
        } else {
            val library = File(explicitHostLibrary)
            require(library.isAbsolute) { "$HOST_LIBRARY_PROPERTY must be an absolute path" }
            System.load(library.canonicalPath)
        }
    }

    external fun abiVersion(): Int

    external fun decode(
        input: ByteArray?,
        expectedCanId: Int,
        output: ByteArray?,
        result: IntArray?,
    ): Int
}

internal fun verifyVescBmsValuesAbiVersion(actual: Int) {
    if (actual != VESC_BMS_VALUES_ABI_VERSION) {
        throw VescProtocolException.TruncatedPayload
    }
}
