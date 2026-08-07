package fail.acheron.floatview.location

import android.Manifest
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.net.Uri
import android.os.Build
import android.os.CancellationSignal
import android.os.Looper
import android.provider.Settings
import androidx.core.content.ContextCompat
import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderBackgroundCapture
import fail.acheron.floatview.backend.LocationProviderObservation
import fail.acheron.floatview.backend.LocationProviderPort
import fail.acheron.floatview.backend.LocationProviderState
import java.time.Instant

fun interface LocationPermissionRequester {
    fun request(permissions: Array<String>)
}

class AndroidLocationProvider(
    context: Context,
    private val requester: LocationPermissionRequester,
) : LocationProviderPort {
    private val applicationContext = context.applicationContext
    private val locationManager =
        applicationContext.getSystemService(Context.LOCATION_SERVICE) as LocationManager
    private val preferences =
        applicationContext.getSharedPreferences("location-permission-history", Context.MODE_PRIVATE)
    private var listener: LocationListener? = null
    private var currentLocationCancellation: CancellationSignal? = null
    private var handler: ((LocationProviderObservation) -> Unit)? = null

    @Synchronized
    override fun state(): LocationProviderState {
        val enabled = runCatching {
            if (Build.VERSION.SDK_INT >= 28) locationManager.isLocationEnabled
            else locationManager.isProviderEnabled(LocationManager.GPS_PROVIDER) ||
                locationManager.isProviderEnabled(LocationManager.NETWORK_PROVIDER)
        }.getOrDefault(false)
        if (!enabled) {
            return LocationProviderState(
                LocationProviderAuthorization.UNAVAILABLE,
                LocationProviderBackgroundCapture.UNAVAILABLE,
                "Location services are disabled.",
            )
        }
        if (!hasForegroundPermission()) {
            return LocationProviderState(
                if (preferences.getBoolean("requested", false)) {
                    LocationProviderAuthorization.DENIED
                } else {
                    LocationProviderAuthorization.NOT_DETERMINED
                },
                LocationProviderBackgroundCapture.UNAVAILABLE,
                if (preferences.getBoolean("requested", false)) "Location permission was denied." else "Location permission has not been resolved.",
            )
        }
        val background = if (hasBackgroundPermission()) {
            LocationProviderBackgroundCapture.AVAILABLE
        } else {
            LocationProviderBackgroundCapture.FOREGROUND_ONLY
        }
        return LocationProviderState(
            if (background == LocationProviderBackgroundCapture.AVAILABLE) {
                LocationProviderAuthorization.AUTHORIZED
            } else {
                LocationProviderAuthorization.FOREGROUND_ONLY
            },
            background,
            if (background == LocationProviderBackgroundCapture.FOREGROUND_ONLY) {
                "Background location access is not enabled."
            } else null,
        )
    }

    @Synchronized
    override fun requestPermission() {
        preferences.edit().putBoolean("requested", true).apply()
        if (!hasForegroundPermission()) {
            requester.request(
                arrayOf(
                    Manifest.permission.ACCESS_COARSE_LOCATION,
                    Manifest.permission.ACCESS_FINE_LOCATION,
                ),
            )
        } else if (Build.VERSION.SDK_INT >= 29 && !hasBackgroundPermission()) {
            requester.request(arrayOf(Manifest.permission.ACCESS_BACKGROUND_LOCATION))
        }
    }

    override fun openSettings() {
        val intent = Intent(
            Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
            Uri.parse("package:${applicationContext.packageName}"),
        ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        applicationContext.startActivity(intent)
    }

    @Synchronized
    override fun startUpdates(handler: (LocationProviderObservation) -> Unit) {
        if (!hasForegroundPermission()) return
        stopUpdates()
        this.handler = handler
        val next = object : LocationListener {
            override fun onLocationChanged(location: Location) {
                if (!location.latitude.isFinite() || !location.longitude.isFinite()) return
                this@AndroidLocationProvider.handler?.invoke(
                    LocationProviderObservation(
                        observedAt = Instant.ofEpochMilli(location.time),
                        latitude = location.latitude,
                        longitude = location.longitude,
                        accuracyM = location.accuracy.takeIf { it >= 0f }?.toDouble(),
                    ),
                )
            }
        }
        listener = next
        val providers = listOf(LocationManager.NETWORK_PROVIDER, LocationManager.GPS_PROVIDER)
            .filter { runCatching { locationManager.isProviderEnabled(it) }.getOrDefault(false) }
        providers.forEach { provider ->
            runCatching {
                locationManager.requestLocationUpdates(provider, 1_000L, 5f, next, Looper.getMainLooper())
            }
        }
        val lastKnown = locationManager.allProviders
            .mapNotNull { provider -> runCatching { locationManager.getLastKnownLocation(provider) }.getOrNull() }
            .filter { location -> location.latitude.isFinite() && location.longitude.isFinite() }
            .maxByOrNull { location -> location.time }
        if (lastKnown != null) {
            next.onLocationChanged(lastKnown)
        } else if (Build.VERSION.SDK_INT >= 30) {
            providers.firstOrNull()?.let { provider ->
                val cancellation = CancellationSignal()
                currentLocationCancellation = cancellation
                runCatching {
                    locationManager.getCurrentLocation(
                        provider,
                        cancellation,
                        ContextCompat.getMainExecutor(applicationContext),
                    ) { location -> location?.let(next::onLocationChanged) }
                }
            }
        }
    }

    @Synchronized
    override fun stopUpdates() {
        currentLocationCancellation?.cancel()
        currentLocationCancellation = null
        listener?.also { locationManager.removeUpdates(it) }
        listener = null
        handler = null
    }

    private fun hasForegroundPermission(): Boolean =
        ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED ||
            ContextCompat.checkSelfPermission(applicationContext, Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED

    private fun hasBackgroundPermission(): Boolean =
        Build.VERSION.SDK_INT < 29 || ContextCompat.checkSelfPermission(
            applicationContext,
            Manifest.permission.ACCESS_BACKGROUND_LOCATION,
        ) == PackageManager.PERMISSION_GRANTED
}
