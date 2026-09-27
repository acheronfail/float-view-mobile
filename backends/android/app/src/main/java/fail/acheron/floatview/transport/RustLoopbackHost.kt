package fail.acheron.floatview.transport

import fail.acheron.floatview.application.*
import org.json.JSONObject
import java.util.concurrent.*
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

/** Candidate host adapter. No listener is selected by production until parity gates pass. */
internal class RustLoopbackHost(
    html: ByteArray,
    private val app: ApplicationService,
    events: EventHub,
    presenter: ((SessionExportRecord) -> Boolean)?,
) : AutoCloseable {
    private val handle: Long
    private val timer = Executors.newSingleThreadScheduledExecutor()
    private val control = ThreadPoolExecutor(1,1,0,TimeUnit.SECONDS,ArrayBlockingQueue(68))
    private val publication = Executors.newSingleThreadExecutor()
    private val sequence = AtomicLong()
    private val dirty = AtomicBoolean()
    private val catalog = AtomicBoolean()
    private val operations: ServerOperations
    private val subscription: AutoCloseable
    @Volatile private var generation = 0L
    @Volatile private var running = false
    private var worker: Thread? = null
    var port = 0; private set
    init {
        RustApplicationRuntime.ensureLoaded()
        handle = RustApplicationRuntime.serverCreate(2,html)
        check(handle>0)
        operations=ServerOperations(app,handle,timer,presenter)
        subscription=events.subscribe { event -> publish(event is SessionRecord) }
    }
    private fun command(name: String, parameters: Any? = null): JSONObject {
        val action=ServerFacts.objectOf("command" to name)
        if(parameters!=null) action.put("parameters",ServerFacts.encode(parameters))
        val input=ServerFacts.objectOf("version" to 1,"action" to action).toString().toByteArray()
        require(input.size<=1048576)
        val output=ByteArray(1048576); val written=IntArray(1)
        check(RustApplicationRuntime.serverCommand(handle,input,output,written)==0) { "Shared server exchange failed" }
        return JSONObject(String(output,0,written[0],Charsets.UTF_8)).getJSONObject("data")
    }
    @Synchronized fun start() {
        if(running) return
        val result=command("start",ServerFacts.objectOf("platform" to "android","port" to 11012))
        generation=result.getLong("generation"); port=result.getInt("port"); running=true
        publish(false)
        publication.submit {}.get()
        worker=Thread({ drain() },"float-view-host").also(Thread::start)
    }
    fun publish(catalogChanged: Boolean) {
        if(catalogChanged) catalog.set(true)
        if(dirty.compareAndSet(false,true)) publication.execute {
            dirty.set(false)
            val changed=catalog.getAndSet(false)
            val facts=synchronized(app) { ServerFacts.objectOf(
                "discovery" to app.discoverySnapshot(), "active" to app.activeConnectionSnapshot(),
                "firmware" to app.firmwareSnapshot(), "packageVersion" to app.refloatPackageVersionSnapshot(),
                "configuration" to app.refloatConfigurationSummarySnapshot(), "savedBoards" to app.savedDevices().take(50),
                "settings" to app.settings(), "location" to app.locationState(), "maintenance" to app.maintenanceJob(),
            ) }
            command("publish",ServerFacts.objectOf("generation" to generation,"sequence" to sequence.incrementAndGet(),"catalogChanged" to changed,"facts" to facts))
        }
    }
    private fun drain() {
        val epoch=generation
        while(running && epoch==generation) {
            val batch=try { command("drain",ServerFacts.objectOf("generation" to epoch,"wait_ms" to 250)) } catch(_:IllegalStateException) { break }
            batch.getJSONArray("cleanup").let { ids -> (0 until ids.length()).forEach { index ->
                val id=ids.getLong(index)
                control.execute { operations.cleanup(id); command("transfer_cleaned",ServerFacts.objectOf("transfer_id" to id)) }
            } }
            batch.getJSONArray("operations").let { items -> (0 until items.length()).forEach { index ->
                val item=items.getJSONObject(index); val op=item.getJSONObject("operation"); val id=item.getJSONObject("id")
                try { control.execute {
                    val outcome=try {
                        val result=operations.execute(op,epoch)
                        ServerFacts.objectOf("status" to "completed","value" to result)
                    } catch(error:Exception) { ServerFacts.objectOf("status" to "failed","value" to failure(error)) }
                    val name=op.getString("operation")
                    if(name !in setOf("read_history","configuration_schema","list_tunes","get_tune","get_profile","list_sessions","get_session","get_maintenance","export_session","begin_import","write_transfer","read_transfer","cleanup_transfer")) {
                        publish(name in setOf("delete_session","delete_saved_board","finish_import","save_maintenance","prune_session"))
                        publication.submit {}.get()
                    }
                    try { complete(id,outcome) } catch(_:IllegalStateException) { runCatching { command("complete",ServerFacts.objectOf("id" to id,"outcome" to ServerFacts.objectOf("status" to "failed","value" to "unavailable"))) } }
                } } catch (_: RejectedExecutionException) {
                    runCatching { complete(id,ServerFacts.objectOf("status" to "failed","value" to "unavailable")) }
                }
            } }
        }
    }
    private fun complete(id: JSONObject, outcome: JSONObject) {
        val text=outcome.toString()
        if(text.toByteArray().size<900000) { command("complete",ServerFacts.objectOf("id" to id,"outcome" to outcome)); return }
        var offset=0
        // At most 8 MiB per domain result. Archives use separate binary transfers.
        require(text.toByteArray().size<=8*1024*1024)
        var position=0
        while(position<text.length) {
            var end=minOf(position+8192,text.length)
            if(end<text.length && text[end-1].isHighSurrogate()) end--
            val part=text.substring(position,end)
            command("complete_fragment",ServerFacts.objectOf("id" to id,"offset" to offset,"text" to part,"last" to (end==text.length)))
            offset+=part.toByteArray().size
            position=end
        }
    }
    @Synchronized fun stop() {
        if(!running) return
        running=false
        val cleanup=command("stop").getJSONArray("cleanup")
        worker?.join(2000); worker=null
        (0 until cleanup.length()).forEach { index -> control.execute { operations.cleanup(cleanup.getLong(index)) } }
        port=0
    }
    override fun close() {
        stop(); subscription.close(); publication.shutdown(); control.shutdown(); timer.shutdown()
        listOf(publication,control,timer).forEach { it.awaitTermination(2,TimeUnit.SECONDS) }
        operations.close(); check(RustApplicationRuntime.serverDestroy(handle)==0)
    }
    private fun failure(error: Exception): String = when(error) {
        is RefloatMutationException -> if(error.unavailable) "unavailable" else "invalid_state"
        is RefloatConfigurationServiceException -> when(error.reason) { RefloatConfigurationServiceError.CONFLICT -> "conflict"; RefloatConfigurationServiceError.UNAVAILABLE -> "unavailable"; else -> "invalid_state" }
        is ProductStoreException.ProfileRevisionConflict, is ProductStoreException.SavedDeviceRevisionConflict, is ProductStoreException.SettingsRevisionConflict, is ProductStoreException.RefloatConfigurationRevisionConflict -> "conflict"
        ProductStoreException.SavedDeviceNotFound, ProductStoreException.RefloatConfigurationNotFound, SessionStoreException.SessionNotFound -> "not_found"
        ProductStoreException.SavedDeviceActive, SessionStoreException.SessionActive -> "invalid_state"
        SessionStoreException.MaintenanceInProgress -> "unavailable"
        is ProductStoreException, is SessionStoreException, is IllegalArgumentException -> "invalid_request"
        is IllegalStateException -> "invalid_state"
        else -> "internal"
    }
}
