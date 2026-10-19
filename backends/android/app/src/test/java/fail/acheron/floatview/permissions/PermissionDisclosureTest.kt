package fail.acheron.floatview.permissions

import org.junit.Assert.*
import org.junit.Test

class PermissionDisclosureTest {
    @Test fun `first location grants require disclosure without an OS rationale`() {
        assertTrue(needsPermissionDisclosure("location", "requestPermission", false))
        assertTrue(needsPermissionDisclosure("backgroundLocation", "requestPermission", false))
    }
    @Test fun `location settings recovery retains disclosure`() {
        assertTrue(needsPermissionDisclosure("location", "openSettings", false))
        assertTrue(needsPermissionDisclosure("backgroundLocation", "openSettings", false))
    }
    @Test fun `other first prompts stay direct and denial rationale remains available`() {
        assertFalse(needsPermissionDisclosure("bluetooth", "requestPermission", false))
        assertFalse(needsPermissionDisclosure("notifications", "requestPermission", false))
        assertTrue(needsPermissionDisclosure("bluetooth", "requestPermission", true))
    }
}
