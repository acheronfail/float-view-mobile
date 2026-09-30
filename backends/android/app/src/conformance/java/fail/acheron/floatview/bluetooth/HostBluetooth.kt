package fail.acheron.floatview.bluetooth

import android.content.Context
import fail.acheron.floatview.application.*
import org.json.JSONObject
import java.io.File

internal fun hostBluetooth(context: Context, broker: BluetoothPermissionHistory,
    emit: (BluetoothEvent) -> Unit, diagnostic: (String) -> Unit): BluetoothPort {
    val file = File(context.filesDir, "fixture-run.json")
    require(file.length() in 1..4096) { "Missing conformance fixture credentials" }
    val config = JSONObject(file.readText())
    check(file.delete())
    return FixtureBluetoothAdapter(config, emit)
}
