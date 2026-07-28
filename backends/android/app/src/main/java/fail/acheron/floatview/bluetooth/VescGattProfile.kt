package fail.acheron.floatview.bluetooth

import java.util.UUID

object VescGattProfile {
    val service: UUID = UUID.fromString("6e400001-b5a3-f393-e0a9-e50e24dcca9e")
    val write: UUID = UUID.fromString("6e400002-b5a3-f393-e0a9-e50e24dcca9e")
    val notify: UUID = UUID.fromString("6e400003-b5a3-f393-e0a9-e50e24dcca9e")
}
