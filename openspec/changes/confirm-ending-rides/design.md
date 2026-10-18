# Design

## Context

Use the existing shared confirmation dialog on every End ride surface.

## Goals / Non-Goals

Prevent accidental completion; leave Disconnect behavior unchanged.

## Decisions

Keep pending admission and errors in the existing action owner. Cancel returns focus and sends no request. The paused-ride card owns its confirmation so Home and session review behave identically.

## Risks / Trade-offs

One extra deliberate action to finish a paused ride; automatic expiry is unchanged.
