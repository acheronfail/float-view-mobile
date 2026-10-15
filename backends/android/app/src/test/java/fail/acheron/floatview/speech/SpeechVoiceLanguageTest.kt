package fail.acheron.floatview.speech

import java.util.Locale
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SpeechVoiceLanguageTest {
    @Test fun englishVoicesAcceptBothStandardLanguageCodeForms() {
        assertTrue(isEnglishSpeechLocale(Locale.US))
        assertTrue(isEnglishSpeechLocale(Locale.UK))
        assertTrue(isEnglishSpeechLocale(Locale("en", "AU")))
        // Observed on Samsung TTS: offline English voices use ISO 639-2.
        assertTrue(isEnglishSpeechLocale(Locale("eng")))
    }

    @Test fun otherOrUnspecifiedLanguagesDoNotQualify() {
        for (locale in listOf(Locale.FRENCH, Locale("fra"), Locale("spa"), Locale.ROOT)) {
            assertFalse(isEnglishSpeechLocale(locale))
        }
    }
}
