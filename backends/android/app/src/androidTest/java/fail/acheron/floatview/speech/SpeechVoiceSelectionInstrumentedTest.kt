package fail.acheron.floatview.speech

import android.speech.tts.TextToSpeech
import android.speech.tts.Voice
import java.util.Locale
import org.junit.Assert.*
import org.junit.Test

class SpeechVoiceSelectionInstrumentedTest {
    private fun voice(name: String, locale: Locale = Locale.US, network: Boolean = false, missing: Boolean = false) =
        Voice(name, locale, Voice.QUALITY_NORMAL, Voice.LATENCY_NORMAL, network,
            if (missing) setOf(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED) else emptySet())

    @Test fun standardAndroidAndSamsungLocalesHonorPreferredVoice() {
        val fallback = voice("a-fallback")
        val standard = voice("z-preferred")
        val samsung = voice("samsung", Locale("eng"))
        assertEquals(standard, chooseOfflineEnglishVoice(setOf(fallback, standard, samsung), standard, Locale.US))
        assertEquals(samsung, chooseOfflineEnglishVoice(setOf(fallback, standard, samsung), samsung, Locale.US))
    }

    @Test fun missingNetworkAndNonEnglishPreferencesCannotDisplaceOfflineFallback() {
        val offline = voice("offline")
        for (invalid in listOf(voice("network", network = true), voice("missing", missing = true), voice("french", Locale.FRENCH))) {
            assertEquals(offline, chooseOfflineEnglishVoice(setOf(invalid, offline), invalid, Locale.US))
            assertNull(chooseOfflineEnglishVoice(setOf(invalid), invalid, Locale.US))
        }
    }

    @Test fun fallbackOrderIsStableAndPrefersDeviceLocale() {
        val au = voice("a-au", Locale("en", "AU"))
        val us = voice("z-us")
        assertEquals(us, chooseOfflineEnglishVoice(setOf(au, us), null, Locale.US))
        assertEquals(au, chooseOfflineEnglishVoice(setOf(us, au), null, Locale.UK))
    }
}
