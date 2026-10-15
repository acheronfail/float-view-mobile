---
name: commit-messages
description: Write and review Float View commit messages and squash titles. Apply when preparing a commit or choosing a commit title.
---

# Commit messages

Use consistent area prefixes and concrete, action-led titles so Float View's
history is easy to scan. These conventions do not require release-note
generation, changelog entries, or release tooling changes.

## Required format

Use exactly one lowercase area prefix followed by a colon, one space, and a
short description:

`<area>: <description>`

| Prefix | Area affected |
| --- | --- |
| `app` | Shared app behavior, including the frontend, shared Rust service, protocol handling, and persistence |
| `apple` | Behavior specific to the iOS or macOS host and its OS integrations |
| `android` | Behavior specific to the Android host and its OS integrations |
| `repo` | Repository guidance, build and release tooling, tests, documentation, and maintenance without an app behavior change |

Choose the prefix by the effect of the change, not just its file location.
Tests and documentation accompanying a behavior change share its area.
A shared behavior change that also updates native adapters uses `app`.
An Apple-only change uses `apple`; name iOS or macOS in the description when
the distinction matters.

Split independent changes into separate commits. For an inseparable change,
choose the primary affected area. Do not use multiple prefixes or invent new
ones.

## Wording

- Keep the whole title at most 72 characters, and shorter when clear.
- Start the description with a capitalized plain action verb, such as Add, Fix,
  Show, or Improve. Describe the concrete outcome in simple language.
- For behavior changes, describe what changes for someone using the app. For
  repository work, name the concrete maintenance outcome.
- Avoid vague titles such as "Fix bugs" or "Update stuff". Keep internal symbol
  names, ticket numbers, and implementation details out of the title; put
  essential technical context in the body.
- Omit a trailing period. Do not add change-type prefixes such as `fix:` or
  `feat(app):`.
- Usually use only a title. Add a brief body when the reason, tradeoff, or
  verification limitation would help a future reader understand the change.
- Apply the same format to squash and revert titles. For a revert, use an
  action-led title such as `app: Revert curved session chart smoothing` and
  identify the reverted commit in the body.
- Apply these conventions to new commits; do not rewrite existing history
  solely to normalize its messages.

Examples:

- `app: Add curved smoothing to session charts`
- `app: Keep ride location prompts out of the live layout`
- `apple: Lower other audio during iOS announcements`
- `android: Support Samsung speech voices`
- `repo: Add commit message guidance`
- `repo: Test live layouts at phone and tablet sizes`

## Preparing a commit

Inspect the intended diff before choosing the title, including the staged diff
when committing. Keep unrelated working-tree changes out of the commit.
Follow the verification policy in `AGENTS.md` and `CONTRIBUTING.md`; this skill
does not add a full-suite or physical-testing gate.

A request to suggest or review a message does not authorize creating a commit.
Creating a commit does not imply pushing it, tagging a release, or publishing
release notes.
