package fail.acheron.floatview.bluetooth

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Test

class AndroidDeviceIdentityTest {
    @Test
    fun identityIsStableCaseInsensitiveAndDoesNotExposeThePlatformAddress() {
        val registry = AndroidDeviceIdentity("installation-a")
        val first = registry.idFor("AA:BB:CC:DD:EE:FF")
        val repeated = registry.idFor("aa:bb:cc:dd:ee:ff")

        assertEquals(first, repeated)
        assertFalse(first.value.contains("AA:BB:CC:DD:EE:FF"))
    }

    @Test
    fun identityIsScopedToTheInstallationAndBoard() {
        val first = AndroidDeviceIdentity("installation-a")
        val second = AndroidDeviceIdentity("installation-b")

        assertNotEquals(first.idFor("AA:BB:CC:DD:EE:FF"), first.idFor("11:22:33:44:55:66"))
        assertNotEquals(first.idFor("AA:BB:CC:DD:EE:FF"), second.idFor("AA:BB:CC:DD:EE:FF"))
    }
}
