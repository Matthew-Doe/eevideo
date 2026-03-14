# Jetson Orin First-Time EEVideo Setup

This guide is for the first time you turn a Jetson Orin into an EEVideo device
with `eedeviced`.

Use it when you want the full setup path from a fresh board to a working first
stream. If you already have binaries on the Jetson, you can skip directly to
Step 3 for install and startup, or Step 5 for host verification. If you need
the provider matrix and CLI reference, use
[eedeviced-provider-guide.md](eedeviced-provider-guide.md).
If you are bringing up Jetson Nano on JetPack 4.x, use
[jetson-nano-jetpack4-first-time-setup.md](jetson-nano-jetpack4-first-time-setup.md).

For Jetson bring-up in this repo, the recommended path is building directly on
the Jetson and running `--input pipeline` with an explicit
`nvarguscamerasrc ... ! appsink` pipeline. The built-in `argus` provider
remains available in the CLI, but it is not currently a tested deployment path
here. The cross-build helpers are kept as a fallback, not the recommended
workflow.

## What You Need

- a Jetson Orin running JetPack 6.x
- a CSI camera that works with `nvarguscamerasrc`
- a second machine that will run `eevid` and `eeview`
- a network path between the Jetson and the host
- this repository checked out on the Jetson
- this repository checked out on the host, or prebuilt `eevid` and `eeview`
  binaries there

Recommended first setup:

- one Jetson
- one host PC
- one camera
- one Ethernet link
- unicast only
- `1280x720@30 UYVY`

## What You Are Building

The first device path in this repo is:

- `eedeviced` on the Jetson
- CoAP/register discovery and control on port `5683`
- one stream named `stream0`
- `CompatibilityV1` transport
- `UYVY` output

The host-side tools stay the same:

- `eevid` for discovery and stream control
- `eeview` for managed live viewing

## Step 1: Prepare The Jetson

Confirm the board is on JetPack 6.x and that the camera stack is alive.

On the Jetson:

```sh
gst-launch-1.0 nvarguscamerasrc sensor-id=0 ! fakesink
```

If that fails, stop here and fix the Jetson camera setup first. `eedeviced`
depends on the same camera stack through the explicit pipeline used below.

Decide these values before continuing:

- Jetson IP address you want the host to use
- network interface name on the Jetson, for example `eth0`
- camera sensor id, usually `0` for the first sensor

## Step 2: Build The Artifacts On The Jetson

Build directly on the Jetson for the recommended path. Cross-building with
`cross/jetson-orin/build.sh` exists in the repo, but it is not the recommended
Jetson bring-up flow.

On the Jetson, from a checkout of this repository:

```sh
cargo build --release -p eedeviced
```

The output lands under:

```text
target/release/
```

For first setup, use these local files on the Jetson:

- `target/release/eedeviced`
- `cross/jetson-orin/systemd/eedeviced.service`
- `cross/jetson-orin/systemd/eedeviced-launch.sh`
- `cross/jetson-orin/systemd/eedeviced.env.example`

## Step 3: Install Files On The Jetson

Example:

```sh
sudo mkdir -p /opt/eevideo /etc/eevideo
sudo cp target/release/eedeviced /opt/eevideo/
sudo cp cross/jetson-orin/systemd/eedeviced.service /etc/systemd/system/
sudo cp cross/jetson-orin/systemd/eedeviced-launch.sh /opt/eevideo/
sudo cp cross/jetson-orin/systemd/eedeviced.env.example /etc/eevideo/eedeviced.env
sudo chmod +x /opt/eevideo/eedeviced-launch.sh
```

## Step 4: Start The Device Manually First

Do not start with `systemd`. Run it manually once so failures are obvious.

On the Jetson:

```sh
gst-launch-1.0 nvarguscamerasrc sensor-id=0 ! \
  'video/x-raw(memory:NVMM),format=NV12,width=1280,height=720,framerate=30/1' ! \
  nvvidconv ! \
  'video/x-raw,format=UYVY,width=1280,height=720' ! \
  fakesink
```

If that fails, fix the local GStreamer path before debugging EEVideo control.

Then start `eedeviced`:

```sh
./target/release/eedeviced \
  --bind 0.0.0.0:5683 \
  --advertise-address 192.168.1.50 \
  --iface eth0 \
  --input pipeline \
  --pixel-format uyvy \
  --width 1280 \
  --height 720 \
  --fps 30 \
  --mtu 1200 \
  --pipeline "nvarguscamerasrc sensor-id=0 ! video/x-raw(memory:NVMM),format=NV12,width=1280,height=720,framerate=30/1 ! nvvidconv ! video/x-raw,format=UYVY,width=1280,height=720 ! appsink name=framesink sync=false max-buffers=1 drop=true"
```

What each flag is doing:

- `--bind`: listens for discovery and register control
- `--advertise-address`: the IP address the host should connect to
- `--iface`: the Jetson NIC used for device discovery context
- `--input pipeline`: uses the operator-owned GStreamer path
- `--pixel-format uyvy`: must match the final appsink caps
- `--width`, `--height`, `--fps`: fixed first stream mode
- `--mtu`: UDP payload limit for the stream
- `--pipeline`: owns the full CSI capture pipeline ending in `appsink name=framesink`

Keep that process running while you validate from the host.

## Step 5: Verify From The Host

First check discovery:

```sh
cargo run -p eevid -- discover
```

If discovery is noisy on your network, use the direct URI:

```sh
cargo run -p eevid -- --device-uri coap://192.168.1.50:5683 describe
```

You should see:

- one device
- one stream named `stream0`
- `compatibility-v1`
- `stream stream0: UYVY 1280x720 @ 30 fps`

Then start managed viewing from the host:

```sh
cargo run -p eeview -- --device-uri coap://192.168.1.50:5683 --bind-address 192.168.1.20 --port 5000
```

That command tells the Jetson where to send the stream, then starts the local
receiver/viewer. The viewer HUD shows live FPS + stream mode by default; add
`--no-overlay` if you want the video without the overlay.

## Step 6: If You Want A Control-Only Smoke First

Before using `eeview`, you can verify that the Jetson accepts stream control:

```sh
cargo run -p eevid -- --device-uri coap://192.168.1.50:5683 stream-start --stream-name stream0 --destination-host 192.168.1.20 --port 5000 --bind-address 192.168.1.20 --max-packet-size 1200 --width 1280 --height 720 --pixel-format uyvy
```

Expected result:

- `running stream-id=... active=true`

## Step 7: Install As A Service After Manual Success

Once manual startup is stable, use the packaged service files:

- `cross/jetson-orin/systemd/eedeviced.service`
- `cross/jetson-orin/systemd/eedeviced-launch.sh`
- `cross/jetson-orin/systemd/eedeviced.env.example`

Edit `/etc/eevideo/eedeviced.env`, then enable the service:

```sh
EEVIDEO_BIND=0.0.0.0:5683
EEVIDEO_ADVERTISE_ADDRESS=192.168.1.50
EEVIDEO_IFACE=eth0
EEVIDEO_INPUT=pipeline
EEVIDEO_PIXEL_FORMAT=uyvy
EEVIDEO_WIDTH=1280
EEVIDEO_HEIGHT=720
EEVIDEO_FPS=30
EEVIDEO_MTU=1200
EEVIDEO_PIPELINE=nvarguscamerasrc sensor-id=0 ! video/x-raw(memory:NVMM),format=NV12,width=1280,height=720,framerate=30/1 ! nvvidconv ! video/x-raw,format=UYVY,width=1280,height=720 ! appsink name=framesink sync=false max-buffers=1 drop=true
```

Then enable the service:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now eedeviced
```

Keep `EEVIDEO_PIXEL_FORMAT=uyvy` for the current Jetson CSI path. The packaged
service now passes pixel format explicitly instead of relying on CLI defaults.

Check status:

```sh
sudo systemctl status eedeviced
journalctl -u eedeviced -f
```

## Common First-Time Problems

If `eevid discover` finds nothing:

- confirm `eedeviced` is still running
- confirm the Jetson IP matches `--advertise-address`
- confirm host and Jetson are on the same reachable network
- try `eevid --device-uri coap://JETSON_IP:5683 describe`

If `eeview` starts but no frames arrive:

- keep `mtu` at `1200`
- use unicast first
- confirm the host `--bind-address` is the host’s real NIC address, not `0.0.0.0`
- verify the full Jetson pipeline works with `gst-launch-1.0 ... ! nvvidconv ! ... ! fakesink`

If the device rejects stream settings:

- `eedeviced` intentionally keeps the first setup fixed to one `UYVY` mode
- use `1280x720`, `30`, and `uyvy` first
- avoid format changes until the base path is stable

## After The First Successful Stream

Once the first setup works, the next things worth validating are:

- repeated start/stop cycles
- boot-time startup through `systemd`
- a higher `mtu` on a known-good LAN

If you choose to experiment with the built-in `argus` provider later, treat it
as an unvalidated path and keep the explicit `pipeline` provider as the default
for production or troubleshooting.
