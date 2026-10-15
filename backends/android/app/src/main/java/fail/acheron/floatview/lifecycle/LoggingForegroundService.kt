package fail.acheron.floatview.lifecycle

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.content.ContextCompat
import fail.acheron.floatview.R
import fail.acheron.floatview.ui.MainActivity
import java.util.concurrent.atomic.AtomicBoolean
import fail.acheron.floatview.notifications.RideNotification
import fail.acheron.floatview.notifications.RideNotificationDismissal
import fail.acheron.floatview.notifications.buildRideNotification

/**
 * Keeps the process-level Bluetooth and durable logger owner eligible while a
 * device connection or session is active. The service owns no product state: BackendRuntime remains
 * the single owner, so Activity/WebView recreation cannot duplicate a job.
 */
class LoggingForegroundService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private val dismissal = RideNotificationDismissal()
    private var lastPresentation: RideNotification? = null
    private var lastAllowed = false
    private val updates = object : Runnable {
        override fun run() {
            val model = presentation()
            val allowed = canNotify()
            if (!dismissal.suppressed(model.generation) && (model != lastPresentation || !lastAllowed) && allowed) {
                getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, notification(model))
                lastPresentation = model
            }
            lastAllowed = allowed
            handler.postDelayed(this, 1000)
        }
    }

    private fun presentation(): RideNotification {
        val service = (application as FloatViewApplication).runtime.service
        return RideNotification.from(service.liveActivity, service.recording, System.currentTimeMillis())
    }

    private fun canNotify(): Boolean = Build.VERSION.SDK_INT < 33 ||
        checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    override fun onCreate() {
        super.onCreate()
        ensureChannel()
        enterForeground()
        running.set(true)
        if (!desiredRunning.get()) stopSelf()
        else handler.postDelayed(updates, 1000)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_DISMISS) {
            if (intent.getLongExtra("generation", -1) == (lastPresentation?.generation ?: -1)) {
                dismissal.dismiss(lastPresentation?.generation)
            }
            return START_NOT_STICKY
        }
        if (intent?.action == ACTION_STOP) {
            (application as FloatViewApplication).runtime.service.requestStop()
            stopSelf()
            return START_NOT_STICKY
        }
        if (intent?.action == ACTION_REFRESH_FOREGROUND_TYPE) enterForeground()
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        running.set(false)
        handler.removeCallbacks(updates)
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun enterForeground() {
        val model = presentation()
        // Required foreground-type changes (for example newly granted location)
        // must still work after dismissal. Do not re-request promotion then.
        val notification = notification(model, !dismissal.suppressed(model.generation))
        lastPresentation = model
        val speechEnabled = (application as FloatViewApplication).runtime.service.announcementsEnabled
        val connectedDeviceType = ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE or
            (if (speechEnabled) ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK else 0)
        val requestedType = if (hasLocationPermission()) {
            connectedDeviceType or ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION
        } else {
            connectedDeviceType
        }
        try {
            startForeground(NOTIFICATION_ID, notification, requestedType)
        } catch (error: SecurityException) {
            // A location FGS may only be promoted while the app is eligible to
            // start one. Keep board logging alive with the connected-device
            // type and retry promotion when permission/lifecycle state changes.
            if (requestedType == connectedDeviceType) throw error
            startForeground(NOTIFICATION_ID, notification, connectedDeviceType)
        }
    }

    private fun hasLocationPermission(): Boolean =
        ContextCompat.checkSelfPermission(this, Manifest.permission.ACCESS_COARSE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED ||
            ContextCompat.checkSelfPermission(this, Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED

    private fun ensureChannel() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                getString(R.string.logging_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = getString(R.string.logging_channel_description)
                setShowBadge(false)
            },
        )
    }

    private fun notification(model: RideNotification, allowPromotion: Boolean = true): Notification {
        val openIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val stopIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, LoggingForegroundService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val dismissIntent = PendingIntent.getService(
            this, 2,
            Intent(this, LoggingForegroundService::class.java).setAction(ACTION_DISMISS)
                .putExtra("generation", model.generation ?: -1),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return buildRideNotification(this, CHANNEL_ID, model, openIntent, stopIntent, dismissIntent, allowPromotion)
    }

    companion object {
        private const val CHANNEL_ID = "active_logging"
        private const val NOTIFICATION_ID = 1001
        private const val ACTION_DISMISS = "fail.acheron.floatview.DISMISS_RIDE"
        private const val ACTION_STOP = "fail.acheron.floatview.STOP_LOGGING"
        private const val ACTION_REFRESH_FOREGROUND_TYPE =
            "fail.acheron.floatview.REFRESH_FOREGROUND_TYPE"
        internal val running = AtomicBoolean(false)
        private val desiredRunning = AtomicBoolean(false)

        fun start(context: Context) {
            desiredRunning.set(true)
            ContextCompat.startForegroundService(
                context,
                Intent(context, LoggingForegroundService::class.java),
            )
        }

        fun stop(context: Context) {
            desiredRunning.set(false)
            if (running.get()) {
                context.stopService(Intent(context, LoggingForegroundService::class.java))
            }
        }

        fun refreshForegroundType(context: Context) {
            if (running.get()) {
                context.startService(
                    Intent(context, LoggingForegroundService::class.java)
                        .setAction(ACTION_REFRESH_FOREGROUND_TYPE),
                )
            }
        }
    }
}
