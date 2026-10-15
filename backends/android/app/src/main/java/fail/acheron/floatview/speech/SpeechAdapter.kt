package fail.acheron.floatview.speech

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import org.json.JSONObject
import java.util.Locale

/** Main-looper confined OS speech executor; no telemetry or announcement policy. */
class SpeechAdapter(private val context: Context, private val report: (String, Long, String?) -> Unit) {
    private val handler = Handler(Looper.getMainLooper())
    private val audio = context.getSystemService(AudioManager::class.java)
    private val attributes = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ASSISTANCE_NAVIGATION_GUIDANCE)
        .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH).build()
    private var engine: TextToSpeech? = null
    private var ready = false
    private var initializing = false
    private var epoch = 0L
    private val lease = SpeechLease()
    private var closed = false
    private val route = object : AudioDeviceCallback() {
        override fun onAudioDevicesRemoved(devices: Array<out AudioDeviceInfo>) { finish("interrupted") }
    }
    init { audio.registerAudioDeviceCallback(route, handler) }
    fun refresh() { handler.post { refreshOnMain() } }
    private fun refreshOnMain() {
        if (closed || initializing) return
        if (engine != null && ready) { selectVoice(); return }
        initializing = true
        val token = ++epoch
        report("initializing", 0, null)
        engine?.shutdown()
        engine = TextToSpeech(context) { status -> handler.post {
            if (token != epoch || closed) return@post
            initializing = false
            ready = status == TextToSpeech.SUCCESS
            if (ready) selectVoice() else report("unavailable", 0, "engine_unavailable")
        } }
        handler.postDelayed({
            if (token == epoch && initializing) {
                initializing = false; ready = false; epoch++
                engine?.shutdown(); engine = null
                report("unavailable", 0, "engine_unavailable")
            }
        }, 10_000)
    }
    private fun selectVoice() {
        val tts = engine ?: return
        val voice = tts.voices?.filter { !it.isNetworkConnectionRequired && isEnglishSpeechLocale(it.locale) }
            ?.sortedWith(compareByDescending<android.speech.tts.Voice> { it.locale == Locale.getDefault() }.thenBy { it.name })?.firstOrNull()
        ready = voice != null
        if (voice == null) { report("unavailable", 0, "no_offline_voice"); return }
        if (tts.setVoice(voice) != TextToSpeech.SUCCESS || tts.setAudioAttributes(attributes) != TextToSpeech.SUCCESS) {
            ready = false; report("unavailable", 0, "engine_unavailable"); return
        }
        report("ready", 0, null)
    }
    fun speak(value: JSONObject) {
        // Decode before crossing threads; never retain a mutable bridge object.
        val request = SpeechRequest.decode(value) ?: return
        val id = request.id
        val expires = request.expires
        val volume = request.volume
        handler.post {
            if (android.os.SystemClock.elapsedRealtime() >= expires || closed || lease.active != null || !ready) { report("outcome", id, "unavailable"); return@post }
            val focusRequest = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK)
                .setAudioAttributes(attributes).setAcceptsDelayedFocusGain(false)
                .setOnAudioFocusChangeListener({ change -> if (change < 0 && lease.active == id) finish("interrupted") }, handler).build()
            if (audio.requestAudioFocus(focusRequest) != AudioManager.AUDIOFOCUS_REQUEST_GRANTED) {
                report("outcome", id, "unavailable"); return@post
            }
            lease.begin(id) { engine?.stop(); audio.abandonAudioFocusRequest(focusRequest) }
            engine?.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                override fun onStart(utteranceId: String?) = Unit
                override fun onDone(utteranceId: String?) = complete(utteranceId, "finished")
                @Deprecated("Android callback") override fun onError(utteranceId: String?) = complete(utteranceId, "unavailable")
                override fun onError(utteranceId: String?, errorCode: Int) = complete(utteranceId, "unavailable")
                override fun onStop(utteranceId: String?, interrupted: Boolean) = complete(utteranceId, "interrupted")
            })
            val params = Bundle().apply { putFloat(TextToSpeech.Engine.KEY_PARAM_VOLUME, volume / 100f) }
            if (engine?.speak(request.phrase, TextToSpeech.QUEUE_FLUSH, params, id.toString()) != TextToSpeech.SUCCESS) finish("unavailable")
            handler.postDelayed({ if (lease.active == id) finish("unavailable") }, (expires - android.os.SystemClock.elapsedRealtime()).coerceAtLeast(0))
        }
    }
    private fun complete(id: String?, outcome: String) { handler.post { if (id == lease.active?.toString()) finish(outcome) } }
    fun cancel(id: Long) { handler.post { if (lease.active == id) finish("interrupted") } }
    private fun finish(outcome: String) {
        val id = lease.active ?: return
        if (lease.finish(id)) report("outcome", id, outcome)
    }
    fun close() { handler.post {
        closed = true; epoch++; finish("interrupted")
        audio.unregisterAudioDeviceCallback(route)
        engine?.shutdown(); engine = null
    } }
}
