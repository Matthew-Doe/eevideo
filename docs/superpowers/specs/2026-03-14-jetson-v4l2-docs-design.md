# Jetson V4L2 Docs Design

**Date:** 2026-03-14

## Goal

Add Jetson `v4l2` camera instructions to the existing Jetson setup docs without
reintroducing the docs overlap that was just removed.

## Scope

Update these docs:

- `docs/jetson-nano-jetpack4-first-time-setup.md`
- `docs/jetson-orin-first-time-setup.md`
- `docs/eedeviced-provider-guide.md`

Do not create another Jetson setup guide.

## Design

### Jetson setup guides

Both Jetson setup guides keep the current CSI `pipeline` path as the default for
`nvarguscamerasrc` cameras.

Each guide adds an alternate Jetson `v4l2` camera path for devices that appear
as `/dev/videoX`.

The new `v4l2` sections should include:

- when to use `v4l2` on Jetson
- `v4l2-ctl --list-devices`
- `v4l2-ctl -d /dev/video0 --list-formats-ext`
- a local `gst-launch-1.0 v4l2src ... ! fakesink` validation command
- a manual `eedeviced --input v4l2 --device /dev/video0 ...` example
- matching `EEVIDEO_INPUT=v4l2` and `EEVIDEO_DEVICE=/dev/video0` service config
- a note that width, height, fps, and pixel format must match a mode reported by
  `v4l2-ctl`

The `v4l2` path should be presented as an alternate hardware path, not as the
new default for Jetson CSI cameras.

### Provider guide

The provider guide should clarify this Jetson rule:

- use `pipeline` for Jetson CSI cameras driven through `nvarguscamerasrc`
- use `v4l2` for Jetson cameras or grabbers exposed as `/dev/videoX`

The provider guide only needs one concise Jetson-specific `v4l2` example plus
that routing rule. It should stay a reference doc, not become another full
bring-up walkthrough.

## Constraints

- Avoid duplicating the full Linux `v4l2` walkthrough from
  `docs/linux-device-first-time-setup.md`
- Avoid undoing the recent docs consolidation
- Keep the existing Jetson recommendation that built-in `argus` is not the
  tested deployment path

## Expected Result

Someone using a Jetson with a V4L2 camera can stay in the Jetson docs, follow a
complete alternate path, and not have to infer how to translate the general
Linux guide back to Jetson.
