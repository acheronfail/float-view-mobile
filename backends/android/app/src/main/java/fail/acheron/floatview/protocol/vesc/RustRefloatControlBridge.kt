package fail.acheron.floatview.protocol

import java.io.File

internal const val REFLOAT_CONTROL_ABI_VERSION = 1

internal object RustRefloatControlBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewRefloatControlJniLibrary"
    private const val LIBRARY_NAME = "float_view_refloat_control_android_jni"

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
        lightingProfile: Int,
        fatalClearAvailable: Int,
        target: Int,
        enabled: Int,
        currentFlags: Int,
        output: ByteArray?,
        result: IntArray?,
    ): Int

    external fun decodeLighting(
        input: ByteArray?,
        lightingProfile: Int,
        result: IntArray?,
    ): Int
}

internal fun verifyRefloatControlAbiVersion(actual: Int) {
    if (actual != REFLOAT_CONTROL_ABI_VERSION) throw VescProtocolException.InvalidFrame
}
