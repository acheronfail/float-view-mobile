package fail.acheron.floatview.protocol.refloat

import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.backend.VescProtocolPort

object RefloatPolling {
    fun layoutRequest(protocol: VescProtocolPort, route: ControllerRoute): ByteArray =
        protocol.refloatTelemetryIdsRequest(route)

    fun telemetryRequest(
        protocol: VescProtocolPort,
        route: ControllerRoute,
        family: RefloatTelemetryFamily,
    ): ByteArray =
        when (family) {
            RefloatTelemetryFamily.LEGACY_FIXED -> protocol.refloatLegacyTelemetryRequest(route)
            RefloatTelemetryFamily.DYNAMIC -> protocol.refloatTelemetryRequest(route)
        }

    fun compactRequest(protocol: VescProtocolPort, route: ControllerRoute): ByteArray =
        protocol.refloatCompactTelemetryRequest(route)
}
