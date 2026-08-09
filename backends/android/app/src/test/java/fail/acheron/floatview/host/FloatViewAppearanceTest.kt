package fail.acheron.floatview.host

import android.content.res.Configuration
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class FloatViewAppearanceTest {
    @Test
    fun frontendIsTheOnlyEffectiveAppearanceSourceAndNativeChromeMapsTheSameMode() {
        assertEquals("frontend-prefers-color-scheme", FloatViewAppearance.source)
        assertTrue(FloatViewAppearance.isDarkMode(Configuration.UI_MODE_NIGHT_YES))
        assertFalse(FloatViewAppearance.isDarkMode(Configuration.UI_MODE_NIGHT_NO))
        assertFalse(FloatViewAppearance.isDarkMode(Configuration.UI_MODE_NIGHT_UNDEFINED))
        assertEquals("#020617", FloatViewAppearance.backgroundColorHex(true))
        assertEquals("#F8FAFC", FloatViewAppearance.backgroundColorHex(false))
    }
}
