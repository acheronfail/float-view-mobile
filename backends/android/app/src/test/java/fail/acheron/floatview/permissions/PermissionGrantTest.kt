package fail.acheron.floatview.permissions

import org.junit.Assert.assertEquals
import org.junit.Test

class PermissionGrantTest {
    @Test fun `first prompt denial retry blocked and policy restriction are distinct`() {
        assertEquals(PermissionGrant("notDetermined", true, false), PermissionGrant.observe(true, false, false, false, false))
        assertEquals(PermissionGrant("denied", true, true), PermissionGrant.observe(true, false, true, true, false))
        assertEquals(PermissionGrant("denied", false, false), PermissionGrant.observe(true, false, true, false, false))
        assertEquals(PermissionGrant("restricted", false, false), PermissionGrant.observe(true, false, false, true, true))
    }
    @Test fun `grants are always current and history cannot fabricate access`() {
        assertEquals("granted", PermissionGrant.observe(true, true, true, false, false).access)
        assertEquals("denied", PermissionGrant.observe(true, false, true, false, false).access)
        assertEquals("notRequired", PermissionGrant.observe(false, false, false, false, false).access)
    }
}
