package fail.acheron.floatview.application

import fail.acheron.floatview.protocol.refloat.RefloatConfigurationAvailability
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationLimits
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationOperation
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationSchema
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationSchemaCodec
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationSchemaIdentity
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationValueCodec
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationValues
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommand
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationCodec
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationResponse

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

enum class RefloatConfigurationServiceError { UNAVAILABLE, INVALID_STATE, STALE_RESPONSE, UNAUDITED_SCHEMA, MALFORMED_RESPONSE, CONFLICT }
class RefloatConfigurationServiceException(val reason: RefloatConfigurationServiceError) : Exception(reason.name)

sealed interface RefloatConfigurationEffect {
    data object None : RefloatConfigurationEffect
    data class SchemaChunk(val route: ControllerRoute, val length: Int, val offset: Int, val operation: Long) : RefloatConfigurationEffect
    data class Values(val route: ControllerRoute, val defaults: Boolean, val operation: Long) : RefloatConfigurationEffect
    data class Write(val route: ControllerRoute, val payload: ByteArray, val operation: Long) : RefloatConfigurationEffect
    data class Verification(val route: ControllerRoute, val operation: Long) : RefloatConfigurationEffect
}

data class RefloatConfigurationCatalogEntry(val version: String, val fingerprint: String, val wireSignature: Long)

object RefloatConfigurationCatalog {
    val entries = listOf(
        RefloatConfigurationCatalogEntry("1.0.0", "324e7f5ef1ea1b2038478af7e6f43c91d63ec06a6caf2907c5b7e6c812382ad9", 4_090_797_430),
        RefloatConfigurationCatalogEntry("1.0.1", "887e7b03c03304a2f8455e71b369b89830e8d6446a73aa598ab067ac81944526", 4_090_797_430),
        RefloatConfigurationCatalogEntry("1.1.0", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 15_843_867),
        RefloatConfigurationCatalogEntry("1.1.1", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 15_843_867),
        RefloatConfigurationCatalogEntry("1.1.2", "b5849e38371a65721bb48182048023d1c707012228554878ce263b1c49ee168f", 15_843_867),
        RefloatConfigurationCatalogEntry("1.2.0", "2561e00a5b6694de0ac272bae077e06050cfc00cf4ba371bb1d5d9616e137f7e", 2_427_955_642),
        RefloatConfigurationCatalogEntry("1.2.1", "9b8979a36759810a7fa9c19d01534ed52ec9ed8ac4a2d3ab3aefc594ba3b93cc", 2_427_955_642),
        RefloatConfigurationCatalogEntry("1.2.2", "d7ca6a2c06ce5701858eebfaaf224e97e08a212d1dab60b664d268a90763eb39", 2_427_955_642),
        RefloatConfigurationCatalogEntry("1.2.3", "fa40717969d1d86b429d7c78b59144b8387b4850be83a72fd3c7cf15ffecd01c", 2_427_955_642),
    )

    fun resolve(reportedVersion: String, schema: RefloatConfigurationSchema) = entries.firstOrNull {
        (it.version == reportedVersion || reportedVersion == "1.0" && it.version.startsWith("1.0.")) &&
            it.fingerprint == schema.fingerprint && it.wireSignature == schema.wireSignature
    }
}

/** Connection-scoped bounded lifecycle owner; ApplicationService remains the scheduler submitter. */
class RefloatConfigurationService(private val cache: RefloatSchemaCachePort? = null) {
    private var route: ControllerRoute? = null
    private var generation = VescRequestGeneration(0)
    private var reportedVersion: String? = null
    private var canWrite = false
    private var flywheel = false
    private var nextOperation = 0L
    private var operation: Long? = null
    private var assembly = ByteArray(0)
    private var totalLength: Int? = null
    private var requestedOffset = 0
    private var schema: RefloatConfigurationSchema? = null
    private var identity: RefloatConfigurationSchemaIdentity? = null
    private var current: RefloatConfigurationValues? = null
    private var defaults: RefloatConfigurationValues? = null
    private var expectedReadback: Map<String, fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue>? = null
    private var revision = 0L
    private var availability = RefloatConfigurationAvailability.UNAVAILABLE
    private var pending: RefloatConfigurationOperation? = null
    private var cacheIdentity: RefloatSchemaCacheIdentity? = null

    @Synchronized
    fun begin(
        route: ControllerRoute,
        generation: VescRequestGeneration,
        firmware: FirmwareRecord,
        reportedVersion: String,
        writable: Boolean,
        flywheel: Boolean,
    ): RefloatConfigurationEffect {
        resetLocked()
        this.route = route
        this.generation = generation
        this.reportedVersion = reportedVersion
        canWrite = writable
        this.flywheel = flywheel
        if (firmware.customConfigurationCount <= INDEX) return RefloatConfigurationEffect.None
        availability = RefloatConfigurationAvailability.LOADING
        firmware.hardwareConfigurationCrc?.takeIf { it != 0L }?.let { crc ->
            val cacheIdentity = RefloatSchemaCacheIdentity(
                "${firmware.major}.${firmware.minor}", firmware.hardwareName ?: "",
                reportedVersion, INDEX, crc,
            )
            this.cacheIdentity = cacheIdentity
            cache?.cachedRefloatSchema(cacheIdentity)?.let { cached ->
                val decoded = runCatching {
                    RefloatConfigurationSchemaCodec.decodeQtCompressed(cached)
                }.getOrNull()
                val entry = decoded?.let { RefloatConfigurationCatalog.resolve(reportedVersion, it) }
                if (decoded != null && entry != null) {
                    schema = decoded
                    identity = RefloatConfigurationSchemaIdentity(
                        "refloat", entry.version, entry.fingerprint, entry.wireSignature,
                    )
                    availability = RefloatConfigurationAvailability.READY
                    if (flywheel) return RefloatConfigurationEffect.None
                    pending = RefloatConfigurationOperation.READ_CURRENT
                    return RefloatConfigurationEffect.Values(route, false, startOperationLocked())
                }
                cache.removeCachedRefloatSchema(cacheIdentity)
            }
        }
        pending = RefloatConfigurationOperation.SCHEMA
        return RefloatConfigurationEffect.SchemaChunk(
            route, VescCustomConfigurationCodec.MAXIMUM_CHUNK_BYTES, 0, startOperationLocked(),
        )
    }

    @Synchronized fun reset() = resetLocked()

    @Synchronized
    fun summary() = RefloatConfigurationSummary(
        availability,
        availability == RefloatConfigurationAvailability.READY && canWrite && !flywheel && current != null,
        identity?.packageVersion,
        identity?.fingerprint,
        identity?.wireSignature,
        current != null && !flywheel,
        defaults != null,
        pending,
    )

    @Synchronized fun loadedSchema() = schema
    @Synchronized fun loadedValues(defaults: Boolean) = if (defaults) this.defaults else current

    @Synchronized
    fun setFlywheel(active: Boolean): RefloatConfigurationEffect {
        val wasActive = flywheel
        flywheel = active
        if (active) {
            current = null
            if (pending == RefloatConfigurationOperation.READ_CURRENT) {
                pending = null
                operation = null
            }
            return RefloatConfigurationEffect.None
        }
        val route = route
        if (!wasActive || schema == null || pending != null || route == null) return RefloatConfigurationEffect.None
        pending = RefloatConfigurationOperation.READ_CURRENT
        return RefloatConfigurationEffect.Values(route, false, startOperationLocked())
    }

    @Synchronized fun invalidateCurrent() { current = null }

    @Synchronized
    fun requestValues(defaults: Boolean): RefloatConfigurationEffect {
        val route = route ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
        if (schema == null || pending != null || (!defaults && flywheel)) fail(RefloatConfigurationServiceError.INVALID_STATE)
        pending = if (defaults) RefloatConfigurationOperation.READ_DEFAULT else RefloatConfigurationOperation.READ_CURRENT
        return RefloatConfigurationEffect.Values(route, defaults, startOperationLocked())
    }

    @Synchronized
    fun requestWrite(
        packageVersion: String,
        fingerprint: String,
        wireSignature: Long,
        baseRevision: Long,
        values: Map<String, fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue>,
    ): RefloatConfigurationEffect {
        val route = route ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
        val schema = schema ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
        val identity = identity ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
        val current = current ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
        if (
            availability != RefloatConfigurationAvailability.READY || !canWrite || flywheel || pending != null ||
            identity.packageVersion != packageVersion || identity.fingerprint != fingerprint ||
            identity.wireSignature != wireSignature || current.revision != baseRevision
        ) fail(RefloatConfigurationServiceError.INVALID_STATE)
        val payload = RefloatConfigurationValueCodec.encode(values, schema)
        expectedReadback = normalized(values, identity.packageVersion)
        pending = RefloatConfigurationOperation.WRITE
        return RefloatConfigurationEffect.Write(route, payload, startOperationLocked())
    }

    @Synchronized
    fun accept(
        response: VescCustomConfigurationResponse,
        context: VescRequestContext,
        responseGeneration: VescRequestGeneration,
    ): RefloatConfigurationEffect {
        if (responseGeneration != generation) fail(RefloatConfigurationServiceError.STALE_RESPONSE)
        val route = route ?: fail(RefloatConfigurationServiceError.STALE_RESPONSE)
        val operation = operation ?: fail(RefloatConfigurationServiceError.STALE_RESPONSE)
        when {
            response is VescCustomConfigurationResponse.SchemaChunk && context is VescRequestContext.RefloatConfigurationSchema -> {
                if (
                    pending != RefloatConfigurationOperation.SCHEMA || context.route != route.toRequestRoute() ||
                    response.index != context.index || response.index != INDEX || response.offset != context.offset ||
                    response.offset != requestedOffset || context.operation != operation || response.bytes.isEmpty() ||
                    response.totalLength > RefloatConfigurationLimits.COMPRESSED_BYTES ||
                    (totalLength != null && totalLength != response.totalLength)
                ) fail(RefloatConfigurationServiceError.STALE_RESPONSE)
                totalLength = response.totalLength
                assembly += response.bytes
                if (assembly.size > response.totalLength) fail(RefloatConfigurationServiceError.MALFORMED_RESPONSE)
                requestedOffset = assembly.size
                if (assembly.size < response.totalLength) {
                    return RefloatConfigurationEffect.SchemaChunk(
                        route,
                        minOf(VescCustomConfigurationCodec.MAXIMUM_CHUNK_BYTES, response.totalLength - assembly.size),
                        assembly.size,
                        operation,
                    )
                }
                val decoded = RefloatConfigurationSchemaCodec.decodeQtCompressed(assembly)
                val entry = reportedVersion?.let { RefloatConfigurationCatalog.resolve(it, decoded) }
                    ?: run {
                        failLocked()
                        fail(RefloatConfigurationServiceError.UNAUDITED_SCHEMA)
                    }
                schema = decoded
                identity = RefloatConfigurationSchemaIdentity("refloat", entry.version, entry.fingerprint, entry.wireSignature)
                pending = null
                this.operation = null
                availability = RefloatConfigurationAvailability.READY
                cacheIdentity?.let { cache?.storeRefloatSchema(assembly.copyOf(), it) }
                if (flywheel) return RefloatConfigurationEffect.None
                pending = RefloatConfigurationOperation.READ_CURRENT
                return RefloatConfigurationEffect.Values(route, false, startOperationLocked())
            }
            response is VescCustomConfigurationResponse.Values && context is VescRequestContext.RefloatConfigurationValues -> {
                val expectedPending = if (context.defaults) RefloatConfigurationOperation.READ_DEFAULT else RefloatConfigurationOperation.READ_CURRENT
                val expectedCommand = if (context.defaults) VescCommand.GET_CUSTOM_CONFIGURATION_DEFAULT else VescCommand.GET_CUSTOM_CONFIGURATION
                val schema = schema ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
                if (
                    context.route != route.toRequestRoute() || response.index != context.index || response.index != INDEX ||
                    context.operation != operation || response.command != expectedCommand || pending != expectedPending
                ) fail(RefloatConfigurationServiceError.STALE_RESPONSE)
                if (!context.defaults && flywheel) fail(RefloatConfigurationServiceError.INVALID_STATE)
                val decoded = RefloatConfigurationValueCodec.decode(response.bytes, schema)
                revision += 1
                val values = RefloatConfigurationValues(revision, if (context.defaults) "default" else "current", decoded)
                if (context.defaults) defaults = values else current = values
                pending = null
                this.operation = null
                return RefloatConfigurationEffect.None
            }
            response is VescCustomConfigurationResponse.WriteAcknowledgement && context is VescRequestContext.RefloatConfigurationWrite -> {
                if (
                    context.route != route.toRequestRoute() || context.index != INDEX ||
                    context.operation != operation || pending != RefloatConfigurationOperation.WRITE ||
                    expectedReadback == null
                ) fail(RefloatConfigurationServiceError.STALE_RESPONSE)
                return RefloatConfigurationEffect.Verification(route, operation)
            }
            response is VescCustomConfigurationResponse.Values && context is VescRequestContext.RefloatConfigurationVerification -> {
                val schema = schema ?: fail(RefloatConfigurationServiceError.INVALID_STATE)
                val expected = expectedReadback ?: fail(RefloatConfigurationServiceError.STALE_RESPONSE)
                if (
                    context.route != route.toRequestRoute() || response.index != context.index || response.index != INDEX ||
                    context.operation != operation || pending != RefloatConfigurationOperation.WRITE ||
                    response.command != VescCommand.GET_CUSTOM_CONFIGURATION || flywheel
                ) fail(RefloatConfigurationServiceError.STALE_RESPONSE)
                val decoded = RefloatConfigurationValueCodec.decode(response.bytes, schema)
                revision += 1
                current = RefloatConfigurationValues(revision, "current", decoded)
                pending = null
                this.operation = null
                expectedReadback = null
                if (decoded != expected) fail(RefloatConfigurationServiceError.CONFLICT)
                return RefloatConfigurationEffect.None
            }
            else -> fail(RefloatConfigurationServiceError.STALE_RESPONSE)
        }
    }

    @Synchronized
    fun requestFailed(context: VescRequestContext, responseGeneration: VescRequestGeneration) {
        if (responseGeneration != generation) return
        val expected = when (context) {
            is VescRequestContext.RefloatConfigurationSchema -> context.operation
            is VescRequestContext.RefloatConfigurationValues -> context.operation
            is VescRequestContext.RefloatConfigurationWrite -> context.operation
            is VescRequestContext.RefloatConfigurationVerification -> context.operation
            else -> null
        }
        if (expected == operation) failLocked()
    }

    @Synchronized
    fun abortActiveOperation(responseGeneration: VescRequestGeneration) {
        if (responseGeneration == generation && pending != null) failLocked()
    }

    private fun startOperationLocked(): Long {
        nextOperation += 1
        operation = nextOperation
        return nextOperation
    }

    private fun failLocked() {
        pending = null
        cacheIdentity = null
        operation = null
        current = null
        expectedReadback = null
        availability = if (schema == null) RefloatConfigurationAvailability.UNAVAILABLE else RefloatConfigurationAvailability.READY
    }

    private fun resetLocked() {
        route = null
        reportedVersion = null
        canWrite = false
        flywheel = false
        operation = null
        assembly = ByteArray(0)
        totalLength = null
        requestedOffset = 0
        schema = null
        identity = null
        current = null
        defaults = null
        expectedReadback = null
        revision = 0
        availability = RefloatConfigurationAvailability.UNAVAILABLE
        pending = null
        cacheIdentity = null
    }

    private fun fail(reason: RefloatConfigurationServiceError): Nothing = throw RefloatConfigurationServiceException(reason)

    private fun normalized(
        values: Map<String, fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue>,
        version: String,
    ): Map<String, fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue> {
        if (version.startsWith("1.0.")) return values.toMap()
        return values.toMutableMap().also {
            if (it.containsKey("meta.is_default")) {
                it["meta.is_default"] = fail.acheron.floatview.protocol.refloat.RefloatConfigurationValue.BooleanValue(false)
            }
        }
    }

    companion object {
        const val INDEX = 0
        const val REQUEST_TIMEOUT_MS = 3_000L
    }
}
