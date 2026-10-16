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
         "distance_meters":1609.344,"battery_voltage":75.56,"battery_percent":67,"highest_temperature_c":54,"temperature_source":"Motor","imperial":true}
    """))

    @Test fun nativeProjectionDecodesHighestTemperature() {
        assertEquals(54.0, state().highestTemperatureC!!, 0.0)
        assertEquals("Motor", state().temperatureSource)
    }

    @Test fun nativeProjectionFormatsRideWithoutRecalculatingGauge() {
        val model = RideNotification.from(state(), true, 100001)
        assertEquals("82%", model.load)
        assertEquals("motor", model.source)
        assertNull(model.status)
        assertEquals("1.00 mi", model.distance)
        assertEquals("67% · 75.6 V", model.battery)
        assertEquals("82%", model.chip)
        assertFalse(model.stale)
        assertTrue(model.recording)
        assertEquals("1.61 km", RideNotification.from(state().copy(imperial = false), false, 100001).distance)
    }

    @Test fun temperatureMatchesIosUnitsAndSensorColorThresholds() {
        fun model(celsius: Double, source: String = "Motor", imperial: Boolean = false) =
            RideNotification.from(state().copy(highestTemperatureC = celsius, temperatureSource = source,
                imperial = imperial), false, 100001)

        assertEquals("54°C", model(54.0).temperature)
        assertEquals("129°F", model(54.0, imperial = true).temperature)
        assertEquals("Motor", model(54.0).temperatureSource)
        assertEquals(0xff16a34a.toInt(), model(30.0).temperatureColor)
        assertEquals(0xffea580c.toInt(), model(45.0).temperatureColor)
        assertEquals(0xffdc2626.toInt(), model(60.0).temperatureColor)
        assertEquals(0xffdc2626.toInt(), model(80.0).temperatureColor)
        assertEquals(0xffea580c.toInt(), model(37.5, "Battery").temperatureColor)
        assertEquals(0xffdc2626.toInt(), model(45.0, "Battery").temperatureColor)
        assertEquals(0xff807e2b.toInt(), model(37.5).temperatureColor)
    }

    @Test fun unavailableOrStaleTemperatureHasNoColorOrSource() {
        for (value in listOf(null, Double.NaN, Double.POSITIVE_INFINITY)) {
            val model = RideNotification.from(state().copy(highestTemperatureC = value), false, 100001)
            assertEquals("—", model.temperature)
            assertNull(model.temperatureColor)
            assertNull(model.temperatureSource)
        }
        val stale = RideNotification.from(state(), false, 115000)
        assertEquals("—", stale.temperature)
        assertNull(stale.temperatureColor)
        assertNull(stale.temperatureSource)
    }

    @Test fun staleAndMissingReadingsNeverLookLive() {
        val stale = RideNotification.from(state(), false, 115000)
        assertTrue(stale.stale)
        assertEquals("—", stale.load)
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
        assertEquals("Stopped", stopped.status)
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
        assertNotEquals(initial, RideNotification.from(state().copy(highestTemperatureC = 55.0), true, 100002))
        assertNotEquals(initial, RideNotification.from(state().copy(temperatureSource = "Battery"), true, 100002))
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
