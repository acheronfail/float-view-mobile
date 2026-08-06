package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.vesc.ControllerRoute

sealed interface AuxiliaryDiscoveryEffect {
    data object PingCan : AuxiliaryDiscoveryEffect
    data class ProbeFirmware(val route: ControllerRoute) : AuxiliaryDiscoveryEffect
    data class RequestValues(val connection: BmsConnection) : AuxiliaryDiscoveryEffect
    data object None : AuxiliaryDiscoveryEffect
}

object BmsIdentityPolicy {
    fun supports(value: FirmwareRecord): Boolean = supportedIdentities.any { it.matches(value) }

    private val supportedIdentities = listOf(
        SupportedIdentity(hardwareType = 1, major = 6, minor = 0),
        SupportedIdentity(hardwareType = 2, major = 6, minor = 6, hardwareName = "VBMS32"),
    )

    private data class SupportedIdentity(
        val hardwareType: Int,
        val major: Int,
        val minor: Int,
        val hardwareName: String? = null,
    ) {
        fun matches(value: FirmwareRecord): Boolean =
            value.hardwareType == hardwareType && value.major == major && value.minor == minor &&
                (hardwareName == null || value.hardwareName == hardwareName)
    }
}

class AuxiliaryNodeDiscovery {
    var generation: Long = 0
        private set
    var state: BmsState = BmsState()
        private set
    var selectedConnection: BmsConnection? = null
        private set
    var activeProbeId: Int? = null
        private set

    private var pendingCanIds = mutableListOf<Int>()
    private val discovered = mutableMapOf<BmsConnection, FirmwareRecord>()
    private var preferredCanId: Int? = null

    fun begin(
        localIdentity: FirmwareRecord,
        reusableInventory: List<FirmwareRecord>?,
        preferredCanId: Int?,
    ): AuxiliaryDiscoveryEffect {
        reset()
        this.preferredCanId = preferredCanId
        if (isVbms(localIdentity)) {
            discovered[BmsConnection.Direct] = localIdentity
            return select(BmsConnection.Direct)
        }
        if (preferredCanId != null) {
            val identity = reusableInventory?.firstOrNull { it.canId == preferredCanId }
            if (identity != null) {
                return if (isVbms(identity)) select(BmsConnection.Can(preferredCanId)) else unavailable()
            }
            return probe(preferredCanId)
        }
        if (reusableInventory != null) {
            reusableInventory.filter(::isVbms).forEach { identity ->
                identity.canId?.let { discovered[BmsConnection.Can(it)] = identity }
            }
            return finish()
        }
        return AuxiliaryDiscoveryEffect.PingCan
    }

    fun acceptCanIds(rawIds: List<Int>, generation: Long): AuxiliaryDiscoveryEffect {
        if (generation != this.generation || preferredCanId != null) return AuxiliaryDiscoveryEffect.None
        val ids = rawIds.filter { it in 0..253 }.distinct().sorted()
        if (ids.size > 64) return unavailable()
        pendingCanIds = ids.toMutableList()
        return nextProbe()
    }

    fun acceptFirmware(value: FirmwareRecord, generation: Long): AuxiliaryDiscoveryEffect {
        val canId = value.canId
        if (generation != this.generation || canId == null || activeProbeId != canId) {
            return AuxiliaryDiscoveryEffect.None
        }
        activeProbeId = null
        preferredCanId?.let { preferred ->
            return if (preferred == canId && isVbms(value)) select(BmsConnection.Can(canId)) else unavailable()
        }
        if (isVbms(value)) discovered[BmsConnection.Can(canId)] = value
        return nextProbe()
    }

    fun probeTimedOut(canId: Int, generation: Long): AuxiliaryDiscoveryEffect {
        if (generation != this.generation || activeProbeId != canId) return AuxiliaryDiscoveryEffect.None
        activeProbeId = null
        return if (preferredCanId == null) nextProbe() else unavailable()
    }

    fun markUnavailable() {
        state = BmsState(BmsStatus.UNAVAILABLE, selectedConnection, state.candidates, state.latestSnapshot)
    }

    fun acceptSnapshot(snapshot: BmsSnapshot) {
        val connection = selectedConnection ?: return
        state = BmsState(BmsStatus.ATTACHED, connection, latestSnapshot = snapshot)
    }

    fun reset() {
        generation++
        state = BmsState()
        selectedConnection = null
        activeProbeId = null
        pendingCanIds.clear()
        discovered.clear()
        preferredCanId = null
    }

    private fun probe(canId: Int): AuxiliaryDiscoveryEffect {
        activeProbeId = canId
        return AuxiliaryDiscoveryEffect.ProbeFirmware(ControllerRoute.Can(canId))
    }

    private fun nextProbe(): AuxiliaryDiscoveryEffect =
        if (pendingCanIds.isEmpty()) finish() else probe(pendingCanIds.removeAt(0))

    private fun finish(): AuxiliaryDiscoveryEffect {
        val entries = discovered.entries.sortedBy { sortKey(it.key) }
        val candidates = entries.take(16).map { (connection, value) ->
            BmsCandidate(
                connection,
                ControllerDiscoveryPolicy.boundedDisplayValue(value.hardwareName),
                ControllerDiscoveryPolicy.version(value),
            )
        }
        if (entries.size > 16) {
            state = BmsState(BmsStatus.AMBIGUOUS, candidates = candidates)
            return AuxiliaryDiscoveryEffect.None
        }
        if (entries.size != 1) {
            state = BmsState(
                if (entries.isEmpty()) BmsStatus.NOT_FOUND else BmsStatus.AMBIGUOUS,
                candidates = candidates,
            )
            return AuxiliaryDiscoveryEffect.None
        }
        return select(entries.single().key)
    }

    private fun select(connection: BmsConnection): AuxiliaryDiscoveryEffect {
        selectedConnection = connection
        state = BmsState(BmsStatus.DISCOVERING, connection)
        return AuxiliaryDiscoveryEffect.RequestValues(connection)
    }

    private fun unavailable(): AuxiliaryDiscoveryEffect {
        markUnavailable()
        return AuxiliaryDiscoveryEffect.None
    }

    private fun isVbms(value: FirmwareRecord): Boolean {
        return BmsIdentityPolicy.supports(value)
    }

    private fun sortKey(connection: BmsConnection): Int = when (connection) {
        BmsConnection.Direct -> -1
        is BmsConnection.Can -> connection.canId
    }
}
