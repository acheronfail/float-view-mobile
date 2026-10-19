package fail.acheron.floatview.permissions

/** Location disclosure precedes first-time grants as well as settings recovery. */
internal fun needsPermissionDisclosure(permission: String, action: String, rationale: Boolean): Boolean =
    rationale || ((permission == "location" || permission == "backgroundLocation") &&
        (action == "requestPermission" || action == "openSettings"))
