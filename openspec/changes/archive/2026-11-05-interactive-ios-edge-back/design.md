# Design

## Context

The existing iOS recognizer dispatches Back only on release; shared Svelte owns logical return destinations, dismissal guards and list restoration.

## Decisions

- Move one temporary viewport snapshot in the native compositor, above the unchanged WebView, with a neutral canvas and Back affordance underneath. Transforming the live document would change containing blocks for fixed controls and risk chart reflow; retaining another mounted route/WebView adds ownership and lifecycle complexity. A live parent preview is excluded.
- Capture once on gesture begin and retain latest translation while capture is pending. Update the image transform directly on movement. Clip the presentation to the WebView viewport, hide it from accessibility and prevent new interaction during settling. Bound state to one capture/layer and reject stale callbacks with a generation.
- On release use the same Back event with an optional completion callback. Shared Back may return a promise, resolved true only after accepted route change and presentation restoration. Surface dismissal, blocked/failed navigation and ineligible requests resolve false. Existing system Back remains synchronously consumed when eligible.
- Await the acknowledgement with a finite timeout. On success reveal the rendered parent beneath the outgoing snapshot and finish sliding out. Otherwise settle back. Remove the layer on completion, cancellation, backgrounding, document replacement or content-process failure. Reduce Motion removes automatic settling animation.

## Risks / Trade-offs

Snapshot capture has small startup latency; asynchronous completion uses only latest gesture state. The backdrop is neutral while dragging rather than a live parent preview. Frozen viewport content is temporary and never persisted. Guards and restored scroll need explicit browser/native verification.

## Migration Plan

No persistence or backend changes. Existing native gesture availability stays boolean. Remove the former release-only dispatch path rather than retain a fallback navigation implementation.
