use std::ffi::OsString;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, ValueEnum};
use eevideo_device::{
    CaptureBackend, CaptureConfiguration, DeviceRuntime, DeviceRuntimeConfig,
    SyntheticCaptureBackend, SyntheticCaptureConfig,
};
use eevideo_proto::{PixelFormat, VideoFrame};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum InputKind {
    Synthetic,
    Argus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceDaemonConfig {
    pub bind: SocketAddr,
    pub interface_name: Option<String>,
    pub advertise_address: Option<Ipv4Addr>,
    pub stream_name: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub mtu: u16,
    pub input: InputKind,
    pub sensor_id: u32,
}

impl Default for DeviceDaemonConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:5683".parse().expect("static socket address"),
            interface_name: None,
            advertise_address: None,
            stream_name: "stream0".to_string(),
            width: 1280,
            height: 720,
            fps: 30,
            mtu: 1200,
            input: InputKind::Synthetic,
            sensor_id: 0,
        }
    }
}

impl DeviceDaemonConfig {
    pub fn runtime_config(&self) -> DeviceRuntimeConfig {
        DeviceRuntimeConfig {
            bind: self.bind,
            interface_name: self.interface_name.clone(),
            advertise_address: self.advertise_address,
            stream_name: self.stream_name.clone(),
            width: self.width,
            height: self.height,
            pixel_format: PixelFormat::Uyvy,
            fps: self.fps,
            mtu: self.mtu,
            enforce_fixed_format: true,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "eedeviced", about = "Single-stream EEVideo device daemon")]
struct Cli {
    #[arg(long, default_value = "0.0.0.0:5683")]
    bind: SocketAddr,
    #[arg(long)]
    iface: Option<String>,
    #[arg(long)]
    advertise_address: Option<Ipv4Addr>,
    #[arg(long, default_value = "stream0")]
    stream_name: String,
    #[arg(long, default_value_t = 1280)]
    width: u32,
    #[arg(long, default_value_t = 720)]
    height: u32,
    #[arg(long, default_value_t = 30)]
    fps: u32,
    #[arg(long, default_value_t = 1200)]
    mtu: u16,
    #[arg(long, default_value = "synthetic")]
    input: InputKind,
    #[arg(long, default_value_t = 0)]
    sensor_id: u32,
}

impl From<Cli> for DeviceDaemonConfig {
    fn from(value: Cli) -> Self {
        Self {
            bind: value.bind,
            interface_name: value.iface,
            advertise_address: value.advertise_address,
            stream_name: value.stream_name,
            width: value.width,
            height: value.height,
            fps: value.fps,
            mtu: value.mtu,
            input: value.input,
            sensor_id: value.sensor_id,
        }
    }
}

pub struct DeviceDaemon {
    runtime: DeviceRuntime,
}

impl DeviceDaemon {
    pub fn spawn(config: DeviceDaemonConfig) -> Result<Self> {
        validate_config(&config)?;
        let runtime = match config.input {
            InputKind::Synthetic => DeviceRuntime::spawn(
                config.runtime_config(),
                SyntheticCaptureBackend::new(SyntheticCaptureConfig::default()),
            )?,
            InputKind::Argus => {
                DeviceRuntime::spawn(config.runtime_config(), ArgusCaptureBackend::new(config.sensor_id))?
            }
        };
        Ok(Self { runtime })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.runtime.local_addr()
    }

    pub fn uri(&self) -> String {
        self.runtime.uri()
    }

    pub fn shutdown(&mut self) {
        self.runtime.shutdown();
    }
}

pub fn main_entry<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    run(cli.into())
}

pub fn run(config: DeviceDaemonConfig) -> Result<()> {
    let device = DeviceDaemon::spawn(config)?;
    let (tx, rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })?;

    println!(
        "EEVideo device listening at {} advertising {}",
        device.local_addr(),
        device.uri()
    );
    println!("press Ctrl+C to stop");

    let _ = rx.recv();
    drop(device);
    Ok(())
}

fn validate_config(config: &DeviceDaemonConfig) -> Result<()> {
    if config.width == 0 || config.height == 0 {
        bail!("frame size must be non-zero");
    }
    if config.fps == 0 {
        bail!("fps must be greater than zero");
    }
    if config.width % 2 != 0 {
        bail!("UYVY device width must be even");
    }
    Ok(())
}

static GST_INIT: OnceLock<std::result::Result<(), String>> = OnceLock::new();

#[derive(Debug)]
struct ArgusCaptureState {
    pipeline: gst::Pipeline,
    sink: gst_app::AppSink,
    started_at: Instant,
    next_frame_id: u32,
    current_format: CaptureConfiguration,
}

#[derive(Debug)]
struct ArgusCaptureBackend {
    sensor_id: u32,
    state: Option<ArgusCaptureState>,
}

impl ArgusCaptureBackend {
    fn new(sensor_id: u32) -> Self {
        Self {
            sensor_id,
            state: None,
        }
    }
}

impl CaptureBackend for ArgusCaptureBackend {
    fn start_capture(&mut self, config: CaptureConfiguration) -> Result<()> {
        ensure_gstreamer_init()?;
        validate_argus_capture_config(&config)?;
        if self.state.is_some() {
            self.stop_capture()?;
        }

        let description = build_argus_pipeline_description(self.sensor_id, &config);
        let element = gst::parse::launch(&description)
            .with_context(|| format!("failed to build Argus pipeline: {description}"))?;
        let pipeline = element
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Argus pipeline description did not produce a gst::Pipeline"))?;
        let sink = pipeline
            .by_name("framesink")
            .ok_or_else(|| anyhow!("Argus pipeline does not expose framesink appsink"))?
            .downcast::<gst_app::AppSink>()
            .map_err(|_| anyhow!("Argus framesink element is not an appsink"))?;

        pipeline
            .set_state(gst::State::Playing)
            .context("failed to start Argus pipeline")?;
        self.state = Some(ArgusCaptureState {
            pipeline,
            sink,
            started_at: Instant::now(),
            next_frame_id: 1,
            current_format: config,
        });
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<()> {
        if let Some(state) = self.state.take() {
            state
                .pipeline
                .set_state(gst::State::Null)
                .context("failed to stop Argus pipeline")?;
        }
        Ok(())
    }

    fn next_frame(&mut self) -> Result<VideoFrame> {
        let state = self
            .state
            .as_mut()
            .ok_or_else(|| anyhow!("Argus capture is not running"))?;
        let sample = state
            .sink
            .try_pull_sample(gst::ClockTime::from_mseconds(200))
            .ok_or_else(|| anyhow!("timed out waiting for an Argus frame"))?;
        let buffer = sample
            .buffer_owned()
            .ok_or_else(|| anyhow!("Argus sample did not include a buffer"))?;
        let map = buffer
            .map_readable()
            .map_err(|_| anyhow!("failed to map Argus sample buffer"))?;
        let expected_len = state
            .current_format
            .pixel_format
            .payload_len(state.current_format.width, state.current_format.height)
            .context("invalid Argus capture format")?;
        if map.as_slice().len() != expected_len {
            bail!(
                "Argus frame payload length mismatch: expected {expected_len}, got {}",
                map.as_slice().len()
            );
        }

        let frame_id = state.next_frame_id;
        state.next_frame_id = state.next_frame_id.wrapping_add(1).max(1);
        let timestamp = buffer
            .pts()
            .map(|pts| pts.nseconds())
            .unwrap_or_else(|| {
                state
                    .started_at
                    .elapsed()
                    .as_nanos()
                    .min(u128::from(u64::MAX)) as u64
            });

        Ok(VideoFrame {
            frame_id,
            timestamp,
            width: state.current_format.width,
            height: state.current_format.height,
            pixel_format: state.current_format.pixel_format,
            payload_type: eevideo_proto::PayloadType::Image,
            data: map.as_slice().to_vec(),
        })
    }

    fn current_format(&self) -> Option<CaptureConfiguration> {
        self.state.as_ref().map(|state| state.current_format.clone())
    }
}

fn ensure_gstreamer_init() -> Result<()> {
    GST_INIT
        .get_or_init(|| gst::init().map_err(|err| err.to_string()))
        .clone()
        .map_err(anyhow::Error::msg)
}

fn validate_argus_capture_config(config: &CaptureConfiguration) -> Result<()> {
    if config.pixel_format != PixelFormat::Uyvy {
        bail!("Argus capture backend only supports UYVY output");
    }
    if config.width % 2 != 0 {
        bail!("Argus UYVY output width must be even");
    }
    Ok(())
}

fn build_argus_pipeline_description(sensor_id: u32, config: &CaptureConfiguration) -> String {
    format!(
        concat!(
            "nvarguscamerasrc sensor-id={sensor_id} ! ",
            "video/x-raw(memory:NVMM),width={width},height={height},framerate={fps}/1 ! ",
            "nvvidconv ! ",
            "video/x-raw,format=UYVY,width={width},height={height} ! ",
            "appsink name=framesink sync=false max-buffers=1 drop=true"
        ),
        sensor_id = sensor_id,
        width = config.width,
        height = config.height,
        fps = config.fps,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_argus_pipeline_description, CaptureConfiguration, DeviceDaemon, DeviceDaemonConfig,
        InputKind,
    };
    use eevideo_control::backend::{CoapRegisterBackend, CoapRegisterBackendConfig};
    use eevideo_control::{ControlBackend, ControlTarget, ControlTransportKind, RequestedStreamConfiguration};
    use eevideo_proto::{PayloadType, PixelFormat, StreamProfileId};
    use std::time::Duration;

    #[test]
    fn argus_pipeline_description_uses_expected_elements() {
        let description = build_argus_pipeline_description(
            2,
            &CaptureConfiguration {
                width: 1280,
                height: 720,
                pixel_format: PixelFormat::Uyvy,
                fps: 30,
            },
        );

        assert!(description.contains("nvarguscamerasrc sensor-id=2"));
        assert!(description.contains("video/x-raw(memory:NVMM),width=1280,height=720,framerate=30/1"));
        assert!(description.contains("nvvidconv"));
        assert!(description.contains("video/x-raw,format=UYVY,width=1280,height=720"));
        assert!(description.contains("appsink name=framesink"));
    }

    #[test]
    fn synthetic_device_uses_fixed_uyvy_format() {
        let device = DeviceDaemon::spawn(DeviceDaemonConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            input: InputKind::Synthetic,
            width: 640,
            height: 480,
            ..DeviceDaemonConfig::default()
        })
        .unwrap();

        let backend = CoapRegisterBackend::new(CoapRegisterBackendConfig {
            request_timeout: Duration::from_millis(250),
            ..CoapRegisterBackendConfig::default()
        });
        let target = ControlTarget {
            device_uri: device.uri(),
            transport_kind: ControlTransportKind::CoapRegister,
            auth_scope: None,
        };
        let mut connection = backend.connect(&target).unwrap();
        let applied = connection
            .configure(RequestedStreamConfiguration {
                stream_name: "stream0".to_string(),
                profile: StreamProfileId::CompatibilityV1,
                destination_host: "127.0.0.1".to_string(),
                port: 5000,
                bind_address: "127.0.0.1".to_string(),
                packet_delay_ns: 0,
                max_packet_size: 1200,
                format: Some(eevideo_control::StreamFormatDescriptor {
                    payload_type: PayloadType::Image,
                    pixel_format: PixelFormat::Uyvy,
                    width: 640,
                    height: 480,
                }),
            })
            .unwrap();
        assert_eq!(applied.format.unwrap().pixel_format, PixelFormat::Uyvy);

        let error = connection
            .configure(RequestedStreamConfiguration {
                stream_name: "stream0".to_string(),
                profile: StreamProfileId::CompatibilityV1,
                destination_host: "127.0.0.1".to_string(),
                port: 5000,
                bind_address: "127.0.0.1".to_string(),
                packet_delay_ns: 0,
                max_packet_size: 1200,
                format: Some(eevideo_control::StreamFormatDescriptor {
                    payload_type: PayloadType::Image,
                    pixel_format: PixelFormat::Rgb8,
                    width: 640,
                    height: 480,
                }),
            })
            .unwrap_err();
        assert_eq!(error.kind(), eevideo_control::ControlErrorKind::AppliedValueMismatch);
    }
}
