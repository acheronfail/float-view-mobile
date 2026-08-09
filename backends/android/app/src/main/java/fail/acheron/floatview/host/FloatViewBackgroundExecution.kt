package fail.acheron.floatview.host

import android.content.Context
import fail.acheron.floatview.LoggingForegroundService

// Capability status: implemented — foreground-service and physical background evidence pass.
// Concrete OS owner: LoggingForegroundService.
internal object FloatViewBackgroundExecution {
    fun setLoggingEligible(context: Context, enabled: Boolean): Boolean {
        return runCatching {
            if (enabled) LoggingForegroundService.start(context)
            else LoggingForegroundService.stop(context)
            true
        }.getOrDefault(false)
    }

    fun effectiveEligibility(foreground: Boolean): Boolean =
        foreground || LoggingForegroundService.running.get()
}
