# Jetson V4L2 Docs Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Jetson-specific `v4l2` setup instructions to the Nano and Orin bring-up docs while keeping the provider guide as the concise reference.

**Architecture:** Keep the existing Jetson CSI `pipeline` path as the default and add a clearly labeled alternate `v4l2` path in each Jetson setup guide. Update the provider guide with the Jetson routing rule and one Jetson `v4l2` example so the setup guides stay self-contained without becoming inconsistent.

**Tech Stack:** Markdown docs, repo cross-linking, `rg`, `git diff --check`

---

## Chunk 1: Jetson Docs Content

### Task 1: Add Nano Jetson `v4l2` path

**Files:**
- Modify: `docs/jetson-nano-jetpack4-first-time-setup.md`

- [ ] **Step 1: Add an alternate `v4l2` camera path section**

Include:
- when to use `v4l2` on Nano
- `v4l2-ctl --list-devices`
- `v4l2-ctl -d /dev/video0 --list-formats-ext`
- a local `gst-launch-1.0 v4l2src ... ! fakesink` validation command
- a manual `eedeviced --input v4l2 --device /dev/video0 ...` example
- matching `EEVIDEO_INPUT=v4l2` and `EEVIDEO_DEVICE=/dev/video0` service config

- [ ] **Step 2: Keep the existing CSI `pipeline` path as the default**

Ensure the new text presents `v4l2` as an alternate path for `/dev/videoX`
devices, not as a replacement for the Jetson CSI guidance already in the file.

- [ ] **Step 3: Add or update troubleshooting notes**

Make the Nano guide explicitly say the configured width, height, fps, and pixel
format must match a mode reported by `v4l2-ctl`.

### Task 2: Add Orin Jetson `v4l2` path

**Files:**
- Modify: `docs/jetson-orin-first-time-setup.md`

- [ ] **Step 1: Add an alternate `v4l2` camera path section**

Mirror the same structure used in the Nano guide:
- `v4l2-ctl` discovery commands
- local `gst-launch-1.0 v4l2src ... ! fakesink`
- manual `eedeviced --input v4l2 --device /dev/video0 ...`
- matching service config keys

- [ ] **Step 2: Preserve the existing Jetson CSI `pipeline` path**

Keep the current Orin recommendation for `pipeline` with explicit
`nvarguscamerasrc ... ! appsink` as the default Jetson CSI route.

- [ ] **Step 3: Add or update troubleshooting notes**

State that the `v4l2` path must use a mode reported by `v4l2-ctl` and should be
validated locally before debugging EEVideo behavior.

## Chunk 2: Provider Reference

### Task 3: Add Jetson `v4l2` guidance to provider guide

**Files:**
- Modify: `docs/eedeviced-provider-guide.md`

- [ ] **Step 1: Add the Jetson routing rule**

Clarify:
- `pipeline` for Jetson CSI paths built around `nvarguscamerasrc`
- `v4l2` for Jetson cameras or frame grabbers exposed as `/dev/videoX`

- [ ] **Step 2: Add one concise Jetson `v4l2` example**

Use a short `eedeviced --input v4l2 --device /dev/video0 ...` example without
copying the full first-time setup walkthrough into the provider guide.

- [ ] **Step 3: Check cross-links**

Keep the provider guide pointing to the Jetson setup guides for full procedures.

## Chunk 3: Verification

### Task 4: Validate docs consistency

**Files:**
- Verify: `docs/jetson-nano-jetpack4-first-time-setup.md`
- Verify: `docs/jetson-orin-first-time-setup.md`
- Verify: `docs/eedeviced-provider-guide.md`

- [ ] **Step 1: Search for stale or contradictory Jetson wording**

Run:

```sh
rg -n "v4l2|pipeline|argus|/dev/video0" docs/jetson-nano-jetpack4-first-time-setup.md docs/jetson-orin-first-time-setup.md docs/eedeviced-provider-guide.md
```

Expected:
- each Jetson setup guide contains both a CSI `pipeline` path and a Jetson
  `v4l2` alternate path
- the provider guide contains the Jetson routing rule and concise example

- [ ] **Step 2: Check Markdown whitespace and patch cleanliness**

Run:

```sh
git diff --check
```

Expected:
- no trailing whitespace or malformed patch output

- [ ] **Step 3: Commit**

```bash
git add docs/jetson-nano-jetpack4-first-time-setup.md docs/jetson-orin-first-time-setup.md docs/eedeviced-provider-guide.md docs/superpowers/specs/2026-03-14-jetson-v4l2-docs-design.md docs/superpowers/plans/2026-03-14-jetson-v4l2-docs.md
git commit -m "docs: add Jetson v4l2 bring-up guidance"
```
