package fail.acheron.floatview.speech

import android.speech.tts.TextToSpeech
import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertTrue
import org.junit.Test

/** Observes the real installed engine without speaking or changing its settings. */
class SpeechEngineInstrumentedTest {
    @Test fun installedEngineCanBeInspectedAndReleased() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val initialized = CountDownLatch(1)
        var engine: TextToSpeech? = null
        var status = TextToSpeech.ERROR
        instrumentation.runOnMainSync {
            engine = TextToSpeech(instrumentation.targetContext) {
                status = it
                initialized.countDown()
            }
        }
        try {
            assertTrue("TTS initialization did not terminate", initialized.await(12, TimeUnit.SECONDS))
            val voices = if (status == TextToSpeech.SUCCESS) engine?.voices.orEmpty() else emptySet()
            Log.i("FloatViewSpeechProbe", "status=$status voices=${voices.size}")
            voices.take(128).forEach {
                Log.i("FloatViewSpeechProbe", "locale=${it.locale.toLanguageTag()} language=${it.locale.language} network=${it.isNetworkConnectionRequired} features=${it.features}")
            }
        } finally {
            instrumentation.runOnMainSync { engine?.shutdown() }
        }
    }
}
