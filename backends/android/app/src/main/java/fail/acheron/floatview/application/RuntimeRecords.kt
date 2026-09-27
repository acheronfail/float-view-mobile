package fail.acheron.floatview.application

data class SamplingSettings(
    val idlePersistIntervalMs: Long = 1_000,
    val ridingPersistIntervalMs: Long = 100,
    val postRideTailMs: Long = 5_000,
    val reconnectDurationMs: Long = 300_000,
    val idleProbeIntervalMs: Long = 500,
)

sealed interface ControllerRoute {
    data object Direct : ControllerRoute
    data class Can(val canId: Int) : ControllerRoute {
        init { require(canId in 0..253) }
    }
}

enum class RefloatBooleanControl(val wireValue: String) {
    PACKAGE("package"), FLYWHEEL("flywheel"), LEDS("leds"), HEADLIGHTS("headlights")
}

enum class ControllerPendingAction(val wireValue: String) { SHUTDOWN("shutdown") }

data class ControllerActions(
    val shutdownAvailable: Boolean = false,
    val pendingAction: ControllerPendingAction? = null,
)

class RefloatMutationException(val unavailable: Boolean = false) : Exception()

object RefloatConfigurationLimits {
    const val COMPRESSED_BYTES = 32 * 1024
    const val EXPANDED_BYTES = 384 * 1024
    const val FIELDS = 512
    const val GROUPS = 16
    const val SUBGROUPS = 64
    const val ENUM_CHOICES = 128
    const val NAME_BYTES = 128
    const val DESCRIPTION_BYTES = 4096
    const val STRING_VALUE_BYTES = 4096
    const val SCHEMA_RECORD_BYTES = 1024 * 1024
    const val VALUE_RECORD_BYTES = 3 * 1024 * 1024
}

enum class RefloatConfigurationFieldType(val wireValue: Int) {
    DOUBLE(1), INTEGER(2), STRING(3), ENUMERATION(4), BOOLEAN(5), BITFIELD(6)
}

enum class RefloatConfigurationWireType(val wireValue: Int) {
    UINT8(1), INT8(2), UINT16(3), INT16(4), UINT32(5), INT32(6),
    DOUBLE16(7), DOUBLE32(8), DOUBLE32_AUTO(9)
}

sealed interface RefloatConfigurationValue {
    data class Decimal(val value: Double) : RefloatConfigurationValue
    data class Integer(val value: Long) : RefloatConfigurationValue
    data class BooleanValue(val value: Boolean) : RefloatConfigurationValue
    data class StringValue(val value: String) : RefloatConfigurationValue
}

data class RefloatConfigurationField(
    val name: String,
    val longName: String,
    val type: RefloatConfigurationFieldType,
    val wireType: RefloatConfigurationWireType?,
    val defaultValue: RefloatConfigurationValue,
    val minimum: Double?,
    val maximum: Double?,
    val step: Double?,
    val precision: Int?,
    val scale: Double?,
    val suffix: String,
    val choices: List<String>,
    val description: String,
)

data class RefloatConfigurationSubgroup(val name: String, val fields: List<String>)
data class RefloatConfigurationGroup(val name: String, val subgroups: List<RefloatConfigurationSubgroup>)
data class RefloatConfigurationSchema(
    val fingerprint: String,
    val wireSignature: Long,
    val fields: List<RefloatConfigurationField>,
    val groups: List<RefloatConfigurationGroup>,
)
data class RefloatConfigurationValues(
    val revision: Long,
    val source: String,
    val values: Map<String, RefloatConfigurationValue>,
)
enum class RefloatConfigurationOperation { SCHEMA, READ_CURRENT, READ_DEFAULT, WRITE, APPLY_TUNE }
enum class RefloatConfigurationAvailability { LOADING, READY, UNAVAILABLE }

data class RefloatSchemaCacheIdentity(
    val firmwareVersion: String,
    val hardwareName: String,
    val packageVersion: String,
    val customIndex: Int,
    val hardwareConfigurationCrc: Long,
)

interface RefloatSchemaCachePort {
    fun cachedRefloatSchema(identity: RefloatSchemaCacheIdentity): ByteArray? = null
    fun storeRefloatSchema(data: ByteArray, identity: RefloatSchemaCacheIdentity) = Unit
    fun removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity) = Unit
}

data class RefloatConfigurationSummary(
    val availability: RefloatConfigurationAvailability,
    val writable: Boolean,
    val packageVersion: String?,
    val schemaFingerprint: String?,
    val wireSignature: Long?,
    val currentLoaded: Boolean,
    val defaultLoaded: Boolean,
    val pendingOperation: RefloatConfigurationOperation?,
)

enum class RefloatConfigurationServiceError {
    UNAVAILABLE, INVALID_STATE, STALE_RESPONSE, UNAUDITED_SCHEMA, MALFORMED_RESPONSE, CONFLICT
}

class RefloatConfigurationServiceException(
    val reason: RefloatConfigurationServiceError,
) : Exception(reason.name)
