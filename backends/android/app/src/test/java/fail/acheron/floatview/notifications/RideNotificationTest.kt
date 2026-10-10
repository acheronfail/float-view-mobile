package fail.acheron.floatview.notifications

import fail.acheron.floatview.application.LiveActivityState
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.Before
import org.junit.After
import java.util.Locale

class RideNotificationTest {
    private val originalLocale = Locale.getDefault()
    @Before fun setLocale() { Locale.setDefault(Locale.US) }
    @After fun restoreLocale() { Locale.setDefault(originalLocale) }
    private fun state() = LiveActivityState.decode(JSONObject("""
        {"footpad_left":"active","footpad_right":"warning","status":null,"generation":7,"observed_at":100000,"source":"motor","percentage":82,
         "distance_meters":1609.344,"battery_voltage":75.56,"battery_percent":67,"imperial":true}
    """))

    @Test fun nativeProjectionFormatsRideWithoutRecalculatingGauge() {
        val model = RideNotification.from(state(), true, 100001)
        assertEquals("Motor 82%", model.load)
        assertEquals("1.00 mi", model.distance)
        assertEquals("67% · 75.6 V", model.battery)
        assertEquals("82%", model.chip)
        assertFalse(model.stale)
        assertTrue(model.recording)
        assertEquals("1.61 km", RideNotification.from(state().copy(imperial = false), false, 100001).distance)
    }

    @Test fun staleAndMissingReadingsNeverLookLive() {
        val stale = RideNotification.from(state(), false, 115000)
        assertTrue(stale.stale)
        assertEquals("Motor —", stale.load)
        assertEquals("—", stale.distance)
        assertEquals("—", stale.battery)
        assertNull(stale.chip)
        val missing = state().copy(observedAt = null, percentage = null, batteryPercent = null)
        assertTrue(RideNotification.from(missing, false, 100001).stale)
        assertNull(RideNotification.from(null, true, 100001).generation)
    }

    @Test fun stoppedBoardReplacesLoadIncludingTheCompactChip() {
        val stopped = RideNotification.from(state().copy(status = "Stopped"), false, 100001)
        assertEquals("Stopped", stopped.load)
        assertEquals("Stopped", stopped.chip)
        assertNull(stopped.source)
        assertEquals("active", stopped.footpadLeft)
        assertEquals("warning", stopped.footpadRight)
        assertNotEquals(stopped, RideNotification.from(state().copy(status = "Stopped", footpadRight = "active"), false, 100001))
        val stale = RideNotification.from(state().copy(status = "Stopped"), false, 115000)
        assertNull(stale.chip)
        assertEquals("neutral", stale.footpadLeft)
        assertEquals("neutral", stale.footpadRight)
    }

    @Test fun voltageIsIndependentOfChargeAvailability() {
        assertEquals("— · 75.6 V", RideNotification.from(state().copy(batteryPercent = null), false, 100001).battery)
        assertEquals("67% · —", RideNotification.from(state().copy(batteryVoltage = null), false, 100001).battery)
    }

    @Test fun timestampsAreCoalescedButVisibleChangesArePublished() {
        val initial = RideNotification.from(state(), true, 100001)
        assertEquals(initial, RideNotification.from(state().copy(observedAt = 100001), true, 100002))
        assertNotEquals(initial, RideNotification.from(state().copy(batteryVoltage = 76.0), true, 100002))
        assertNotEquals(initial, RideNotification.from(state().copy(percentage = 83.0), true, 100002))
    }

    @Test fun dismissalSurvivesMissingReadingsUntilANewConnection() {
        val dismissal = RideNotificationDismissal()
        assertFalse(dismissal.suppressed(7))
        dismissal.dismiss(7)
        assertTrue(dismissal.suppressed(7))
        assertTrue(dismissal.suppressed(null))
        assertFalse(dismissal.suppressed(8))
    }

    @Test fun dismissalWhileConnectingAlsoSuppressesFirstReadyReadings() {
        val dismissal = RideNotificationDismissal()
        dismissal.dismiss(null)
        assertTrue(dismissal.suppressed(null))
        assertTrue(dismissal.suppressed(7))
        assertTrue(dismissal.suppressed(7))
        assertFalse(dismissal.suppressed(8))
    }

    @Test fun nullPayloadValuesRemainUnavailable() {
        val state = LiveActivityState.decode(JSONObject("""
            {"footpad_left":"active","footpad_right":"warning","status":null,"generation":8,"observed_at":null,"source":null,"percentage":null,
             "distance_meters":null,"battery_voltage":null,"battery_percent":null,"imperial":false}
        """))
        assertNull(state.observedAt)
        assertNull(state.source)
        assertNull(state.percentage)
        assertNull(state.batteryPercent)
        assertNull(state.batteryVoltage)
    }
}
