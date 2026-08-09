package fail.acheron.floatview.application

import fail.acheron.floatview.protocol.vesc.ControllerRoute
import org.junit.Assert.assertEquals
import org.junit.Test
import java.time.Instant

private fun auxiliaryFirmware(
    hardwareType: Int,
    canId: Int? = null,
    name: String = "node",
    major: Int = 6,
    minor: Int = 0,
) = FirmwareRecord(major, minor, name, null, hardwareType, canId)

class AuxiliaryNodeDiscoveryTest {
    @Test
    fun recognizesOnlyTheObservedVbms32Identity() {
        assertEquals(true, BmsIdentityPolicy.supports(auxiliaryFirmware(2, name = "VBMS32", minor = 6)))
        assertEquals(false, BmsIdentityPolicy.supports(auxiliaryFirmware(2, name = "42508004", minor = 6)))
        val vbms32 = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.RequestValues(BmsConnection.Can(10)),
            vbms32.begin(
                auxiliaryFirmware(0),
                listOf(auxiliaryFirmware(2, 10, "VBMS32", minor = 6)),
                null,
            ),
        )

        listOf(
            auxiliaryFirmware(2, 10, "Gateway", minor = 6),
            auxiliaryFirmware(2, 10, "VBMS32", minor = 5),
            auxiliaryFirmware(2, 10, "VBMS32", major = 7, minor = 0),
        ).forEach { identity ->
            val otherCustomModule = AuxiliaryNodeDiscovery()
            assertEquals(
                AuxiliaryDiscoveryEffect.None,
                otherCustomModule.begin(auxiliaryFirmware(0), listOf(identity), null),
            )
            assertEquals(BmsStatus.NOT_FOUND, otherCustomModule.state.status)
        }
    }

    @Test
    fun selectsDirectAndReusableUniqueVbms() {
        val direct = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.RequestValues(BmsConnection.Direct),
            direct.begin(auxiliaryFirmware(1), null, null),
        )

        val can = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.RequestValues(BmsConnection.Can(42)),
            can.begin(auxiliaryFirmware(0), listOf(auxiliaryFirmware(1, 42)), null),
        )
    }

    @Test
    fun boundsAndClassifiesCandidates() {
        val none = AuxiliaryNodeDiscovery()
        assertEquals(AuxiliaryDiscoveryEffect.None, none.begin(auxiliaryFirmware(0), emptyList(), null))
        assertEquals(BmsStatus.NOT_FOUND, none.state.status)

        val multiple = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.None,
            multiple.begin(
                auxiliaryFirmware(0),
                listOf(auxiliaryFirmware(1, 8), auxiliaryFirmware(1, 3)),
                null,
            ),
        )
        assertEquals(BmsStatus.AMBIGUOUS, multiple.state.status)
        assertEquals(listOf(BmsConnection.Can(3), BmsConnection.Can(8)), multiple.state.candidates.map { it.connection })

        val overflow = AuxiliaryNodeDiscovery()
        assertEquals(AuxiliaryDiscoveryEffect.PingCan, overflow.begin(auxiliaryFirmware(0), null, null))
        assertEquals(AuxiliaryDiscoveryEffect.None, overflow.acceptCanIds((0..64).toList(), overflow.generation))
        assertEquals(BmsStatus.UNAVAILABLE, overflow.state.status)
    }

    @Test
    fun directlyProbesPreferencesAndRejectsWrongIdentity() {
        val discovery = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.ProbeFirmware(ControllerRoute.Can(42)),
            discovery.begin(auxiliaryFirmware(0), emptyList(), 42),
        )
        assertEquals(
            AuxiliaryDiscoveryEffect.None,
            discovery.acceptFirmware(auxiliaryFirmware(0, 42), discovery.generation),
        )
        assertEquals(BmsStatus.UNAVAILABLE, discovery.state.status)

        val selected = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.ProbeFirmware(ControllerRoute.Can(42)),
            selected.begin(auxiliaryFirmware(0), null, 42),
        )
        assertEquals(
            AuxiliaryDiscoveryEffect.RequestValues(BmsConnection.Can(42)),
            selected.acceptFirmware(auxiliaryFirmware(1, 42), selected.generation),
        )
    }

    @Test
    fun retainsStaleSnapshotRecoversAndResetsAcrossGenerations() {
        val discovery = AuxiliaryNodeDiscovery()
        assertEquals(
            AuxiliaryDiscoveryEffect.RequestValues(BmsConnection.Can(42)),
            discovery.begin(auxiliaryFirmware(0), listOf(auxiliaryFirmware(1, 42)), null),
        )
        val snapshot = bmsSnapshot(42)
        discovery.acceptSnapshot(snapshot)
        assertEquals(BmsStatus.ATTACHED, discovery.state.status)
        discovery.markUnavailable()
        assertEquals(BmsStatus.UNAVAILABLE, discovery.state.status)
        assertEquals(snapshot, discovery.state.latestSnapshot)
        discovery.acceptSnapshot(snapshot)
        assertEquals(BmsStatus.ATTACHED, discovery.state.status)

        assertEquals(
            AuxiliaryDiscoveryEffect.ProbeFirmware(ControllerRoute.Can(43)),
            discovery.begin(auxiliaryFirmware(0), null, 43),
        )
        assertEquals(null, discovery.state.latestSnapshot)
        discovery.reset()
        assertEquals(BmsState(), discovery.state)
    }

    private fun bmsSnapshot(canId: Int) = BmsSnapshot(
        Instant.ofEpochSecond(1), 50.0, 54.0, 0.0, 0.0, 0.0, 0.0,
        listOf(4.1), listOf(false), listOf(25.0), 25.0, 24.0, 50.0, 25.0,
        0.8, 0.9, canId, 1.0, 2.0, 1.0, 2.0, 101_325.0, 1, "OK",
    )
}
