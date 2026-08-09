package fail.acheron.floatview.host

import fail.acheron.floatview.location.AndroidLocationProvider

// Capability status: implemented — provider tests and physical background GPS evidence pass.
// Concrete OS owner: AndroidLocationProvider.
internal typealias FloatViewLocation = AndroidLocationProvider
