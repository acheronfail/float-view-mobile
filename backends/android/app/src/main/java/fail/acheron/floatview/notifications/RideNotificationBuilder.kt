package fail.acheron.floatview.notifications

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.res.Configuration
import androidx.core.graphics.toColorInt
import android.os.Build
import android.os.Bundle
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.ForegroundColorSpan
import android.text.style.TtsSpan
import fail.acheron.floatview.R

internal fun buildRideNotification(
    context: Context,
    channel: String,
    model: RideNotification,
    open: PendingIntent,
    stop: PendingIntent,
    dismiss: PendingIntent,
    allowPromotion: Boolean = true,
): Notification {
    val title = context.getString(if (model.recording) R.string.logging_notification_title else R.string.ride_notification_title)
    val text = when {
        model.generation == null -> context.getString(R.string.ride_notification_waiting_connection)
        model.stale -> context.getString(R.string.ride_notification_waiting_telemetry)
        else -> context.getString(R.string.ride_notification_values, model.load, model.distance, model.battery)
    }
    val dark = context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    // Standard styled text keeps the system layout and promotion eligibility.
    val content = SpannableStringBuilder()
    if (model.generation != null && !model.stale) {
        for ((index, pad) in listOf(model.footpadLeft, model.footpadRight).withIndex()) {
            val color = when (pad) {
                "active" -> if (dark) "#6ee7b7" else "#047857"
                "warning" -> if (dark) "#fda4af" else "#be123c"
                else -> if (dark) "#94a3b8" else "#64748b"
            }
            val start = content.length
            content.append(if (index == 0) "◖" else "◗")
            content.setSpan(ForegroundColorSpan(color.toColorInt()), start, content.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
            val label = when (pad) {
                "active" -> "enabled"
                "warning" -> "disabled while moving"
                else -> "disabled"
            }
            content.setSpan(TtsSpan.TextBuilder("${if (index == 0) "Left" else "Right"} footpad $label")
                .build(), start, content.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
        }
        content.append(" · ")
    }
    content.append(text)
    val builder = Notification.Builder(context, channel)
        .setSmallIcon(R.drawable.ic_ride_notification)
        .setContentTitle(title)
        .setContentText(content)
        .setStyle(Notification.BigTextStyle().bigText(content))
        .setContentIntent(open)
        .setDeleteIntent(dismiss)
        .setOngoing(true)
        .setOnlyAlertOnce(true)
        .setShowWhen(false)
        .setVisibility(Notification.VISIBILITY_PUBLIC)
        .setCategory(Notification.CATEGORY_SERVICE)
        .addAction(Notification.Action.Builder(null, context.getString(R.string.logging_notification_stop), stop).build())
    // Match the gauge hue through the system notification accent; Android owns
    // the text/chip styling. Colorized/custom layouts are ineligible for promotion.
    val accent = when (model.source) {
        "duty" -> if (dark) "#fcd34d" else "#946c00"
        "motor" -> if (dark) "#fda4af" else "#be123c"
        "battery" -> if (dark) "#6ee7b7" else "#047857"
        else -> null
    }
    accent?.let { builder.setColor(it.toColorInt()) }
    if (Build.VERSION.SDK_INT >= 36) {
        // Documented EXTRA_REQUEST_PROMOTED_ONGOING works with compileSdk 36;
        // the convenience setter is only available in SDK 36.1.
        builder.addExtras(Bundle().apply {
            putBoolean("android.requestPromotedOngoing", allowPromotion && model.generation != null && !model.stale)
        })
        builder.setShortCriticalText(model.chip)
    }
    return builder.build()
}
