package fail.acheron.floatview

class VescCommandGenerator(private var canId: Int? = null) {
    private fun getPayload(vararg values: Int): MutableList<Int> {
        val payload = mutableListOf<Int>()
        canId?.also { canId ->
            payload.add(CommPacketId.COMM_FORWARD_CAN())
            payload.add(canId)
        }

        payload.addAll(values.toList())
        return payload
    }

    fun setCanId(id: Int?) {
        canId = id
    }

    /*
     * Vesc commands
     */

    fun firmware(): List<Int> {
        return getPayload(CommPacketId.COMM_FW_VERSION())
    }

    fun getValues(): List<Int> {
        return getPayload(CommPacketId.COMM_GET_VALUES())
    }

    /*
     * Refloat commands
     */

    private fun getRefloatPayload(vararg values: Int): List<Int> {
        return getPayload(CommPacketId.COMM_CUSTOM_APP_DATA(), 101, *values)
    }

    fun refloatInfo(version: Int): List<Int> {
        return getRefloatPayload(RefloatCommPacketId.COMMAND_INFO(), version)
    }

    fun refloatRtdata(): List<Int> {
        return getRefloatPayload(RefloatCommPacketId.COMMAND_GET_RTDATA())
    }
}
