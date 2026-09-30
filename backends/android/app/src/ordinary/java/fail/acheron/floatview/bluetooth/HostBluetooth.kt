package fail.acheron.floatview.bluetooth

import android.content.Context
import fail.acheron.floatview.application.*

internal fun hostBluetooth(context: Context, broker: BluetoothPermissionHistory,
    emit: (BluetoothEvent) -> Unit, diagnostic: (String) -> Unit): BluetoothPort =
    AndroidBluetoothAdapter(context, broker, emit, diagnostic)
