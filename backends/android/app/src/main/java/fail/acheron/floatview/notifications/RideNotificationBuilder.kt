package fail.acheron.floatview.notifications

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import android.content.res.Configuration
import androidx.core.graphics.toColorInt
import android.os.Build
import android.os.Bundle
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
    val builder = Notification.Builder(context, channel)
        .setSmallIcon(R.drawable.ic_ride_notification)
        .setContentTitle(title)
        .setContentText(text)
        .setStyle(Notification.BigTextStyle().bigText(text))
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
    val dark = context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
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
