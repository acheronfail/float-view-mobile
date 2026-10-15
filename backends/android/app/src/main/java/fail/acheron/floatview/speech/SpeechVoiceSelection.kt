package fail.acheron.floatview.speech

import android.speech.tts.TextToSpeech
import android.speech.tts.Voice
import java.util.Locale

/** Preserve the engine's selected voice when eligible, with a deterministic offline fallback. */
internal fun chooseOfflineEnglishVoice(voices: Set<Voice>, preferred: Voice?, locale: Locale): Voice? {
    val eligible = voices.filter {
        !it.isNetworkConnectionRequired && isEnglishSpeechLocale(it.locale) &&
            TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED !in it.features.orEmpty()
    }
    return eligible.firstOrNull { it.name == preferred?.name }
        ?: eligible.sortedWith(compareByDescending<Voice> { it.locale == locale }.thenBy { it.name }).firstOrNull()
}
