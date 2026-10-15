package fail.acheron.floatview.speech

import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import org.json.JSONObject
import java.util.Locale

/** Main-looper confined OS speech executor; no telemetry or announcement policy. */
class SpeechAdapter(private val context: Context, report: (String, Long, String?) -> Unit) {
    private val handler = Handler(Looper.getMainLooper())
    private val audio = context.getSystemService(AudioManager::class.java)
    private val attributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ASSISTANCE_NAVIGATION_GUIDANCE)
        .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build()
    private var closed = false
    private val executor = SpeechExecutor(
        createEngine = { AndroidSpeechEngine() },
        acquireFocus = { onLoss ->
            val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK)
                .setAudioAttributes(attributes).setAcceptsDelayedFocusGain(false)
                .setOnAudioFocusChangeListener({ change -> if (change < 0) onLoss() }, handler).build()
            if (audio.requestAudioFocus(request) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED)
                ({ audio.abandonAudioFocusRequest(request); Unit }) else null
        },
        now = SystemClock::elapsedRealtime,
        later = { delay, action -> handler.postDelayed({ action() }, delay); Unit },
        report = report,
    )
    private val route = object : AudioDeviceCallback() {
        override fun onAudioDevicesRemoved(devices: Array<out AudioDeviceInfo>) { executor.interrupt() }
    }
    init { audio.registerAudioDeviceCallback(route, handler) }
    fun refresh() { handler.post { executor.refresh() } }
    fun speak(value: JSONObject) {
        // Decode before crossing threads; never retain a mutable bridge object.
        val request = SpeechRequest.decode(value) ?: return
        handler.post { executor.speak(request) }
    }
    fun cancel(id: Long) { handler.post { executor.cancel(id) } }
    fun openVoiceSettings(expiresAt: Long, done: (Boolean) -> Unit) { handler.post {
        val process = android.app.ActivityManager.RunningAppProcessInfo()
        android.app.ActivityManager.getMyMemoryState(process)
        if (closed || SystemClock.elapsedRealtime() >= expiresAt ||
            process.importance > android.app.ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND ||
            !canOpenVoiceSettings(context)) { done(false); return@post }
        val launched = runCatching {
            context.startActivity(voiceSettingsIntent().addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            executor.voiceSettingsOpened()
        }.isSuccess
        done(launched)
    } }
    companion object {
        private fun voiceSettingsIntent() = Intent("com.android.settings.TTS_SETTINGS")
        fun canOpenVoiceSettings(context: Context): Boolean =
            voiceSettingsIntent().resolveActivity(context.packageManager) != null
    }
    fun close() { handler.post {
        if (!closed) {
            closed = true
            executor.close()
            audio.unregisterAudioDeviceCallback(route)
        }
    } }

    private inner class AndroidSpeechEngine : SpeechEngine {
        private var tts: TextToSpeech? = null
        override fun initialize(done: (Boolean) -> Unit) {
            tts = TextToSpeech(context) { status -> handler.post { done(status == TextToSpeech.SUCCESS) } }
        }
        override fun selectVoice(): String? {
            val current = tts ?: return "engine_unavailable"
            val voice = chooseOfflineEnglishVoice(current.voices.orEmpty(), current.voice, Locale.getDefault())
                ?: return "no_offline_voice"
            return if (current.setVoice(voice) == TextToSpeech.SUCCESS && current.setAudioAttributes(attributes) == TextToSpeech.SUCCESS)
                null else "engine_unavailable"
        }
        override fun speak(request: SpeechRequest, done: (Long, String) -> Unit): Boolean {
            val current = tts ?: return false
            current.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                override fun onStart(utteranceId: String?) = Unit
                override fun onDone(utteranceId: String?) = complete(utteranceId, "finished")
                @Deprecated("Android callback") override fun onError(utteranceId: String?) = complete(utteranceId, "unavailable")
                override fun onError(utteranceId: String?, errorCode: Int) = complete(utteranceId, "unavailable")
                override fun onStop(utteranceId: String?, interrupted: Boolean) = complete(utteranceId, "interrupted")
                private fun complete(id: String?, outcome: String) {
                    val operation = id?.toLongOrNull() ?: return
                    handler.post { done(operation, outcome) }
                }
            })
            val params = Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, request.volume / 100f) }
            return current.speak(request.phrase, TextToSpeech.QUEUE_FLUSH, params, request.id.toString()) == TextToSpeech.SUCCESS
        }
        override fun stop() { tts?.stop() }
        override fun shutdown() { tts?.shutdown(); tts = null }
    }
}
