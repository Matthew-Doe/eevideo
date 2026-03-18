# Jetson Demo Video Design

## Goal

Create a short product demo video for a potential robotics or computer-vision
developer.

The video's core message is:

`eevideo` provides simple discovery, control, and live viewing for a networked
video device.

## Audience

- robotics developers
- computer-vision developers
- technical evaluators who want to see value quickly

They are likely comfortable with terminals and device workflows, but they do
not want a long build or setup walkthrough in the video.

## Demo Context

- real device: existing Jetson Nano
- the Jetson must already be running `eedeviced`
- the device must already be advertising one stable compatible stream before
  recording starts
- host machine records the terminal and viewer workflow
- target runtime: 30 to 45 seconds

The demo should present the Jetson as a live network video source that is easy
to find, inspect, and view from a workstation.

For this short video, assume the host-side tools are already built and
available as `eevid` and `eeview` on `PATH`, or wrapped by shell aliases that
hide `cargo run`. Do not show build steps during the recording.

## Recommended Approach

Use a "find device, start stream, see video" narrative.

Why this approach:

- it reaches the payoff quickly
- it matches the stated value proposition
- it avoids overloading the viewer with implementation detail

## Story Arc

1. Introduce the product promise in one line.
2. Show the host discovering the Jetson on the network.
3. Show a concise description of the device and stream.
4. Launch managed live viewing, making it clear that this step performs the
   stream setup for the host.
5. Hold on the live feed long enough for the viewer to feel the payoff.

## Shot Plan

### Shot 1: Title card

Duration: 2 to 3 seconds

Suggested caption:

`EEVideo: discover, control, and view a Jetson video device`

### Shot 2: Discovery

Duration: 5 to 8 seconds

Show a host terminal running:

```sh
eevid discover
```

Desired outcome:

- the Jetson device appears clearly in terminal output
- the device URI is readable enough to reuse in the next command

### Shot 3: Describe

Duration: 6 to 8 seconds

Show a host terminal running:

```sh
eevid --device-uri coap://<jetson-ip>:5683 describe
```

The visible output should make at least these points legible:

- device identity
- one available stream
- supported profile or stream mode summary

Do not linger on register listings if they make the output noisy.

### Shot 4: Live view

Duration: 12 to 15 seconds

Show the host launching:

```sh
eeview --device-uri coap://<jetson-ip>:5683 --bind-address <host-ip> --port 5000
```

Then cut to or reveal the live viewer window.

This is the visible control step in the demo: `eeview` is not just displaying a
local window, it is also telling the device where to send the stream and then
starting managed viewing on the host.

The live scene should include visible motion so the audience immediately reads
it as a real camera feed.

### Shot 5: Closing hold

Duration: 4 to 6 seconds

Hold on the live stream with a short caption such as:

`Discover. Inspect. View.`

or

`Simple host-side discovery, control, and live viewing.`

## Capture Guidelines

- keep terminal font large
- keep the desktop uncluttered
- pre-stage commands to minimize typing noise
- use a real, visually obvious camera scene
- prefer one clean terminal window plus the viewer window
- make the `eeview` command line fully readable, especially `--bind-address`
- avoid showing long setup or build steps

If possible, place an object or hand movement in frame during the live-view
segment so the viewer immediately trusts that the stream is live.

## Messaging Guardrails

Emphasize:

- simple network discovery
- lightweight control and inspection
- live viewing from the host

Avoid emphasizing:

- transport internals
- packet format details
- broad production-readiness claims
- unsupported hardware generalizations
- latency or performance claims unless they are demonstrated and measured

## Success Criteria

The demo is successful if a viewer can understand, within one watch, that:

- a Jetson device is discoverable over the network
- the host can query it with simple commands
- the host can open a live view with minimal friction

## Open Preparation Items

Before recording, confirm:

- the Jetson Nano is already configured, reachable, and running `eedeviced`
- the device is already advertising the exact stream mode you want to show
- the camera path is stable on the device
- the host machine can discover the device reliably
- the host `--bind-address` is the real reachable IP of the receiving NIC, not
  `127.0.0.1`
- the host IP and Jetson IP are known in advance
- if discovery is noisy on the network, the direct device URI is prepared in
  advance
- the live scene looks good on camera
