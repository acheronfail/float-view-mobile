package fail.acheron.floatview.application

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
    fun `sampling tolerates bounded timer jitter`() {
        val policy = AdaptiveSamplingPolicy(SamplingSettings(1_000, 100, 500))
        assertTrue(policy.shouldPersist(RideState.RIDING, 0))
        assertFalse(policy.shouldPersist(RideState.RIDING, 79))
        assertTrue(policy.shouldPersist(RideState.RIDING, 80))
        assertFalse(policy.shouldPersist(RideState.RIDING, 159))
        assertTrue(policy.shouldPersist(RideState.RIDING, 160))
    }
}
