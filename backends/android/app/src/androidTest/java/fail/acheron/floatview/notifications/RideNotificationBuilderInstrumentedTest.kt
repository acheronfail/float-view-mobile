package fail.acheron.floatview.notifications

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.os.Build
import android.text.Spanned
import android.text.style.ForegroundColorSpan
import android.text.style.StyleSpan
import android.text.style.TtsSpan
import androidx.test.platform.app.InstrumentationRegistry
import fail.acheron.floatview.application.LiveActivityState
import fail.acheron.floatview.ui.MainActivity
import org.junit.Assert.*
import org.junit.Test
import org.junit.Before
import org.junit.After
import java.util.Locale

class RideNotificationBuilderInstrumentedTest {
    private val originalLocale = Locale.getDefault()
    @Before fun setLocale() { Locale.setDefault(Locale.US) }
    @After fun restoreLocale() { Locale.setDefault(originalLocale) }
    private val context: Context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private val state = LiveActivityState(
        generation = 7, observedAt = 100000, source = "motor", percentage = 82.0,
        distanceMeters = 1609.344, batteryPercent = 67.0, batteryVoltage = 75.56,
        highestTemperatureC = 54.0, temperatureSource = "Motor", imperial = true,
        status = null, footpadLeft = "active", footpadRight = "warning",
    )

    private fun build(
        state: LiveActivityState? = this.state, now: Long = 100001,
        allowPromotion: Boolean = true, dark: Boolean = false,
    ): Notification {
        val config = Configuration(context.resources.configuration).apply {
            setLocale(Locale.US)
            uiMode = (uiMode and Configuration.UI_MODE_NIGHT_MASK.inv()) or
                if (dark) Configuration.UI_MODE_NIGHT_YES else Configuration.UI_MODE_NIGHT_NO
        }
        val themed = context.createConfigurationContext(config)
        val open = PendingIntent.getActivity(context, 0, Intent(context, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        return buildRideNotification(themed, "notification_test", RideNotification.from(state, false, now), open, open, open, allowPromotion)
    }

    @Test fun freshReadingsHaveSeparateLabelledRowsAndACompactSummary() {
        val notification = build()
        val rows = notification.extras.getCharSequence(Notification.EXTRA_BIG_TEXT) as Spanned
        assertEquals(
            "◖◗ · Load 82% · Motor\nDistance 1.00 mi\nBattery 67% · 75.6 V\n🌡\uFE0E Temp 129°F · Motor",
            rows.toString(),
        )
        assertEquals("Load 82% · Temp 129°F", notification.extras.getCharSequence(Notification.EXTRA_TEXT).toString())
        val headings = rows.getSpans(0, rows.length, StyleSpan::class.java).map {
            rows.subSequence(rows.getSpanStart(it), rows.getSpanEnd(it)).toString()
        }
        assertEquals(listOf("Load", "Distance", "Battery", "Temp"), headings)
    }

    @Test fun stoppedBoardHasNoLoadSourceAndMissingTemperatureStaysExplicit() {
        val stopped = build(state.copy(status = "Stopped"))
        assertTrue(stopped.extras.getCharSequence(Notification.EXTRA_BIG_TEXT).toString().startsWith("◖◗ · Board Stopped\n"))
        assertEquals("Board Stopped · Temp 129°F", stopped.extras.getCharSequence(Notification.EXTRA_TEXT).toString())
        val missing = build(state.copy(highestTemperatureC = null, temperatureSource = null))
        assertTrue(missing.extras.getCharSequence(Notification.EXTRA_BIG_TEXT).toString().endsWith("Temp —"))
    }

    @Test fun staleAndDisconnectedNotificationsDoNotShowOldMetrics() {
        for ((notification, expected) in listOf(
            build(now = 115000) to "Waiting for telemetry",
            build(state = null) to "Waiting for the device connection",
        )) {
            assertEquals(expected, notification.extras.getCharSequence(Notification.EXTRA_TEXT).toString())
            assertEquals(expected, notification.extras.getCharSequence(Notification.EXTRA_BIG_TEXT).toString())
            assertFalse(notification.extras.getBoolean("android.requestPromotedOngoing"))
        }
    }

    @Test fun temperatureColourOnlyCoversTheIconAndSpokenTextNamesTheSensor() {
        for (dark in listOf(false, true)) {
            val rows = build(dark = dark).extras.getCharSequence(Notification.EXTRA_BIG_TEXT) as Spanned
            val temperatureStart = rows.toString().indexOf("🌡")
            val spans = rows.getSpans(temperatureStart, rows.length, ForegroundColorSpan::class.java)
            assertEquals(1, spans.size)
            assertEquals("🌡\uFE0E", rows.subSequence(rows.getSpanStart(spans[0]), rows.getSpanEnd(spans[0])).toString())
            val spoken = rows.getSpans(temperatureStart, rows.length, TtsSpan::class.java).single()
            assertEquals("Temperature, Motor, 129°F", spoken.args.getString(TtsSpan.ARG_TEXT))
        }
    }

    @Test fun standardTemplateRetainsPromotionRequestAndStopAction() {
        val notification = build()
        assertNull(notification.contentView)
        assertNull(notification.bigContentView)
        assertEquals(Notification.BigTextStyle::class.java.name, notification.extras.getString(Notification.EXTRA_TEMPLATE))
        assertTrue(notification.flags and Notification.FLAG_ONGOING_EVENT != 0)
        assertFalse(notification.extras.getBoolean("android.colorized"))
        assertEquals("Stop and disconnect", notification.actions.single().title.toString())
        assertNotNull(notification.contentIntent)
        assertNotNull(notification.deleteIntent)
        if (Build.VERSION.SDK_INT >= 36) {
            assertTrue(notification.extras.getBoolean("android.requestPromotedOngoing"))
            assertFalse(build(allowPromotion = false).extras.getBoolean("android.requestPromotedOngoing"))
        }
    }
}
