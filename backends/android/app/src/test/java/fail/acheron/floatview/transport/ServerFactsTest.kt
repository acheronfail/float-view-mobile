package fail.acheron.floatview.transport

import fail.acheron.floatview.application.*
import java.time.Instant
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ServerFactsTest {
    @Test fun linkedPublicationAcceptsDomainFactsAndRejectsStaleUpdates() {
        RustApplicationRuntime.ensureLoaded()
        val handle=RustApplicationRuntime.serverCreate(2,"html".toByteArray())
        assertTrue(handle>0)
        try {
            val facts=ServerFacts.objectOf(
                "discovery" to DiscoverySnapshot(BluetoothState(BluetoothRadioState.POWERED_ON,BluetoothPermissionState.GRANTED,BluetoothPermissionState.GRANTED),null,listOf(DiscoveredDevice(DeviceId("test"),"Board",-45,Instant.EPOCH))),
                "active" to null,"firmware" to null,"packageVersion" to null,
                "configuration" to RefloatConfigurationSummary(RefloatConfigurationAvailability.UNAVAILABLE,false,null,null,null,false,false,null),
                "savedBoards" to emptyList<SavedDeviceRecord>(),"settings" to SettingsRecord(),
                "location" to LocationProviderState(LocationProviderAuthorization.NOT_DETERMINED,LocationProviderBackgroundCapture.UNAVAILABLE),"maintenance" to null,
            )
            val request=ServerFacts.objectOf("version" to 1,"action" to ServerFacts.objectOf("command" to "publish","parameters" to ServerFacts.objectOf("generation" to 0,"sequence" to 1,"catalogChanged" to false,"facts" to facts))).toString().toByteArray()
            val output=ByteArray(1048576); val written=IntArray(1)
            assertEquals(0,RustApplicationRuntime.serverCommand(handle,request,output,written))
            assertTrue(written[0]>0)
            assertNotEquals(0,RustApplicationRuntime.serverCommand(handle,request,output,written))
            assertNotEquals(0,RustApplicationRuntime.serverCommand(handle,request,ByteArray(1),written))
        } finally { assertEquals(0,RustApplicationRuntime.serverDestroy(handle)); assertEquals(0,RustApplicationRuntime.serverDestroy(handle)) }
    }
    @Test fun privateFirmwareIdentityCannotEnterDomainFacts() {
        val encoded=ServerFacts.encode(FirmwareRecord(6,5,"hardware","name",controllerUuid=byteArrayOf(1,2,3))) as JSONObject
        assertEquals(setOf("major","minor","hardwareName"),encoded.keys().asSequence().toSet())
    }
}
