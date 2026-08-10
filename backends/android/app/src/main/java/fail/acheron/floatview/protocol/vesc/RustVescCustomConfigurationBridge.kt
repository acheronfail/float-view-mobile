package fail.acheron.floatview.protocol

import java.io.File

internal const val VESC_CUSTOM_CONFIGURATION_ABI_VERSION = 1

internal object RustVescCustomConfigurationBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewVescCustomConfigurationJniLibrary"
    private const val LIBRARY_NAME = "float_view_vesc_custom_configuration_android_jni"

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

    external fun encode(
        operation: Int,
        index: Int,
        length: Long,
        offset: Long,
        values: ByteArray?,
        output: ByteArray?,
        result: IntArray?,
    ): Int

    external fun decode(
        input: ByteArray?,
        result: IntArray?,
    ): Int
}

internal fun verifyVescCustomConfigurationAbiVersion(actual: Int) {
    if (actual != VESC_CUSTOM_CONFIGURATION_ABI_VERSION) {
        throw VescProtocolException.TruncatedPayload
    }
}
