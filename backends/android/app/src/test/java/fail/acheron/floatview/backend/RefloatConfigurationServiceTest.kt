package fail.acheron.floatview.backend

import fail.acheron.floatview.protocol.refloat.RefloatConfigurationAvailability
import fail.acheron.floatview.protocol.refloat.RefloatConfigurationSchemaCodec
import fail.acheron.floatview.protocol.vesc.ControllerRoute
import fail.acheron.floatview.protocol.vesc.VescCommand
import fail.acheron.floatview.protocol.vesc.VescCustomConfigurationResponse
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class RefloatConfigurationServiceTest {
    private val root = File(System.getProperty("floatViewFixtureRoot"), "refloat-configuration")

    @Test
    fun assemblesAuditedSchemaThenLoadsCurrentValues() {
        val compressed = File(root, "schemas/1.2.3.qcompress").readBytes()
        val current = File(root, "values/1.2.3-current.bin").readBytes()
        val service = RefloatConfigurationService()
        val generation = VescRequestGeneration(9)
        val firmware = FirmwareRecord(
            6, 6, "THOR300", null,
            customConfigurationCount = 1, hardwareConfigurationCrc = 123,
        )
        var effect = service.begin(ControllerRoute.Can(42), generation, firmware, "1.2.3", true, false)
        while (effect is RefloatConfigurationEffect.SchemaChunk) {
            val request = effect
            val end = minOf(request.offset + request.length, compressed.size)
            effect = service.accept(
                VescCustomConfigurationResponse.SchemaChunk(
                    0, compressed.size, request.offset, compressed.copyOfRange(request.offset, end),
                ),
                VescRequestContext.RefloatConfigurationSchema(
                    request.route.toRequestRoute(), 0, request.offset, request.operation,
                ),
                generation,
            )
        }
        val read = effect as RefloatConfigurationEffect.Values
        assertEquals(ControllerRoute.Can(42), read.route)
        assertFalse(read.defaults)
        service.accept(
            VescCustomConfigurationResponse.Values(VescCommand.GET_CUSTOM_CONFIGURATION, 0, current),
            VescRequestContext.RefloatConfigurationValues(
                VescRequestRoute.Can(42), 0, false, read.operation,
            ),
            generation,
        )
        val summary = service.summary()
        assertEquals(RefloatConfigurationAvailability.READY, summary.availability)
        assertEquals("1.2.3", summary.packageVersion)
        assertTrue(summary.currentLoaded)
        assertTrue(summary.writable)
        assertEquals(167, service.loadedValues(false)?.values?.size)

        val writeBytes = File(root, "values/1.2.3-write.bin").readBytes()
        val readbackBytes = File(root, "values/1.2.3-readback.bin").readBytes()
        val schema = requireNotNull(service.loadedSchema())
        val draft = fail.acheron.floatview.protocol.refloat.RefloatConfigurationValueCodec.decode(writeBytes, schema)
        val baseRevision = requireNotNull(service.loadedValues(false)).revision
        listOf(
            arrayOf("1.2.2", schema.fingerprint, schema.wireSignature, baseRevision),
            arrayOf("1.2.3", "0" + schema.fingerprint.drop(1), schema.wireSignature, baseRevision),
            arrayOf("1.2.3", schema.fingerprint, schema.wireSignature + 1L, baseRevision),
            arrayOf("1.2.3", schema.fingerprint, schema.wireSignature, baseRevision + 1),
        ).forEach { stale ->
            assertEquals(
                RefloatConfigurationServiceError.INVALID_STATE,
                assertThrows(RefloatConfigurationServiceException::class.java) {
                    service.requestWrite(
                        stale[0] as String, stale[1] as String, stale[2] as Long,
                        stale[3] as Long, draft,
                    )
                }.reason,
            )
        }
        val write = service.requestWrite(
            "1.2.3", schema.fingerprint, schema.wireSignature, baseRevision, draft,
        ) as RefloatConfigurationEffect.Write
        assertEquals(ControllerRoute.Can(42), write.route)
        assertTrue(write.payload.contentEquals(writeBytes))
        val verification = service.accept(
            VescCustomConfigurationResponse.WriteAcknowledgement,
            VescRequestContext.RefloatConfigurationWrite(
                VescRequestRoute.Can(42), 0, write.operation,
            ),
            generation,
        )
        assertEquals(RefloatConfigurationEffect.Verification(ControllerRoute.Can(42), write.operation), verification)
        service.accept(
            VescCustomConfigurationResponse.Values(VescCommand.GET_CUSTOM_CONFIGURATION, 0, readbackBytes),
            VescRequestContext.RefloatConfigurationVerification(
                VescRequestRoute.Can(42), 0, write.operation,
            ),
            generation,
        )
        assertEquals(baseRevision + 1, service.loadedValues(false)?.revision)
        val mismatchBase = requireNotNull(service.loadedValues(false)).revision
        val mismatchWrite = service.requestWrite(
            "1.2.3", schema.fingerprint, schema.wireSignature, mismatchBase, draft,
        ) as RefloatConfigurationEffect.Write
        service.accept(
            VescCustomConfigurationResponse.WriteAcknowledgement,
            VescRequestContext.RefloatConfigurationWrite(
                VescRequestRoute.Can(42), 0, mismatchWrite.operation,
            ),
            generation,
        )
        assertEquals(
            RefloatConfigurationServiceError.CONFLICT,
            assertThrows(RefloatConfigurationServiceException::class.java) {
                service.accept(
                    VescCustomConfigurationResponse.Values(VescCommand.GET_CUSTOM_CONFIGURATION, 0, current),
                    VescRequestContext.RefloatConfigurationVerification(
                        VescRequestRoute.Can(42), 0, mismatchWrite.operation,
                    ),
                    generation,
                )
            }.reason,
        )
        assertEquals(mismatchBase + 1, service.loadedValues(false)?.revision)

        service.setFlywheel(true)
        assertFalse(service.summary().currentLoaded)
        assertFalse(service.summary().writable)
        assertEquals(
            RefloatConfigurationServiceError.INVALID_STATE,
            assertThrows(RefloatConfigurationServiceException::class.java) {
                service.requestValues(false)
            }.reason,
        )
        val postFlywheel = service.setFlywheel(false) as RefloatConfigurationEffect.Values
        assertFalse(postFlywheel.defaults)
        assertFalse(service.summary().currentLoaded)
        service.requestFailed(
            VescRequestContext.RefloatConfigurationValues(
                VescRequestRoute.Can(42), 0, false, postFlywheel.operation,
            ),
            generation,
        )
        assertEquals(null, service.summary().pendingOperation)

        val replacement = service.begin(
            ControllerRoute.Direct, VescRequestGeneration(10), firmware, "1.2.3", false, false,
        ) as RefloatConfigurationEffect.SchemaChunk
        assertEquals(RefloatConfigurationAvailability.LOADING, service.summary().availability)
        assertEquals(
            RefloatConfigurationServiceError.STALE_RESPONSE,
            assertThrows(RefloatConfigurationServiceException::class.java) {
                service.accept(
                    VescCustomConfigurationResponse.SchemaChunk(
                        0, compressed.size, replacement.offset, compressed,
                    ),
                    VescRequestContext.RefloatConfigurationSchema(
                        VescRequestRoute.Direct, 0, replacement.offset, replacement.operation,
                    ),
                    generation,
                )
            }.reason,
        )
    }

    @Test
    fun resolvesLegacyPatchOnlyByFullFingerprint() {
        val zero = RefloatConfigurationSchemaCodec.decodeQtCompressed(File(root, "schemas/1.0.0.qcompress").readBytes())
        val one = RefloatConfigurationSchemaCodec.decodeQtCompressed(File(root, "schemas/1.0.1.qcompress").readBytes())
        assertEquals("1.0.0", RefloatConfigurationCatalog.resolve("1.0", zero)?.version)
        assertEquals("1.0.1", RefloatConfigurationCatalog.resolve("1.0", one)?.version)
        assertEquals(zero.wireSignature, one.wireSignature)
    }

    @Test
    fun usesOnlyValidatedExactSchemaCacheEntries() {
        val compressed = File(root, "schemas/1.2.3.qcompress").readBytes()
        val identity = RefloatSchemaCacheIdentity("6.6", "THOR300", "1.2.3", 0, 123)
        val firmware = FirmwareRecord(
            6, 6, "THOR300", null,
            customConfigurationCount = 1, hardwareConfigurationCrc = 123,
        )
        val values = mutableMapOf(identity to compressed)
        var removals = 0
        val cache = object : RefloatSchemaCachePort {
            override fun cachedRefloatSchema(identity: RefloatSchemaCacheIdentity) = values[identity]
            override fun storeRefloatSchema(data: ByteArray, identity: RefloatSchemaCacheIdentity) { values[identity] = data }
            override fun removeCachedRefloatSchema(identity: RefloatSchemaCacheIdentity) {
                values.remove(identity); removals += 1
            }
        }
        val service = RefloatConfigurationService(cache)
        assertEquals(
            RefloatConfigurationEffect.Values(ControllerRoute.Direct, false, 1),
            service.begin(ControllerRoute.Direct, VescRequestGeneration(1), firmware, "1.2.3", true, false),
        )
        assertEquals(RefloatConfigurationAvailability.READY, service.summary().availability)

        values[identity] = "corrupt".toByteArray()
        assertTrue(
            RefloatConfigurationService(cache).begin(
                ControllerRoute.Direct, VescRequestGeneration(2), firmware, "1.2.3", true, false,
            ) is RefloatConfigurationEffect.SchemaChunk,
        )
        assertEquals(1, removals)
    }
}
