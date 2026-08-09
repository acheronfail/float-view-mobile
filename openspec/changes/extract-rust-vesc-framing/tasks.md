## 1. Establish the bounded Rust workspace

- [x] 1.1 Add a Rust 1.93.0 toolchain file and a locked `backends/core` Cargo workspace containing exactly one VESC framing crate with no normal third-party dependencies or mobile build targets.
- [x] 1.2 Document `backends/core` as the candidate portable-component location in `AGENTS.md` and backend ownership documentation, including the no-production-cutover boundary and the later per-host deletion checkpoint.
- [x] 1.3 Extend repository layout and dependency auditing to reject unexpected core workspace members, generated bindings, host callbacks, and normal third-party dependencies in this slice.

## 2. Implement VESC framing

- [x] 2.1 Implement dependency-free CRC-16 plus bounded short/long VESC frame encoding with checked length arithmetic, the 4,096-byte payload limit, and typed errors.
- [x] 2.2 Implement exact frame decoding that validates supported headers, declared and actual length, footer, CRC, payload bounds, and absence of trailing bytes.
- [x] 2.3 Implement bounded stream reassembly that matches shipped noise, fragmentation, multiple-frame, malformed-input recovery, and 8,192-byte aggregate-buffer behavior without logging sensitive bytes.

## 3. Prove component parity

- [x] 3.1 Add Rust tests that reuse immutable source-derived framing evidence and cover short, long, empty, maximum, and over-limit payload boundaries without creating a Rust-only protocol oracle.
- [x] 3.2 Add adversarial stream tests for one-byte fragmentation, split headers, leading noise, multiple frames, CRC/footer/header corruption, inconsistent and oversized declarations, buffer overflow, and recovery after malformed input.
- [x] 3.3 Confirm the existing Swift, Kotlin, and conformance framing tests still pass unchanged and that neither production host imports, links, packages, or calls the Rust candidate.

## 4. Integrate deterministic verification

- [x] 4.1 Add focused locked Cargo format-check, warnings-denied Clippy, and workspace-test recipes and include them in the existing `just lint`, `just test all`, and `just verify` flow without installing cross-compilation targets.
- [x] 4.2 Run the focused Rust checks plus `just test apple`, `just test android`, `just test conformance`, and `just audit`, resolving every framing or ownership mismatch.
- [x] 4.3 Run `just verify` successfully and record that this change establishes component parity only; Apple and Android production cutovers remain separate future OpenSpec changes.
