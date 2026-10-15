package fail.acheron.floatview.speech
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
class SpeechRequestTest {
    private fun value() = JSONObject("""{"id":1,"expires_at_ms":10000,"load":{"source":"motor","percent":78},"speed":32,"imperial":false,"volume":42,"lower_other_audio":true}""")
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

}
