package fail.acheron.floatview


class VescCommandLayer(private val address: String) {

    val parser = VescCommandParser()

    fun parse(command: Int, response: ByteArray) {
        val buf = VescBuffer(response)
        when (CommPacketId.fromInt(command)) {
            CommPacketId.COMM_FW_VERSION ->
                    LocalServer.broadcastMessage(
                            BackendMessage.commandFirmware(address, parser.firmware(buf))
                    )
            else -> {
                LocalServer.broadcastMessage(
                        BackendMessage.commandError(
                                command.toDouble(),
                                "No parser for command: $command"
                        )
                )
            }
        }
    }
}
