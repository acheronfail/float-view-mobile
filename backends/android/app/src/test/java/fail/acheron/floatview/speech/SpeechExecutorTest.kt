package fail.acheron.floatview.speech

import org.junit.Assert.*
import org.junit.Test

class SpeechExecutorTest {
    private class Engine : SpeechEngine {
        lateinit var initialized: (Boolean) -> Unit
        var reason: String? = null
        var accepts = true
        var throwsOnSpeak = false
        var throwsOnStop = false
        var stopped = 0
        var shutdowns = 0
        val callbacks = mutableMapOf<Long, (Long, String) -> Unit>()
        val requests = mutableListOf<SpeechRequest>()
        override fun initialize(done: (Boolean) -> Unit) { initialized = done }
        override fun selectVoice() = reason
        override fun speak(request: SpeechRequest, done: (Long, String) -> Unit): Boolean {
            if (throwsOnSpeak) error("engine failure")
            requests += request; callbacks[request.id] = done; return accepts
        }
        override fun stop() { stopped++; if (throwsOnStop) error("stop failure") }
        override fun shutdown() { shutdowns++ }
        fun complete(id: Long, outcome: String) { callbacks.getValue(id)(id, outcome) }
    }
    private class Rig {
        var time = 0L
        var denyFocus = false
        var noEngine = false
        var focusRequests = 0
        var releases = 0
        val losses = mutableListOf<() -> Unit>()
        val engines = mutableListOf<Engine>()
        val scheduled = mutableListOf<Pair<Long, () -> Unit>>()
        val events = mutableListOf<Triple<String, Long, String?>>()
        val executor = SpeechExecutor(
            createEngine = { if (noEngine) error("no engine"); Engine().also { engines += it } },
            acquireFocus = { loss -> focusRequests++; losses += loss; if (denyFocus) null else ({ releases++; Unit }) },
            now = { time }, later = { delay, action -> scheduled += (time + delay) to action },
            report = { kind, id, reason -> events += Triple(kind, id, reason) },
        )
        fun ready(): Engine { executor.refresh(); return engines.last().also { it.initialized(true) } }
        fun request(id: Long = 1) = SpeechRequest(id, time + 10_000, "motor" to 72, 25, false, 50)
        fun advance(ms: Long) {
            time += ms
            val due = scheduled.filter { it.first <= time }; scheduled.removeAll(due.toSet())
            due.forEach { it.second() }
        }
        fun outcomes(id: Long) = events.filter { it.first == "outcome" && it.second == id }.map { it.third }
    }
    @Test fun unavailableEngineInitFailureAndMissingVoiceRecoverOnRefresh() {
        val r = Rig(); r.noEngine = true; r.executor.refresh()
        assertEquals("engine_unavailable", r.events.last().third)
        r.executor.speak(r.request()); assertEquals(0, r.focusRequests)
        r.noEngine = false; r.executor.refresh(); r.engines.last().initialized(false)
        assertEquals("engine_unavailable", r.events.last().third)
        assertEquals(1, r.engines.last().shutdowns)
        r.executor.refresh(); r.engines.last().reason = "no_offline_voice"; r.engines.last().initialized(true)
        assertEquals("no_offline_voice", r.events.last().third)
        r.ready(); assertEquals("ready", r.events.last().first)
    }
    @Test fun initTimeoutCloseAndLateCallbacksCannotReviveOldEngine() {
        val r = Rig(); r.executor.refresh(); val old = r.engines.last()
        r.advance(10_000); assertEquals(1, old.shutdowns)
        val current = r.ready(); val count = r.events.size
        old.initialized(true); assertEquals(count, r.events.size)
        r.executor.close(); r.executor.close(); r.executor.refresh(); current.initialized(true)
        assertEquals(1, current.shutdowns); assertEquals(2, r.engines.size)
        r.executor.speak(r.request()); assertEquals(0, r.focusRequests)
    }
    @Test fun deniedFocusDoesNotSpeakOrQueueAndFreshAttemptCanRecover() {
        val r = Rig(); val e = r.ready(); r.denyFocus = true
        r.executor.speak(r.request()); assertEquals(listOf("unavailable"), r.outcomes(1))
        assertTrue(e.requests.isEmpty()); assertEquals(0, r.releases)
        r.denyFocus = false; r.executor.speak(r.request(2)); e.complete(2, "finished")
        assertEquals(1, r.releases); assertEquals(listOf("finished"), r.outcomes(2))
    }
    @Test fun completionErrorStopCancelFocusLossAndTimeoutReleaseExactlyOnce() {
        for (terminal in listOf("finished", "unavailable", "interrupted", "cancel", "focus", "route", "timeout", "close")) {
            val r = Rig(); val e = r.ready(); r.executor.speak(r.request())
            when (terminal) {
                "cancel" -> r.executor.cancel(1)
                "focus" -> r.losses.single()()
                "route" -> r.executor.interrupt()
                "timeout" -> r.advance(10_000)
                "close" -> r.executor.close()
                else -> e.complete(1, terminal)
            }
            e.complete(1, "finished"); r.executor.cancel(1); r.advance(10_000)
            assertEquals(terminal, 1, r.releases); assertEquals(terminal, 1, e.stopped)
            assertEquals(terminal, 1, r.outcomes(1).size)
        }
    }
    @Test fun lateCompletionFocusLossAndWatchdogCannotReleaseReplacement() {
        val r = Rig(); val e = r.ready(); r.executor.speak(r.request()); r.executor.cancel(1)
        r.time = 1_000; r.executor.speak(r.request(2))
        e.complete(1, "finished"); r.losses.first()(); r.executor.cancel(1); r.advance(9_000)
        assertEquals(1, r.releases); assertTrue(r.outcomes(2).isEmpty())
        e.complete(2, "finished"); assertEquals(2, r.releases)
    }
    @Test fun busyAndExpiredRequestsNeverAcquireFocusAndPlaybackFailuresReleaseIt() {
        val r = Rig(); val e = r.ready()
        r.executor.speak(r.request().copy(expires = 0)); assertEquals(0, r.focusRequests)
        r.executor.speak(r.request(2)); r.executor.speak(r.request(3)); assertEquals(1, r.focusRequests)
        assertEquals(0.5, e.requests.single().volume / 100.0, 0.0)
        r.executor.cancel(2); e.accepts = false; r.executor.speak(r.request(4))
        assertEquals(listOf("unavailable"), r.outcomes(4)); assertEquals(2, r.releases)
        e.throwsOnSpeak = true; r.executor.speak(r.request(5))
        assertEquals(listOf("unavailable"), r.outcomes(5)); assertEquals(3, r.releases)
    }
    @Test fun returningFromVoiceChooserDefersReinitUntilActiveSpeechFinishes() {
        val r = Rig(); val old = r.ready(); r.executor.speak(r.request())
        r.executor.voiceSettingsOpened(); r.executor.refresh()
        assertEquals(0, old.shutdowns); assertEquals(0, r.releases)
        old.complete(1, "finished"); assertEquals(1, r.releases); assertEquals(1, old.shutdowns)
        val current = r.engines.last(); current.initialized(true)
        r.executor.speak(r.request(2)); old.complete(1, "interrupted")
        assertEquals(1, r.releases); current.complete(2, "finished"); assertEquals(2, r.releases)
    }
    @Test fun chooserReturnDuringInitializationReplacesOnlyAfterInitCompletion() {
        val r = Rig(); r.executor.refresh(); val old = r.engines.last()
        r.executor.voiceSettingsOpened(); r.executor.refresh(); old.initialized(true)
        assertEquals(1, old.shutdowns); assertEquals(2, r.engines.size)
        r.engines.last().initialized(true); assertEquals("ready", r.events.last().first)
        r.advance(10_000); assertEquals("ready", r.events.last().first)
    }
    @Test fun duplicateInitializationCannotChangeReadinessAndStopFailureStillReleasesFocus() {
        val r = Rig(); val e = r.ready(); val count = r.events.size
        e.initialized(false); assertEquals(count, r.events.size)
        e.throwsOnStop = true; r.executor.speak(r.request()); e.complete(1, "finished")
        assertEquals(1, r.releases); assertEquals(listOf("finished"), r.outcomes(1))
    }
}
