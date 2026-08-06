package fail.acheron.floatview.backend

import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VescRequestSchedulerTest {
    @Test
    fun prioritizesControllerAndRidingWork() {
        val harness = Harness()
        val generation = harness.subject.beginGeneration()
        harness.enqueue(1, VescRequestPriority.CONTROLLER_WORK, VescRequestContext.CurrentLimits(VescRequestRoute.Direct), generation)
        harness.enqueue(2, VescRequestPriority.AUXILIARY, VescRequestContext.BmsValues(VescRequestRoute.Can(42)), generation)
        harness.enqueue(3, VescRequestPriority.RIDING_TELEMETRY, VescRequestContext.RefloatTelemetry(VescRequestRoute.Direct), generation)
        harness.enqueue(4, VescRequestPriority.CONTROLLER_IDENTIFICATION, VescRequestContext.ControllerFirmware(VescRequestRoute.Can(7)), generation)

        assertEquals(listOf(1), harness.writes)
        assertTrue(harness.subject.receive(byteArrayOf(1), 1))
        assertEquals(listOf(1, 4), harness.writes)
        assertTrue(harness.subject.receive(byteArrayOf(4), 1))
        assertEquals(listOf(1, 4, 3), harness.writes)
        assertTrue(harness.subject.receive(byteArrayOf(3), 1))
        assertEquals(listOf(1, 4, 3, 2), harness.writes)
    }

    @Test
    fun attributesRoutesAndAdvancesAfterTimeout() {
        val harness = Harness()
        val generation = harness.subject.beginGeneration()
        harness.enqueue(7, context = VescRequestContext.ControllerFirmware(VescRequestRoute.Can(7)), generation = generation)
        harness.enqueue(42, context = VescRequestContext.AuxiliaryFirmware(VescRequestRoute.Can(42)), generation = generation)
        harness.timers.fireFirstActive()
        assertEquals(listOf(7, 42), harness.writes)
        assertEquals(VescRequestCompletion.TimedOut, harness.completions.first().second)
        assertEquals(VescRequestContext.AuxiliaryFirmware(VescRequestRoute.Can(42)), harness.subject.inFlightRequest()?.context)
        assertTrue(harness.subject.receive(byteArrayOf(42), 1))
    }

    @Test
    fun boundsQueueCancelsReconnectAndRejectsLateResponses() {
        val harness = Harness()
        val generation = harness.subject.beginGeneration()
        harness.enqueue(0, generation = generation)
        (1..VescScheduledRequest.MAX_QUEUED_REQUESTS).forEach { harness.enqueue(it, generation = generation) }
        harness.enqueue(255, generation = generation)
        assertEquals(VescRequestCompletion.QueueFull, harness.completions.last().second)

        val newer = harness.subject.beginGeneration()
        assertEquals(65, harness.completions.count { it.second == VescRequestCompletion.Cancelled })
        assertFalse(harness.subject.receive(byteArrayOf(0), 1))
        harness.enqueue(9, generation = generation)
        assertEquals(VescRequestCompletion.StaleGeneration, harness.completions.last().second)
        harness.enqueue(10, generation = newer)
        assertEquals(10, harness.writes.last())
    }

    @Test
    fun startsResponseDeadlineWhenWorkLeavesTheQueue() {
        val harness = Harness()
        val generation = harness.subject.beginGeneration()
        harness.enqueue(
            62,
            context = VescRequestContext.AuxiliaryCanPing,
            timeoutMs = 10_000,
            generation = generation,
        )
        harness.enqueue(
            31,
            priority = VescRequestPriority.CONTROLLER_WORK,
            context = VescRequestContext.RefloatTelemetry(VescRequestRoute.Direct),
            timeoutMs = 900,
            generation = generation,
        )

        harness.clock.milliseconds = 2_550
        assertTrue(harness.subject.receive(byteArrayOf(62), 1))

        assertEquals(listOf(62, 31), harness.writes)
        assertEquals(3_450L, harness.subject.inFlightRequest()?.deadlineMonotonicMs)
        assertTrue(harness.completions[0].second is VescRequestCompletion.Response)
        assertEquals(1, harness.completions.size)
    }

    private class Harness {
        val clock = TestClock()
        val timers = TestTimers()
        val writes = mutableListOf<Int>()
        val completions = mutableListOf<Pair<VescScheduledRequest, VescRequestCompletion>>()
        val subject = VescRequestScheduler(clock, timers, { writes += it.first().toInt() and 0xff })

        fun enqueue(
            byte: Int,
            priority: VescRequestPriority = VescRequestPriority.AUXILIARY,
            context: VescRequestContext = VescRequestContext.BmsValues(VescRequestRoute.Direct),
            generation: VescRequestGeneration,
            timeoutMs: Long = 100,
        ) = subject.enqueue(
            byteArrayOf(byte.toByte()), VescExpectedResponse(1), priority, context, timeoutMs, generation,
        ) { request, outcome -> completions += request to outcome }
    }

    private class TestClock : ClockPort {
        var milliseconds = 0L
        override fun monotonicMillis() = milliseconds
        override fun now() = Instant.EPOCH
    }

    private class TestTimers : SchedulerPort {
        private data class Item(var active: Boolean = true, val action: () -> Unit)
        private val items = mutableListOf<Item>()
        override fun schedule(delayMs: Long, action: () -> Unit): CancellationPort {
            val item = Item(action = action)
            items += item
            return CancellationPort { item.active = false }
        }
        override fun scheduleRepeating(intervalMs: Long, action: () -> Unit) = schedule(intervalMs, action)
        fun fireFirstActive() = items.firstOrNull { it.active }?.also { it.active = false }?.action?.invoke()
    }
}
