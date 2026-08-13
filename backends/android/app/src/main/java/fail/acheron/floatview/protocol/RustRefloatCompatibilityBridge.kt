package fail.acheron.floatview.protocol

import java.io.File

internal const val REFLOAT_COMPATIBILITY_ABI_VERSION = 1

internal object RustRefloatCompatibilityBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewRefloatCompatibilityJniLibrary"
    private const val LIBRARY_NAME = "float_view_refloat_compatibility_android_jni"

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

    external fun infoRequest(
        output: ByteArray?,
        result: IntArray?,
    ): Int

    external fun decodeInfo(
        input: ByteArray?,
        output: ByteArray?,
        result: IntArray?,
    ): Int

    external fun admitLayout(layout: ByteArray?): Int
}

internal fun verifyRefloatCompatibilityAbiVersion(actual: Int) {
    if (actual != REFLOAT_COMPATIBILITY_ABI_VERSION) {
        throw VescProtocolException.TruncatedPayload
    }
}
