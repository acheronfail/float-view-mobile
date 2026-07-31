package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatCompatibility
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import java.nio.file.Files
import java.nio.file.Path
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ControllerDiscoveryVectorTest {
    @Test
    fun dynamicRefloatDiscoveryRequiresAuditedCompleteLayout() {
        val supported = ControllerDiscovery()
        assertEquals(
            ControllerDiscoveryEffect.RequestTelemetryLayout,
            supported.acceptRefloatInfo("1.2.1"),
        )
        assertEquals(
            ControllerDiscoveryEffect.Ready,
            supported.acceptTelemetryLayout(
                RefloatCompatibility.requiredFields + "future.energy",
                RefloatCompatibility.requiredRuntimeFields,
            ),
        )

        val incomplete = ControllerDiscovery()
        assertEquals(
            ControllerDiscoveryEffect.RequestTelemetryLayout,
            incomplete.acceptRefloatInfo("1.1.2"),
        )
        val failed = incomplete.acceptTelemetryLayout(
            RefloatCompatibility.requiredFields.drop(1),
            RefloatCompatibility.requiredRuntimeFields,
        )
        assertTrue(failed is ControllerDiscoveryEffect.Failed)
        assertEquals(
            ConnectionIssueCode.UNSUPPORTED_REFLOAT,
            (failed as ControllerDiscoveryEffect.Failed).issue.code,
        )

        val unknown = ControllerDiscovery().acceptRefloatInfo("1.3.0")
        assertTrue(unknown is ControllerDiscoveryEffect.Failed)
        assertEquals(
            ConnectionIssueCode.UNSUPPORTED_REFLOAT,
            (unknown as ControllerDiscoveryEffect.Failed).issue.code,
        )
    }

    @Test
    fun sharedLifecycleVectorsMatchAndroidDiscovery() {
        vectors().forEach { vector ->
            val discovery = ControllerDiscovery()
            val (_, initial) = discovery.begin()
            assertEquals(
                "${vector.name}: initial probe",
                ControllerDiscoveryEffect.ProbeFirmware(ControllerRoute.Direct),
                initial,
            )
            val hint = vector.hintCanId?.let {
                ControllerRoutingHint("saved", it, uuid(vector.hintUuidByte))
            }
            var effect = discovery.acceptTransport(
                firmware(hardwareType = vector.transportHardwareType),
                vector.preferredCanId,
                hint,
            )
            val continuity = ControllerContinuity(
                vector.activeRouteCanId != null,
                vector.activeRouteCanId?.let(ControllerRoute::Can),
                uuid(vector.activeUuidByte),
            )
            repeat(128) {
                effect = when (val current = effect) {
                    is ControllerDiscoveryEffect.ProbeFirmware -> {
                        val canId = (current.route as? ControllerRoute.Can)?.canId
                            ?: return@repeat
                        val target = vector.controllers.firstOrNull { it.canId == canId }
                        if (target == null) {
                            discovery.canProbeTimedOut(
                                canId,
                                discovery.canProbeGeneration,
                            )
                        } else {
                            discovery.acceptCanFirmware(target.firmware, continuity)
                        }
                    }
                    ControllerDiscoveryEffect.PingCan ->
                        discovery.acceptCanIds(vector.canIds, continuity)
                    is ControllerDiscoveryEffect.ClearHintThen -> current.next
                    else -> return@repeat
                }
            }
            assertEquals(vector.expectedEffect, effect.stableName())
            assertEquals(vector.expectedRoute, discovery.controllerRoute.stableName())
            assertEquals(
                vector.expectedCandidates,
                discovery.controllerCandidates.joinToString(",") { it.canId.toString() },
            )
            assertEquals(
                vector.expectedIssue,
                discovery.connectionIssue?.code?.name?.lowercase().orEmpty(),
            )
        }
    }
}

private data class DiscoveryVector(
    val name: String,
    val transportHardwareType: Int,
    val preferredCanId: Int?,
    val hintCanId: Int?,
    val hintUuidByte: Int?,
    val canIds: List<Int>,
    val controllers: List<VectorController>,
    val activeRouteCanId: Int?,
    val activeUuidByte: Int?,
    val expectedEffect: String,
    val expectedRoute: String,
    val expectedCandidates: String,
    val expectedIssue: String,
)

private data class VectorController(
    val canId: Int,
    val firmware: FirmwareRecord,
)

private fun vectors(): List<DiscoveryVector> {
    val path = Path.of(
        System.getProperty("floatViewFixtureRoot"),
        "connection/controller-discovery.tsv",
    )
    return Files.readAllLines(path)
        .filterNot { it.isBlank() || it.startsWith("#") }
        .map { line ->
            val columns = line.split("|", limit = 13)
            DiscoveryVector(
                name = columns[0],
                transportHardwareType = columns[1].toInt(),
                preferredCanId = columns[2].intOrNull(),
                hintCanId = columns[3].intOrNull(),
                hintUuidByte = columns[4].intOrNull(),
                canIds = columns[5].ints(),
                controllers = columns[6].controllers(),
                activeRouteCanId = columns[7].intOrNull(),
                activeUuidByte = columns[8].intOrNull(),
                expectedEffect = columns[9],
                expectedRoute = columns[10],
                expectedCandidates = columns[11],
                expectedIssue = columns[12],
            )
        }
}

private fun String.intOrNull(): Int? = takeIf(String::isNotEmpty)?.toInt()
private fun String.ints(): List<Int> =
    takeIf(String::isNotEmpty)?.split(",")?.map(String::toInt).orEmpty()

private fun String.controllers(): List<VectorController> =
    takeIf(String::isNotEmpty)?.split(";")?.map { encoded ->
        val (canId, hardwareType, uuidByte, minor) = encoded.split(":").map(String::toInt)
        VectorController(
            canId,
            firmware(hardwareType, canId, uuid(uuidByte), minor),
        )
    }.orEmpty()

private fun firmware(
    hardwareType: Int,
    canId: Int? = null,
    controllerUuid: ByteArray? = null,
    minor: Int = 6,
) = FirmwareRecord(
    major = 6,
    minor = minor,
    hardwareName = if (hardwareType == 0) "THOR300" else "Gateway",
    firmwareName = null,
    hardwareType = hardwareType,
    canId = canId,
    controllerUuid = controllerUuid,
)

private fun uuid(value: Int?): ByteArray? =
    value?.takeIf { it != 0 }?.let { ByteArray(12) { it.toByte() } }

private fun ControllerDiscoveryEffect.stableName(): String = when (this) {
    is ControllerDiscoveryEffect.RequestRefloatInfo -> "request_refloat"
    ControllerDiscoveryEffect.SelectingController -> "selecting"
    is ControllerDiscoveryEffect.Failed -> "failed"
    is ControllerDiscoveryEffect.ProbeFirmware -> "probe"
    ControllerDiscoveryEffect.PingCan -> "ping"
    is ControllerDiscoveryEffect.ClearHintThen -> "clear_hint"
    ControllerDiscoveryEffect.RequestTelemetryLayout -> "layout"
    ControllerDiscoveryEffect.Ready -> "ready"
    ControllerDiscoveryEffect.None -> "none"
}

private fun ControllerRoute?.stableName(): String = when (this) {
    ControllerRoute.Direct -> "direct"
    is ControllerRoute.Can -> "can:$canId"
    null -> ""
}
