# EEVideo Rust Plugin Implementation Profile

This repository implements a practical EEVideo v1 interoperability profile for
the current public EEVideo ecosystem and host-side Go behavior.

## Scope

- `eevideosrc` receives the same compatibility leader/payload/trailer stream shape
  currently parsed by the public `goeevideo` receiver path.
- `eevideosink` emits that same compatibility format for host-to-host and local
  loopback use.
- Supported raw formats are limited to the formats already mapped by the public
  Go viewer path: `GRAY8`, `GRAY16_LE`, `video/x-bayer` (`grbg`, `rggb`,
  `gbrg`, `bggr`), `RGB`, and `UYVY`.

## Explicit Non-Goals For v1

- No claim of conformance to the unfinished native EEVideo stream packet
  specification published by the upstream EEVideo spec project.
- No public CoAP/register-control API on the GStreamer elements.
- No JPEG transport.
- No resend, PTP/NTP timing profile, multicast tuning, FEC, or security profile.
- No dynamic mid-stream caps renegotiation.

## Source Of Truth

The wire-level behavior in this repo is intentionally aligned to the current Go
implementation, especially:

- the public `goeevideo` capture path
- the public `goeevideo` pixel-format mapping
- the public `eeview` GStreamer bridge

Where the prose spec and the public code differ, this repository follows the
public code.
