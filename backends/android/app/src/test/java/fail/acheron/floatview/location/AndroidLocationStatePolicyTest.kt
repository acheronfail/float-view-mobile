package fail.acheron.floatview.location

import fail.acheron.floatview.backend.LocationProviderAuthorization
import fail.acheron.floatview.backend.LocationProviderBackgroundCapture
import fail.acheron.floatview.backend.LocationProviderState
import org.junit.Assert.assertEquals
import org.junit.Test

class AndroidLocationStatePolicyTest {
    @Test
    fun mapsProviderAndPermissionTransitionsDirectly() {
        assertState(
            providerEnabled = false,
            permissionRequested = false,
            foregroundGranted = false,
            backgroundGranted = false,
            authorization = LocationProviderAuthorization.UNAVAILABLE,
            background = LocationProviderBackgroundCapture.UNAVAILABLE,
        )
        assertState(
            providerEnabled = true,
            permissionRequested = false,
            foregroundGranted = false,
            backgroundGranted = false,
            authorization = LocationProviderAuthorization.NOT_DETERMINED,
            background = LocationProviderBackgroundCapture.UNAVAILABLE,
        )
        assertState(
            providerEnabled = true,
            permissionRequested = true,
            foregroundGranted = false,
            backgroundGranted = false,
            authorization = LocationProviderAuthorization.DENIED,
            background = LocationProviderBackgroundCapture.UNAVAILABLE,
        )
        assertState(
            providerEnabled = true,
            permissionRequested = true,
            foregroundGranted = true,
            backgroundGranted = false,
            authorization = LocationProviderAuthorization.FOREGROUND_ONLY,
            background = LocationProviderBackgroundCapture.FOREGROUND_ONLY,
        )
        assertState(
            providerEnabled = true,
            permissionRequested = true,
            foregroundGranted = true,
            backgroundGranted = true,
            authorization = LocationProviderAuthorization.AUTHORIZED,
            background = LocationProviderBackgroundCapture.AVAILABLE,
        )
    }

    private fun assertState(
        providerEnabled: Boolean,
        permissionRequested: Boolean,
        foregroundGranted: Boolean,
        backgroundGranted: Boolean,
        authorization: LocationProviderAuthorization,
        background: LocationProviderBackgroundCapture,
    ) {
        assertEquals(
            LocationProviderState(authorization, background),
            AndroidLocationStatePolicy.state(
                providerEnabled,
                permissionRequested,
                foregroundGranted,
                backgroundGranted,
            ),
        )
    }
}
