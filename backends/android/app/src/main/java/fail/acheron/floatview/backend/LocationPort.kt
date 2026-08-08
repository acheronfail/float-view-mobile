package fail.acheron.floatview.backend

/** Focused Android location port vocabulary transported to the Rust core. */
enum class LocationProviderAuthorization {
    NOT_DETERMINED,
    FOREGROUND_ONLY,
    AUTHORIZED,
    DENIED,
    RESTRICTED,
    UNAVAILABLE,
}

enum class LocationProviderBackgroundCapture { AVAILABLE, FOREGROUND_ONLY, UNAVAILABLE }

data class LocationProviderState(
    val authorization: LocationProviderAuthorization,
    val backgroundCapture: LocationProviderBackgroundCapture,
)

data class LocationProviderObservation(
    val latitude: Double,
    val longitude: Double,
    val accuracyM: Double?,
)

interface LocationProviderPort {
    fun state(): LocationProviderState
    fun setStateHandler(handler: (LocationProviderState) -> Unit) = Unit
    fun requestPermission()
    fun openSettings()
    fun startUpdates(handler: (LocationProviderObservation) -> Unit)
    fun stopUpdates()
}
