# Final source and package inventory

Production source uses the unchanged convergence audit exclusions.

| Source   | Baseline | Selected server |
| -------- | -------: | --------------: |
| Rust     |   28,550 |          32,760 |
| Apple    |   11,338 |           9,490 |
| Android  |   10,635 |           7,672 |
| Combined |   50,523 |          49,922 |

Net reduction: **601 lines**. Workspace members: 26 (<34).
Both existing binding families remain; no temporary bridge or second runtime.
The existing <59,280 convergence limit also passes. The TypeScript wire contract
and canonical SQL files have no diff from the planning baseline.

The Rust lockfile adds 62 external package/version entries, including target-only
and optional packages; licenses and features are in dependencies.md. FlyingFox
0.27.1 and Ktor 3.0.3 server CIO/core/WebSocket dependencies were deleted.
Android explicitly retains its existing coroutines-core 1.9.0 for native imports.
No TLS, HTTP/2, multi-thread Tokio runtime or extra binding library is selected.

| Artifact                         | Baseline bytes | Final bytes | Delta bytes | Final SHA-256                                                    |
| -------------------------------- | -------------: | ----------: | ----------: | ---------------------------------------------------------------- |
| macOS Release executable         |        8448416 |     9498816 |    +1050400 | 5c16affc778a72a436f0ae0f9606e285365ea0b659c55b8bab36e3dd2987dd54 |
| iOS simulator Release executable |       20388264 |    20558280 |     +170016 | a1443cdbd4741d4a26f6d9b1ddcb656092c1a5edf4ade3638d96068ac3d2d832 |
| iOS device Debug launcher        |          72552 |       72552 |          +0 | bbcbd36a93d9fa390914dd86d798b2474488616751535a609b76cd27535cd2bb |
| Android Debug APK                |       35831485 |    56030093 |   +20198608 | 0d57ae4bf205c2994ec6a6a00c400309f446e1cef4744f5642f7556b343916e9 |
| Android Release APK              |       23999306 |    29571836 |    +5572530 | 59bcd41512403781b67ed428616207d0bae4058fcd15cdd47b3c13ba3c81087a |
| Apple Debug Rust archive         |       66985496 |   144625624 |   +77640128 | b9b00aee1e9ca9cc3af449f8772e72e8156b98c4da873d38a470058fe91a0b42 |
| Apple Release Rust archive       |       23110480 |    31282488 |    +8172008 | cf9fe4ffc56f9603b004df09dfe1bce5b4c0906cafe315a915ce16c2e22a250a |

Apple application baselines were freshly packaged before implementation; Android
and archive baselines were the existing local artifacts recorded in baseline.md,
so those deltas are observational, not controlled package-size benchmarks.
The iOS Debug launcher excludes its debug dylib and must not be read as total
application size. Networking and compression increase linked binary size despite
the reduction in maintained source. Debug Rust archives include debug information.
