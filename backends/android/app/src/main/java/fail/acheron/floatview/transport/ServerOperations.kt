package fail.acheron.floatview.transport

import fail.acheron.floatview.application.*
import org.json.JSONObject
import java.io.File
import java.io.RandomAccessFile
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

/** Executes closed domain operations against the one process-owned application. */
internal class ServerOperations(
    private val app: ApplicationService,
    private val handle: Long,
    private val timer: ScheduledExecutorService,
    private val present: ((SessionExportRecord) -> Boolean)?,
) : AutoCloseable {
    private val files = mutableMapOf<Long, Pair<File, RandomAccessFile>>()
    private var scan: java.util.concurrent.ScheduledFuture<*>? = null
    private fun result(kind: String, facts: Any? = null) = ServerFacts.objectOf("kind" to kind).also {
        if (kind != "unit" && kind != "transfer_ready") it.put("facts", ServerFacts.encode(facts))
    }
    @Synchronized fun cleanup(id: Long) { files.remove(id)?.let { (file, stream) -> stream.close(); file.delete() } }
    @Synchronized override fun close() { files.keys.toList().forEach(::cleanup); scan?.cancel(false) }
    fun execute(op: JSONObject, generation: Long): JSONObject {
        fun s(key: String) = op.getString(key)
        fun n(key: String) = op.getLong(key)
        fun optional(key: String) = ServerFacts.nullable(op, key)
        fun values() = ServerFacts.values(op.getJSONObject("values"))
        fun identity() = ServerFacts.identity(op.getJSONObject("identity"))
        return when (s("operation")) {
            "request_bluetooth_permission" -> result("bluetooth", app.requestBluetoothPermission())
            "request_location_permission" -> result("location", app.requestLocationPermission())
            "open_location_settings" -> result("location", app.openLocationSettings())
            "scan" -> { val id = app.startDiscovery(); scan?.cancel(false); scan = timer.schedule({ app.stopDiscovery(id) }, n("timeout_ms"), TimeUnit.MILLISECONDS); result("unit") }
            "connect" -> { app.connect(DeviceId(s("device_id")), if (op.getBoolean("saved")) TrackingMode.SAVED else TrackingMode.GUEST); result("unit") }
            "select_controller" -> { app.selectController(DeviceId(s("device_id")), op.getInt("can_id")); result("unit") }
            "disconnect" -> { app.disconnect(); result("unit") }
            "shutdown_controller" -> { app.shutdownController(); result("unit") }
            "set_control" -> result("controls", app.setRefloatControl(RefloatBooleanControl.entries.single { it.wireValue == s("control") }, op.getBoolean("enabled")))
            "clear_fatal_error" -> result("controls", app.clearRefloatFatalError())
            "configuration_schema" -> result("configuration_schema", requireNotNull(app.refloatConfigurationSchemaSnapshot()))
            "read_configuration" -> result("configuration_values", app.readRefloatConfiguration(op.getBoolean("defaults")))
            "write_configuration" -> result("configuration_values", app.writeRefloatConfiguration(s("package_version"), s("fingerprint"), n("signature"), n("revision"), values()))
            "list_tunes" -> result("tunes", app.refloatTunes(identity(), op.getInt("limit"), optional("cursor")))
            "get_tune" -> result("tune", app.refloatTune(s("id")))
            "create_tune" -> result("tune", app.createRefloatTune(s("name"), optional("description"), identity(), n("revision"), values()))
            "update_tune" -> result("tune", app.updateRefloatTune(s("id"), op.getInt("revision"), s("name"), optional("description"), values()))
            "delete_tune" -> { app.deleteRefloatTune(s("id"), op.getInt("revision")); result("unit") }
            "apply_tune" -> result("configuration_values", app.applyRefloatTune(s("id"), op.getInt("revision"), n("base_revision")))
            "update_saved_board" -> result("saved_board", app.updateSavedDevice(s("id"), op.getInt("revision"), optional("name")))
            "delete_saved_board" -> { app.deleteSavedDevice(s("id")); result("unit") }
            "update_settings" -> result("settings", app.updateSettings(op.getInt("revision"), s("patch_json")))
            "get_profile" -> result("profile", app.boardProfile(s("id")))
            "update_profile" -> result("profile", app.updateBoardProfile(s("id"), op.getInt("revision"), s("patch_json")))
            "list_sessions" -> result("sessions", app.sessions(SessionListQuery(cursor=optional("cursor"), limit=op.getInt("limit"), deviceHistoryId=optional("history_id"), state=if(op.getBoolean("completed_only")) "completed" else null)))
            "get_session" -> result("session", ServerFacts.objectOf("summary" to app.session(s("id")), "boundary" to app.sessionSnapshot(s("id")), "path" to app.sessionPath(s("id"))))
            "delete_session" -> { app.deleteSession(s("id")); result("unit") }
            "cancel_delete" -> result("cancel_delete", app.cancelPendingDelete(s("id")))
            "get_maintenance" -> result("maintenance", app.maintenanceJob())
            "cancel_maintenance" -> result("maintenance", app.cancelMaintenanceJob(s("id")))
            "save_maintenance" -> { val j = op.getJSONObject("job"); fun nullable(key: String) = ServerFacts.nullable(j,key)
                app.saveMaintenanceJob(SessionMaintenanceJobRecord(j.getString("operationId"),j.getString("kind"),j.getString("scope"),j.getString("state"),nullable("sessionId"),j.getLong("targetResolutionSeconds"),nullable("upperSessionId"),nullable("lastSessionId"),j.getLong("completedSessions"),j.getLong("skippedSessions"),j.getLong("retainedSamples"),j.getLong("reclaimableBytes"),nullable("error"))); result("unit") }
            "prune_session" -> result("prune_preview", if(op.getBoolean("preview")) app.prunePreview(s("id"),n("resolution")) else app.pruneSession(s("id"),n("resolution")))
            "begin_import" -> { synchronized(this) { check(files.size<2); val file=File.createTempFile("float-view-import-", ".tmp"); files[n("transfer_id")]=file to RandomAccessFile(file,"rw") }; result("transfer_ready") }
            "write_transfer" -> { val data=ByteArray(65536); val size=IntArray(1); check(RustApplicationRuntime.serverReadTransfer(handle,generation,n("transfer_id"),data,size)==0); synchronized(this) { val stream=files.getValue(n("transfer_id")).second; check(stream.filePointer==n("offset")); stream.write(data,0,size[0]) }; result("transfer_ready") }
            "finish_import" -> { val file=synchronized(this) { files.getValue(n("transfer_id")).first }; result("imported",app.importSessionArchive(file.readBytes())) }
            "export_session" -> { val export=app.exportSession(s("id"),s("format")); val file=File(export.filePath); val presented=op.getBoolean("present") && present?.invoke(export)==true
                if(n("transfer_id")!=0L) synchronized(this) { check(files.size<2); files[n("transfer_id")]=file to RandomAccessFile(file,"r") }
                result("export",ServerFacts.objectOf("file_name" to export.fileName,"presented" to presented,"transfer_id" to n("transfer_id"),"length" to file.length())) }
            "read_transfer" -> { val data=synchronized(this) { val stream=files.getValue(n("transfer_id")).second; stream.seek(n("offset")); ByteArray(op.getInt("length")).also(stream::readFully) }; check(RustApplicationRuntime.serverWriteTransfer(handle,generation,n("transfer_id"),n("offset"),data)==0); result("transfer_ready") }
            "cleanup_transfer" -> { cleanup(n("transfer_id")); result("unit") }
            else -> error("Unknown host operation")
        }
    }
}
