package fail.acheron.floatview.protocol

import java.io.File

internal object RustVescFrameBridge {
    private const val HOST_LIBRARY_PROPERTY = "floatViewVescFrameJniLibrary"
    private const val LIBRARY_NAME = "float_view_vesc_frame_android_jni"

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
        input: ByteArray?,
        output: ByteArray?,
        outputLength: IntArray?,
    ): Int

    external fun decodeExact(
        input: ByteArray?,
        output: ByteArray?,
        outputLength: IntArray?,
    ): Int
}

internal fun verifyVescFrameAbiVersion(actual: Int) {
    if (actual != 1) throw VescProtocolException.InvalidFrame
}
