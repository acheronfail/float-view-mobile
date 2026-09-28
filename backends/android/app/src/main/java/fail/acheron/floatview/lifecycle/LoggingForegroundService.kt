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
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import fail.acheron.floatview.R
import fail.acheron.floatview.ui.MainActivity
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Keeps the process-level Bluetooth and durable logger owner eligible while a
 * session is active. The service owns no product state: BackendRuntime remains
 * the single owner, so Activity/WebView recreation cannot duplicate a job.
 */
class LoggingForegroundService : Service() {
    override fun onCreate() {
        super.onCreate()
        ensureChannel()
        enterForeground()
        running.set(true)
        if (!desiredRunning.get()) stopSelf()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
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
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun enterForeground() {
        val connectedDeviceType = ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
        val requestedType = if (hasLocationPermission()) {
            connectedDeviceType or ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION
        } else {
            connectedDeviceType
        }
        try {
            startForeground(NOTIFICATION_ID, notification(), requestedType)
        } catch (error: SecurityException) {
            // A location FGS may only be promoted while the app is eligible to
            // start one. Keep board logging alive with the connected-device
            // type and retry promotion when permission/lifecycle state changes.
            if (requestedType == connectedDeviceType) throw error
            startForeground(NOTIFICATION_ID, notification(), connectedDeviceType)
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

    private fun notification(): Notification {
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
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(getString(R.string.logging_notification_title))
            .setContentText(getString(R.string.logging_notification_text))
            .setContentIntent(openIntent)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .addAction(0, getString(R.string.logging_notification_stop), stopIntent)
            .build()
    }

    companion object {
        private const val CHANNEL_ID = "active_logging"
        private const val NOTIFICATION_ID = 1001
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
