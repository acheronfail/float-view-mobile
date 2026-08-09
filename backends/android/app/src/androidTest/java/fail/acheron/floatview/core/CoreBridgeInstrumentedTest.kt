package fail.acheron.floatview.core

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.io.File
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CoreBridgeInstrumentedTest {
    @Test
    fun nativeLibraryHasTheExpectedPrivateIdentity() {
        NativeCore.requireCompatibleLibrary()
        assertEquals(STATUS_BUFFER_TOO_SMALL, NativeCore.buildIdentity(ByteArray(15)))
    }

    @Test
    fun compatibilityCheckRejectsAbiAndBuildMismatches() {
        expectIllegalState {
            NativeCore.requireCompatibleIdentity(5, STATUS_OK, EXPECTED_BUILD_ID.copyOf())
        }
        expectIllegalState {
            NativeCore.requireCompatibleIdentity(4, STATUS_INVALID_ARGUMENT, EXPECTED_BUILD_ID.copyOf())
        }
        expectIllegalState {
            NativeCore.requireCompatibleIdentity(4, STATUS_OK, ByteArray(EXPECTED_BUILD_ID.size))
        }
    }

    @Test
    fun jniCopiesBoundedDocumentCommandAndGrowsWaitBufferWithoutDequeuing() {
        NativeCore.requireCompatibleLibrary()
        val handle = NativeCore.create()
        val generation = LongArray(1)
        assertTrue(handle > 0)
        assertEquals(STATUS_OK, NativeCore.start(handle, 0, generation))
        try {
            val submitted = importPayload(
                "/tmp/float-view/import.floatview.tmp",
                listOf("application/octet-stream", "application/zip"),
                4L * 1024 * 1024,
            )
            val expected = submitted.copyOf()
            assertEquals(STATUS_OK, NativeCore.submit(handle, generation[0], COMMAND_PRESENT_IMPORT, submitted))
            submitted.fill(0x5a)

            val header = LongArray(7)
            val undersized = ByteArray(1) { 0x33 }
            assertEquals(
                STATUS_BUFFER_TOO_SMALL,
                NativeCore.waitCommand(handle, generation[0], 0, header, undersized),
            )
            assertEquals(expected.size.toLong(), header[6])
            assertArrayEquals(byteArrayOf(0x33), undersized)

            val output = ByteArray(header[6].toInt())
            assertEquals(STATUS_OK, NativeCore.waitCommand(handle, generation[0], 0, header, output))
            assertEquals(COMMAND_PRESENT_IMPORT.toLong(), header[1])
            assertEquals(expected.size.toLong(), header[5])
            assertArrayEquals(expected, output)

            val decoded = PlatformCommandEnvelope.decode(
                NativeHostCommand(
                    kind = header[1].toInt(),
                    runtimeGeneration = header[3],
                    operationId = header[4],
                    payload = output,
                    presentationGeneration = 0,
                ),
            )
            val document = AndroidHostCommand.decode(decoded.command as PlatformCommand.Host)
                as AndroidHostCommand.PresentImport
            assertEquals("/tmp/float-view/import.floatview.tmp", document.stagingPath)
            assertEquals(listOf("application/octet-stream", "application/zip"), document.allowedContentTypes)
            assertEquals(4L * 1024 * 1024, document.maximumBytes)
            assertEquals(STATUS_OK, NativeCore.complete(handle, generation[0], header[4], 0, byteArrayOf()))
        } finally {
            assertEquals(STATUS_OK, NativeCore.stop(handle))
            assertEquals(STATUS_OK, NativeCore.destroy(handle))
        }
    }

    @Test
    fun jniRejectsInvalidPathsAssociationsNamesAndAnyHostUuidInput() {
        NativeCore.requireCompatibleLibrary()
        assertFalse(
            "Firmware UUID must be parsed in Rust rather than accepted through JNI",
            NativeCore::class.java.declaredMethods.any { it.name.contains("uuid", ignoreCase = true) },
        )

        val handle = NativeCore.create()
        val generation = LongArray(1)
        assertTrue(handle > 0)
        assertEquals(
            STATUS_INVALID_ARGUMENT,
            NativeCore.startConfigured(
                handle,
                0,
                0,
                "relative/core".encodeToByteArray(),
                "<html></html>".encodeToByteArray(),
                generation,
            ),
        )
        assertEquals(STATUS_OK, NativeCore.start(handle, 0, generation))
        try {
            assertEquals(
                STATUS_INVALID_ARGUMENT,
                NativeCore.submitBluetoothDevice(
                    handle, generation[0], 0, 1, byteArrayOf(), byteArrayOf(), -40,
                ),
            )
            assertEquals(
                STATUS_INVALID_ARGUMENT,
                NativeCore.submitBluetoothDevice(
                    handle, generation[0], 0, 1, ByteArray(257), byteArrayOf(), -40,
                ),
            )
            assertEquals(
                STATUS_INVALID_ARGUMENT,
                NativeCore.submitBluetoothDevice(
                    handle, generation[0], 0, 1, byteArrayOf(1), byteArrayOf(0xff.toByte()), -40,
                ),
            )
            assertEquals(
                STATUS_INVALID_ARGUMENT,
                NativeCore.submitBluetoothDevice(
                    handle, generation[0], 0, 1, byteArrayOf(1), ByteArray(129), -40,
                ),
            )
        } finally {
            assertEquals(STATUS_OK, NativeCore.stop(handle))
            assertEquals(STATUS_OK, NativeCore.destroy(handle))
        }
    }

    @Test
    fun jniRejectsDuplicateAndStaleCompletions() {
        NativeCore.requireCompatibleLibrary()
        val handle = NativeCore.create()
        val generation = LongArray(1)
        assertTrue(handle > 0)
        assertEquals(STATUS_OK, NativeCore.start(handle, 0, generation))
        try {
            assertEquals(STATUS_OK, NativeCore.submit(handle, generation[0], 42, byteArrayOf(1)))
            val header = LongArray(7)
            assertEquals(
                STATUS_OK,
                NativeCore.waitCommand(handle, generation[0], 0, header, ByteArray(1)),
            )
            val operation = header[4]
            assertNotEquals(0L, operation)
            assertEquals(STATUS_OK, NativeCore.complete(handle, generation[0], operation, 0, byteArrayOf()))
            assertEquals(
                STATUS_DUPLICATE_OPERATION,
                NativeCore.complete(handle, generation[0], operation, 0, byteArrayOf()),
            )
            assertEquals(
                STATUS_STALE_GENERATION,
                NativeCore.complete(handle, generation[0], operation + 1, 0, byteArrayOf()),
            )
            assertEquals(
                STATUS_STALE_GENERATION,
                NativeCore.complete(handle, generation[0] + 1, operation, 0, byteArrayOf()),
            )
        } finally {
            assertEquals(STATUS_OK, NativeCore.stop(handle))
            assertEquals(STATUS_OK, NativeCore.destroy(handle))
        }
    }

    @Test
    fun stopWakesBlockedJniWaiter() {
        NativeCore.requireCompatibleLibrary()
        val handle = NativeCore.create()
        val generation = LongArray(1)
        assertTrue(handle > 0)
        assertEquals(STATUS_OK, NativeCore.start(handle, 0, generation))

        val entered = CountDownLatch(1)
        val result = AtomicInteger(Int.MIN_VALUE)
        val waiter = Thread {
            entered.countDown()
            result.set(
                NativeCore.waitCommand(
                    handle,
                    generation[0],
                    30_000,
                    LongArray(7),
                    ByteArray(0),
                ),
            )
        }
        waiter.start()
        assertTrue(entered.await(5, TimeUnit.SECONDS))
        assertEquals(STATUS_OK, NativeCore.stop(handle))
        waiter.join(5_000)
        assertFalse("JNI waiter did not wake after stop", waiter.isAlive)
        assertTrue(
            "JNI waiter returned an unexpected shutdown status: ${result.get()}",
            result.get() == STATUS_STOPPED || result.get() == STATUS_WRONG_STATE,
        )
        assertEquals(STATUS_OK, NativeCore.destroy(handle))
    }

    @Test
    fun runtimeCanRestartWithANewHandleAndRejectsTheOldGeneration() {
        NativeCore.requireCompatibleLibrary()
        val firstHandle = NativeCore.create()
        val firstGeneration = LongArray(1)
        assertEquals(STATUS_OK, NativeCore.start(firstHandle, 0, firstGeneration))
        assertEquals(STATUS_OK, NativeCore.stop(firstHandle))
        assertEquals(STATUS_OK, NativeCore.destroy(firstHandle))

        val secondHandle = NativeCore.create()
        val secondGeneration = LongArray(1)
        assertEquals(STATUS_OK, NativeCore.start(secondHandle, 0, secondGeneration))
        try {
            assertNotEquals(firstHandle, secondHandle)
            assertNotEquals(firstGeneration[0], secondGeneration[0])
            assertEquals(
                STATUS_STALE_GENERATION,
                NativeCore.submit(secondHandle, firstGeneration[0], 42, byteArrayOf()),
            )
            assertEquals(STATUS_OK, NativeCore.submit(secondHandle, secondGeneration[0], 42, byteArrayOf()))
        } finally {
            assertEquals(STATUS_OK, NativeCore.stop(secondHandle))
            assertEquals(STATUS_OK, NativeCore.destroy(secondHandle))
        }
    }

    @Test
    fun workerUsesCopiedBuffersAndReplacementPresentation() {
        val received = CountDownLatch(1)
        lateinit var worker: ProcessCoreBridge
        worker = processBridge { error("stale presentation handler was invoked") }
        worker.use {
            it.replacePresentation(1)
            it.replacePresentation(2) { command ->
                if (command.kind == 42) {
                    assertArrayEquals(byteArrayOf(1, 2, 3), command.payload)
                    assertEquals(2, command.presentationGeneration)
                    worker.complete(command)
                    received.countDown()
                } else {
                    worker.complete(command)
                }
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
            processBridge { command ->
                if (command.kind == 43) {
                    worker.cancel(command)
                    cancelled.countDown()
                } else {
                    worker.complete(command)
                }
            }
        worker.use {
            it.submitForSmoke(43, byteArrayOf())
            check(cancelled.await(5, TimeUnit.SECONDS))
        }
    }

    @Test
    fun completionArrivingAfterBridgeCloseIsIgnored() {
        val received = CountDownLatch(1)
        lateinit var lateCommand: NativeHostCommand
        lateinit var worker: ProcessCoreBridge
        worker =
            processBridge { command ->
                if (command.kind == 44) {
                    lateCommand = command
                    received.countDown()
                } else {
                    worker.complete(command)
                }
            }
        worker.submitForSmoke(44, byteArrayOf())
        check(received.await(5, TimeUnit.SECONDS))
        worker.close()
        worker.complete(lateCommand)
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

    private fun importPayload(path: String, contentTypes: List<String>, maximumBytes: Long): ByteArray {
        val writer = BridgeWriter()
        writer.bytes(path.encodeToByteArray(), 4096)
        writer.u8(contentTypes.size)
        contentTypes.forEach { writer.bytes(it.encodeToByteArray(), 128) }
        writer.i64(maximumBytes)
        return writer.finish()
    }

    private fun expectIllegalState(block: () -> Unit) {
        try {
            block()
        } catch (_: IllegalStateException) {
            return
        }
        throw AssertionError("Expected compatibility validation to fail")
    }

    private fun processBridge(onCommand: (NativeHostCommand) -> Unit): ProcessCoreBridge {
        val context = ApplicationProvider.getApplicationContext<Context>()
        return ProcessCoreBridge(
            softwareTransport = true,
            storageRoot = File(context.cacheDir, "core-bridge-${System.nanoTime()}"),
            frontendIndex = "<html></html>".encodeToByteArray(),
            onCommand = onCommand,
        )
    }

    private companion object {
        const val STATUS_OK = 0
        const val STATUS_INVALID_ARGUMENT = 1
        const val STATUS_WRONG_STATE = 3
        const val STATUS_BUFFER_TOO_SMALL = 4
        const val STATUS_STOPPED = 6
        const val STATUS_STALE_GENERATION = 8
        const val STATUS_DUPLICATE_OPERATION = 9
        const val COMMAND_PRESENT_IMPORT = 12
        val EXPECTED_BUILD_ID = "float-view-abi04".encodeToByteArray()
    }
}
