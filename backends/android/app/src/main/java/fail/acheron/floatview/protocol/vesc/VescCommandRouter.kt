package fail.acheron.floatview.protocol.vesc

sealed interface ControllerRoute {
    data object Direct : ControllerRoute
    data class Can(val canId: Int) : ControllerRoute {
        init {
            require(canId in 0..253)
        }
    }
}

object VescCommandRouter {
    /** The sole owner of COMM_FORWARD_CAN wrapping. */
    fun route(command: ByteArray, via: ControllerRoute): ByteArray =
        when (via) {
            ControllerRoute.Direct -> command
            is ControllerRoute.Can ->
                byteArrayOf(VescCommand.FORWARD_CAN.byte, via.canId.toByte()) + command
        }
}
