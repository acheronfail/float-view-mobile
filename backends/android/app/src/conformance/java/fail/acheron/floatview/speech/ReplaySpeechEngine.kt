package fail.acheron.floatview.speech

import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject

/** Software-host playback boundary; conformance APK only, never physical audio evidence. */
internal fun replaySpeechEngine(): SpeechEngine = object : SpeechEngine {
    private val handler = Handler(Looper.getMainLooper())
    private var epoch = 0L
    override fun initialize(done: (Boolean) -> Unit) = done(true)
    override fun selectVoice(): String? = null
    override fun speak(request: SpeechRequest, done: (Long, String) -> Unit): Boolean {
        val operation = ++epoch
        if (request.battery != null) {
            Log.i("FloatViewSpeechReplay", "FLOAT_VIEW_SPEECH_REPLAY " + JSONObject().put("phrase", request.phrase).toString())
        }
        handler.post { if (operation == epoch) done(request.id, "finished") }
        return true
    }
    override fun stop() { epoch++ }
    override fun shutdown() { stop() }
}
