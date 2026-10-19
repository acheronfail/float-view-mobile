package fail.acheron.floatview.ui

import java.net.URI

internal enum class WebNavigation { BACKEND, EXTERNAL, BLOCK }

/** Only the bundled document stays in the WebView; public web links leave it. */
internal fun webNavigation(url: String, origin: String): WebNavigation {
    val target = runCatching { URI(url) }.getOrNull() ?: return WebNavigation.BLOCK
    val backend = runCatching { URI(origin) }.getOrNull() ?: return WebNavigation.BLOCK
    if (target.scheme == backend.scheme && target.rawAuthority == backend.rawAuthority &&
        target.rawPath.orEmpty() in setOf("", "/") && target.rawQuery == null) return WebNavigation.BACKEND
    val host = target.host?.lowercase() ?: return WebNavigation.BLOCK
    if (target.scheme?.lowercase() !in setOf("http", "https") || target.rawUserInfo != null ||
        host == "localhost" || host.endsWith(".localhost") || host.startsWith("127.") ||
        host in setOf("[::1]", "::1", "0.0.0.0") || host == backend.host) return WebNavigation.BLOCK
    return WebNavigation.EXTERNAL
}
