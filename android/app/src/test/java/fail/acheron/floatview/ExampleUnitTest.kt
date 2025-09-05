package fail.acheron.floatview

import android.annotation.SuppressLint
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlinx.serialization.modules.SerializersModule
import kotlinx.serialization.modules.polymorphic
import kotlinx.serialization.modules.subclass
import org.junit.Test

import org.junit.Assert.*

@Serializable
sealed class BackendCommand {

    @Serializable
    @SerialName("scan")
    class Scan : BackendCommand()

    @Serializable
    @SerialName("connect")
    data class Connect(val address: String) : BackendCommand()
}

/**
 * Example local unit test, which will execute on the development machine (host).
 *
 * See [testing documentation](http://d.android.com/tools/testing).
 */
class ExampleUnitTest {
    @Test
    fun addition_isCorrect() {
        assertEquals(4, 2 + 2)
    }

    @Test
    fun testJson() {
        val serializersModule = SerializersModule {
            polymorphic(BackendCommand::class) {
                subclass(BackendCommand.Scan::class)
                subclass(BackendCommand.Connect::class)
            }
        }

        val json = Json {
            this.serializersModule = serializersModule
            this.classDiscriminator = "kind"
        }

        val msgScan = json.decodeFromString<BackendCommand>("""{"kind":"scan"}""")
        println(msgScan)
        val msgConnect = json.decodeFromString<BackendCommand>("""{"kind":"connect","address":"asdf"}""")
        println(msgConnect)

        when (msgConnect) {
            is BackendCommand.Scan -> {
                println(msgConnect)
            }
            is BackendCommand.Connect -> {
                println(msgConnect.address)
            }
        }
    }
}
