package fail.acheron.floatview.speech
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
class SpeechRequestTest {
    private fun value() = JSONObject("""{"id":1,"expires_at_ms":10000,"footpad":null,"load":{"source":"motor","percent":78},"speed":32,"imperial":false,"volume":42,"lower_other_audio":true}""")
    @Test fun combinedPhrasePreservesRustWinnerAndGain() {
        val request = SpeechRequest.decode(value())!!
        assertEquals("motor 78 percent, 32 kilometres per hour", request.phrase)
        assertEquals(42, request.volume)
    }
    @Test fun malformedRequestsAreRejectedBeforeAudioAdmission() {
        for (bad in listOf(value().put("volume",101), value().put("speed",1.5), value().put("text","arbitrary"), value().put("imperial","false"), value().put("load",JSONObject("""{"source":"other","percent":50}""")))) {
            assertNull(SpeechRequest.decode(bad))
        }
    }
    @Test fun speedOnlyAndImperialPhrasesOmitRedundantPrefix() {
        assertEquals("32 kilometres per hour", SpeechRequest.decode(value().put("load", JSONObject.NULL))!!.phrase)
        assertEquals("motor 78 percent, 32 miles per hour", SpeechRequest.decode(value().put("imperial", true))!!.phrase)
    }

    @Test fun footpadsAreClosedAndComeBeforeLoadAndSpeed() {
        for ((wire, phrase) in listOf("left" to "left footpad off", "right" to "right footpad off", "both" to "both footpads off")) {
            assertEquals("$phrase, motor 78 percent, 32 kilometres per hour", SpeechRequest.decode(value().put("footpad", wire))!!.phrase)
            assertEquals(phrase, SpeechRequest.decode(value().put("footpad", wire).put("load", JSONObject.NULL).put("speed", JSONObject.NULL))!!.phrase)
        }
        for (raw in listOf("none", "unknown", "LEFT", 1, true)) assertNull(SpeechRequest.decode(value().put("footpad", raw)))
        assertNull(SpeechRequest.decode(value().also { it.remove("footpad") }))
        assertNull(SpeechRequest.decode(value().put("load", JSONObject.NULL).put("speed", JSONObject.NULL)))
    }
}
