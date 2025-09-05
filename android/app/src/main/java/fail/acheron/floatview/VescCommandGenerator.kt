package fail.acheron.floatview

class VescCommandGenerator(private var canId: Int? = null) {
    private fun getPayload(): MutableList<Int> {
        val payload = mutableListOf<Int>()
        canId?.also { canId ->
            payload.add(CommPacketId.COMM_FORWARD_CAN.value)
            payload.add(canId)
        }

        return payload
    }

    fun setCanId(id: Int?) {
        canId = id
    }

    fun firmware(): List<Int> {
        val payload = getPayload()
        payload.add(CommPacketId.COMM_FW_VERSION.value)
        return payload
    }
}
