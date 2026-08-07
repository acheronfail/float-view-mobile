package fail.acheron.floatview.core

import androidx.test.ext.junit.runners.AndroidJUnit4
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CoreBridgeInstrumentedTest {
    @Test
    fun nativeLibraryHasTheExpectedPrivateIdentity() {
        NativeCore.requireCompatibleLibrary()
        assertEquals(4, NativeCore.buildIdentity(ByteArray(15)))
    }

    @Test
    fun workerUsesCopiedBuffersAndReplacementPresentation() {
        val received = CountDownLatch(1)
        lateinit var worker: ProcessCoreBridge
        worker = ProcessCoreBridge { error("stale presentation handler was invoked") }
        worker.use {
            it.replacePresentation(1)
            it.replacePresentation(2) { command ->
                assertEquals(42, command.kind)
                assertArrayEquals(byteArrayOf(1, 2, 3), command.payload)
                assertEquals(2, command.presentationGeneration)
                worker.complete(command)
                received.countDown()
            }
            val input = byteArrayOf(1, 2, 3)
            it.submitForSmoke(42, input)
            input.fill(9)
            check(received.await(5, TimeUnit.SECONDS))
        }
    }

    @Test
    fun processWorkerReturnsCancellationAndStopWakesItsWaiter() {
        val cancelled = CountDownLatch(1)
        lateinit var worker: ProcessCoreBridge
        worker =
            ProcessCoreBridge { command ->
                assertEquals(43, command.kind)
                worker.cancel(command)
                cancelled.countDown()
            }
        worker.use {
            it.submitForSmoke(43, byteArrayOf())
            check(cancelled.await(5, TimeUnit.SECONDS))
        }
    }

    @Test
    fun jniRejectsOversizedArraysBeforeCopying() {
        NativeCore.requireCompatibleLibrary()
        val handle = NativeCore.create()
        val generation = LongArray(1)
        assertEquals(0, NativeCore.start(handle, 0, generation))
        assertEquals(
            1,
            NativeCore.submit(handle, generation[0], 42, ByteArray(64 * 1024 + 1)),
        )
        assertEquals(
            1,
            NativeCore.waitCommand(
                handle,
                generation[0],
                0,
                LongArray(7),
                ByteArray(64 * 1024 + 1),
            ),
        )
        assertEquals(0, NativeCore.stop(handle))
        assertEquals(0, NativeCore.destroy(handle))
    }
}
