package fail.acheron.floatview.application

import java.io.File

/** The sole hand-written JNI family; all process and listener state lives in Rust. */
class RustApplicationRuntime private constructor() {
    companion object {
        @JvmStatic external fun serviceCreate(root: ByteArray?): Long
        @JvmStatic external fun serviceDestroy(handle: Long): Int
        @JvmStatic external fun serviceApply(handle: Long, command: ByteArray?, required: IntArray?): Int
        @JvmStatic external fun serviceCopyResult(handle: Long, output: ByteArray?, written: IntArray?): Int
        @JvmStatic external fun serviceServerCreate(service: Long, platform: Int, html: ByteArray): Long
        @JvmStatic external fun serverDestroy(handle: Long): Int
        @JvmStatic external fun serverCommand(handle: Long, input: ByteArray, output: ByteArray, written: IntArray): Int
        @Volatile private var loaded = false
        @Synchronized fun ensureLoaded() {
            if (loaded) return
            val explicit = System.getProperty("floatViewApplicationRuntimeJniLibrary")
            if (explicit == null) System.loadLibrary("float_view_application_runtime_android_jni")
            else {
                val library = File(explicit)
                require(library.isAbsolute)
                System.load(library.canonicalPath)
            }
            loaded = true
        }
    }
}
