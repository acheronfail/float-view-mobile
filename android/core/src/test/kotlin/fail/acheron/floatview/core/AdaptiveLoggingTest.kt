package fail.acheron.floatview.core

import java.time.Instant
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AdaptiveLoggingTest {
    @Test
    fun `sampling persists transitions riding cadence and tail`() {
        val policy = AdaptiveSamplingPolicy(SamplingSettings(1_000, 100, 500))
        assertTrue(policy.shouldPersist(RideState.IDLE, 0))
        assertFalse(policy.shouldPersist(RideState.IDLE, 500))
        assertTrue(policy.shouldPersist(RideState.RIDING, 600))
        assertTrue(policy.shouldPersist(RideState.RIDING, 700))
        assertTrue(policy.shouldPersist(RideState.IDLE, 750))
        assertTrue(policy.shouldPersist(RideState.IDLE, 850))
        assertFalse(policy.shouldPersist(RideState.IDLE, 1_300))
    }

    @Test
    fun `alerts require duration and respect cooldown`() {
        val sample = TelemetrySample(
            1, Instant.EPOCH, 0, RideState.RIDING, 58.0, 0.9, 25.0, 20.0, 1,
        )
        val rule = AlertRule("duty", AlertKind.DUTY_HIGH, 0.8, 500, cooldownMs = 1_000)
        val evaluator = AlertEvaluator()
        assertFalse(evaluator.evaluate(rule, sample, true, 0))
        assertTrue(evaluator.evaluate(rule, sample, true, 500))
        assertFalse(evaluator.evaluate(rule, sample, true, 600))
        assertTrue(evaluator.evaluate(rule, sample, true, 1_500))
    }
}
