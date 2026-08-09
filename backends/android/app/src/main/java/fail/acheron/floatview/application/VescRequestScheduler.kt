package fail.acheron.floatview.application

import fail.acheron.floatview.protocol.vesc.ControllerRoute

enum class VescRequestPriority(val rank: Int) {
    AUXILIARY(0),
    CONTROLLER_WORK(1),
    RIDING_TELEMETRY(2),
    CONTROLLER_IDENTIFICATION(3),
}

sealed interface VescRequestRoute {
    data object Direct : VescRequestRoute
    data class Can(val canId: Int) : VescRequestRoute {
        init { require(canId in 0..253) }
    }
}

fun ControllerRoute.toRequestRoute(): VescRequestRoute = when (this) {
    ControllerRoute.Direct -> VescRequestRoute.Direct
    is ControllerRoute.Can -> VescRequestRoute.Can(canId)
}

data class VescExpectedResponse(
    val command: Int,
    val refloatCommand: Int? = null,
) {
    init {
        require(command in 0..255)
        require(refloatCommand == null || refloatCommand in 0..255)
    }
}

sealed interface VescRequestContext {
    val route: VescRequestRoute?

    data class ControllerFirmware(override val route: VescRequestRoute) : VescRequestContext
    data object CanPing : VescRequestContext { override val route: VescRequestRoute? = null }
    data object AuxiliaryCanPing : VescRequestContext { override val route: VescRequestRoute? = null }
    data class RefloatInfo(override val route: VescRequestRoute) : VescRequestContext
    data class RefloatLayout(override val route: VescRequestRoute) : VescRequestContext
    data class RefloatTelemetry(override val route: VescRequestRoute) : VescRequestContext
    data class RefloatControl(override val route: VescRequestRoute) : VescRequestContext
    data class ControllerShutdown(override val route: VescRequestRoute) : VescRequestContext
    data class RefloatLighting(override val route: VescRequestRoute) : VescRequestContext
    data class CurrentLimits(override val route: VescRequestRoute) : VescRequestContext
    data class AuxiliaryFirmware(override val route: VescRequestRoute) : VescRequestContext
    data class BmsValues(override val route: VescRequestRoute) : VescRequestContext
    data class RefloatConfigurationSchema(
        override val route: VescRequestRoute, val index: Int, val offset: Int, val operation: Long,
    ) : VescRequestContext
    data class RefloatConfigurationValues(
        override val route: VescRequestRoute, val index: Int, val defaults: Boolean, val operation: Long,
    ) : VescRequestContext
    data class RefloatConfigurationWrite(
        override val route: VescRequestRoute, val index: Int, val operation: Long,
    ) : VescRequestContext
    data class RefloatConfigurationVerification(
        override val route: VescRequestRoute, val index: Int, val operation: Long,
    ) : VescRequestContext
}

@JvmInline
value class VescRequestGeneration(val value: Long) {
    init { require(value >= 0) }
}

data class VescScheduledRequest(
    val sequence: Long,
    val payload: ByteArray,
    val expectedResponse: VescExpectedResponse,
    val priority: VescRequestPriority,
    val context: VescRequestContext,
    val deadlineMonotonicMs: Long,
    val generation: VescRequestGeneration,
) {
    init {
        require(sequence >= 0)
        require(payload.size <= MAX_PAYLOAD_BYTES)
        require(deadlineMonotonicMs >= 0)
    }

    companion object {
        const val MAX_QUEUED_REQUESTS = 64
        const val MAX_PAYLOAD_BYTES = 65_536
    }
}

sealed interface VescRequestCompletion {
    data class Response(val payload: ByteArray) : VescRequestCompletion
    data object Sent : VescRequestCompletion
    data object TimedOut : VescRequestCompletion
    data object QueueFull : VescRequestCompletion
    data object Cancelled : VescRequestCompletion
    data object StaleGeneration : VescRequestCompletion
}

class VescRequestScheduler(
    private val clock: ClockPort,
    private val scheduler: SchedulerPort,
    private val write: (ByteArray) -> Unit,
    private val diagnostic: (String) -> Unit = {},
) {
    private data class Pending(
        val request: VescScheduledRequest,
        val timeoutMs: Long,
        val completion: (VescScheduledRequest, VescRequestCompletion) -> Unit,
        val expectsResponse: Boolean,
    )

    private var generation = VescRequestGeneration(0)
    private var nextSequence = 0L
    private val queue = mutableListOf<Pending>()
    private var inFlight: Pending? = null
    private var deadlineTimer: CancellationPort? = null

    @Synchronized
    fun currentGeneration(): VescRequestGeneration = generation

    @Synchronized
    fun queuedCount(): Int = queue.size

    @Synchronized
    fun inFlightRequest(): VescScheduledRequest? = inFlight?.request

    fun beginGeneration(): VescRequestGeneration {
        val cancelled: List<Pending>
        val next: VescRequestGeneration
        synchronized(this) {
            generation = VescRequestGeneration(generation.value + 1)
            next = generation
            deadlineTimer?.cancel()
            deadlineTimer = null
            cancelled = queue.toList() + listOfNotNull(inFlight)
            queue.clear()
            inFlight = null
        }
        cancelled.forEach { it.completion(it.request, VescRequestCompletion.Cancelled) }
        return next
    }

    fun enqueue(
        payload: ByteArray,
        expectedResponse: VescExpectedResponse,
        priority: VescRequestPriority,
        context: VescRequestContext,
        timeoutMs: Long,
        requestGeneration: VescRequestGeneration,
        completion: (VescScheduledRequest, VescRequestCompletion) -> Unit,
    ) = enqueueInternal(
        payload, expectedResponse, priority, context, timeoutMs, requestGeneration, true, completion,
    )

    fun enqueueOneWay(
        payload: ByteArray,
        expectedResponse: VescExpectedResponse,
        priority: VescRequestPriority,
        context: VescRequestContext,
        timeoutMs: Long,
        requestGeneration: VescRequestGeneration,
        completion: (VescScheduledRequest, VescRequestCompletion) -> Unit,
    ) = enqueueInternal(
        payload, expectedResponse, priority, context, timeoutMs, requestGeneration, false, completion,
    )

    private fun enqueueInternal(
        payload: ByteArray,
        expectedResponse: VescExpectedResponse,
        priority: VescRequestPriority,
        context: VescRequestContext,
        timeoutMs: Long,
        requestGeneration: VescRequestGeneration,
        expectsResponse: Boolean,
        completion: (VescScheduledRequest, VescRequestCompletion) -> Unit,
    ) {
        require(timeoutMs > 0)
        val rejected: Pair<Pending, VescRequestCompletion>? = synchronized(this) {
            nextSequence += 1
            val request = VescScheduledRequest(
                nextSequence,
                payload.copyOf(),
                expectedResponse,
                priority,
                context,
                clock.monotonicMillis() + timeoutMs,
                requestGeneration,
            )
            val pending = Pending(request, timeoutMs, completion, expectsResponse)
            when {
                requestGeneration != generation -> pending to VescRequestCompletion.StaleGeneration
                queue.size >= VescScheduledRequest.MAX_QUEUED_REQUESTS ->
                    pending to VescRequestCompletion.QueueFull
                else -> {
                    queue += pending
                    null
                }
            }
        }
        if (rejected != null) {
            diagnostic(
                if (rejected.second == VescRequestCompletion.QueueFull) {
                    "vesc_request_queue_full"
                } else {
                    "vesc_request_stale_generation"
                },
            )
            rejected.first.completion(rejected.first.request, rejected.second)
            return
        }
        pump()
    }

    fun receive(
        payload: ByteArray,
        command: Int,
        refloatCommand: Int? = null,
        validContext: (VescRequestContext) -> Boolean = { true },
        deferNext: Boolean = false,
    ): Boolean {
        val completed = synchronized(this) {
            val current = inFlight
            if (
                current == null ||
                current.request.generation != generation ||
                current.request.expectedResponse.command != command ||
                current.request.expectedResponse.refloatCommand != refloatCommand ||
                !validContext(current.request.context)
            ) {
                null
            } else {
                deadlineTimer?.cancel()
                deadlineTimer = null
                inFlight = null
                current
            }
        }
        if (completed == null) {
            diagnostic("vesc_unmatched_or_late_response")
            return false
        }
        completed.completion(completed.request, VescRequestCompletion.Response(payload.copyOf()))
        if (!deferNext) pump()
        return true
    }

    fun resume() = pump()

    fun cancelMatching(
        predicate: (VescRequestContext) -> Boolean,
        resume: Boolean = true,
    ) {
        val cancelled = synchronized(this) {
            val values = queue.filter { predicate(it.request.context) }.toMutableList()
            queue.removeAll { predicate(it.request.context) }
            inFlight?.takeIf { predicate(it.request.context) }?.let {
                deadlineTimer?.cancel()
                deadlineTimer = null
                inFlight = null
                values += it
            }
            values
        }
        cancelled.forEach { it.completion(it.request, VescRequestCompletion.Cancelled) }
        if (resume) pump()
    }

    fun cancel(cancelledGeneration: VescRequestGeneration) {
        val cancelled = synchronized(this) {
            val values = queue.filter { it.request.generation == cancelledGeneration }.toMutableList()
            queue.removeAll { it.request.generation == cancelledGeneration }
            inFlight?.takeIf { it.request.generation == cancelledGeneration }?.let {
                deadlineTimer?.cancel()
                deadlineTimer = null
                inFlight = null
                values += it
            }
            values
        }
        cancelled.forEach { it.completion(it.request, VescRequestCompletion.Cancelled) }
        pump()
    }

    private fun pump() {
        val send = synchronized(this) {
            if (inFlight != null || queue.isEmpty()) return@synchronized null
            val next = queue.maxWithOrNull(
                compareBy<Pending> { it.request.priority.rank }
                    .thenByDescending { it.request.sequence },
            ) ?: return@synchronized null
            queue.remove(next)
            if (next.request.generation != generation) return@synchronized null
            val dispatched = next.copy(
                request = VescScheduledRequest(
                    next.request.sequence,
                    next.request.payload.copyOf(),
                    next.request.expectedResponse,
                    next.request.priority,
                    next.request.context,
                    clock.monotonicMillis() + next.timeoutMs,
                    next.request.generation,
                ),
            )
            inFlight = dispatched
            deadlineTimer = scheduler.schedule(dispatched.timeoutMs) {
                timeout(dispatched.request.sequence, dispatched.request.generation)
            }
            dispatched
        }
        if (send == null) {
            discardStaleQueuedRequests()
            return
        }
        write(send.request.payload.copyOf())
        if (!send.expectsResponse) completeOneWay(send.request.sequence)
    }

    private fun completeOneWay(sequence: Long) {
        val completed = synchronized(this) {
            val current = inFlight
            if (current?.request?.sequence != sequence || current.expectsResponse) return@synchronized null
            deadlineTimer?.cancel()
            deadlineTimer = null
            inFlight = null
            current
        } ?: return
        completed.completion(completed.request, VescRequestCompletion.Sent)
        pump()
    }

    private fun timeout(sequence: Long, timeoutGeneration: VescRequestGeneration) {
        val completed = synchronized(this) {
            val current = inFlight
            if (
                current?.request?.sequence != sequence ||
                current.request.generation != timeoutGeneration
            ) return@synchronized null
            deadlineTimer = null
            inFlight = null
            current
        } ?: return
        completed.completion(completed.request, VescRequestCompletion.TimedOut)
        pump()
    }

    private fun discardStaleQueuedRequests() {
        val stale = synchronized(this) {
            val values = queue.filter { it.request.generation != generation }
            queue.removeAll { it.request.generation != generation }
            values
        }
        stale.forEach { it.completion(it.request, VescRequestCompletion.StaleGeneration) }
        if (stale.isNotEmpty()) pump()
    }
}
