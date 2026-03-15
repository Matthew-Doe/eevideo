use std::sync::Arc;

use crate::backend::{
    local_bind_addr, parse_device_endpoint, CoapRegisterBackend, CoapRegisterBackendConfig,
};
use crate::register::RegisterClient;
use crate::register_map::{
    read_register_field, read_register_value, register_name, resolve_stream_prefix,
    stream_prefixes, write_register_fields, write_register_u32, FieldUpdate, RegisterSelector,
    RegisterValue,
};
use crate::yaml::DeviceConfig;
use crate::{
    AdvertisedStream, AdvertisedStreamMode, AppliedStreamConfiguration, ControlBackend,
    ControlCapabilities, ControlError, ControlErrorKind, ControlSession, ControlTarget,
    ControlTransportKind, DiscoveredDevice, RequestedStreamConfiguration, RunningStream,
    SharedControlBackend,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceSummary {
    pub target: ControlTarget,
    pub interface_name: String,
    pub interface_address: String,
    pub device_address: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceDescription {
    pub summary: DeviceSummary,
    pub capabilities: ControlCapabilities,
    pub device: DeviceConfig,
    pub streams: Vec<AdvertisedStream>,
}

#[derive(Clone, Debug)]
pub struct DeviceController {
    backend: CoapRegisterBackend,
}

impl DeviceController {
    pub fn new(config: CoapRegisterBackendConfig) -> Self {
        Self {
            backend: CoapRegisterBackend::new(config),
        }
    }

    pub fn backend(&self) -> &CoapRegisterBackend {
        &self.backend
    }

    pub fn shared_backend(&self) -> SharedControlBackend {
        Arc::new(self.backend.clone())
    }

    pub fn discover(
        &self,
        device_uri_filter: Option<&str>,
    ) -> Result<Vec<DeviceSummary>, ControlError> {
        let target = ControlTarget {
            device_uri: device_uri_filter.unwrap_or_default().to_string(),
            transport_kind: ControlTransportKind::CoapRegister,
            auth_scope: None,
        };
        let devices = self.backend.discover(&target)?;
        Ok(devices.into_iter().map(summary_from_discovered).collect())
    }

    pub fn describe(&self, target: &ControlTarget) -> Result<DeviceDescription, ControlError> {
        let (client, device) = self.client_and_device(target, None)?;
        let capabilities = self.backend.connect(target)?.describe()?;
        let streams = read_advertised_streams(&client, &device)?;

        Ok(DeviceDescription {
            summary: DeviceSummary {
                target: target.clone(),
                interface_name: device.location.interface_name.clone(),
                interface_address: device.location.interface_address.clone(),
                device_address: device.location.device_address.clone(),
            },
            capabilities,
            streams,
            device,
        })
    }

    pub fn read_register(
        &self,
        target: &ControlTarget,
        bind_address: Option<&str>,
        selector: &RegisterSelector,
    ) -> Result<RegisterValue, ControlError> {
        let (client, device) = self.client_and_device(target, bind_address)?;
        read_register_value(&client, &device, selector)
    }

    pub fn write_register(
        &self,
        target: &ControlTarget,
        bind_address: Option<&str>,
        selector: &RegisterSelector,
        value: u32,
    ) -> Result<(), ControlError> {
        let (client, device) = self.client_and_device(target, bind_address)?;
        write_register_u32(&client, &device, selector, value)
    }

    pub fn read_field(
        &self,
        target: &ControlTarget,
        bind_address: Option<&str>,
        selector: &RegisterSelector,
        field_name: &str,
    ) -> Result<u32, ControlError> {
        let (client, device) = self.client_and_device(target, bind_address)?;
        read_register_field(&client, &device, selector, field_name)
    }

    pub fn write_field(
        &self,
        target: &ControlTarget,
        bind_address: Option<&str>,
        selector: &RegisterSelector,
        field_name: &str,
        value: u32,
    ) -> Result<(), ControlError> {
        let (client, device) = self.client_and_device(target, bind_address)?;
        write_register_fields(
            &client,
            &device,
            selector,
            &[FieldUpdate::new(field_name, value)],
        )
    }

    pub fn configure_stream(
        &self,
        target: &ControlTarget,
        request: RequestedStreamConfiguration,
    ) -> Result<AppliedStreamConfiguration, ControlError> {
        let mut session = ControlSession::new(self.shared_backend(), target.clone(), request);
        let requested = session.requested().clone();
        session.configure(requested)
    }

    pub fn start_stream(
        &self,
        target: &ControlTarget,
        request: RequestedStreamConfiguration,
    ) -> Result<RunningStream, ControlError> {
        let mut session = ControlSession::new(self.shared_backend(), target.clone(), request);
        session.start()
    }

    pub fn stop_stream(
        &self,
        target: &ControlTarget,
        stream_name: &str,
        bind_address: Option<&str>,
    ) -> Result<(), ControlError> {
        let (client, device) = self.client_and_device(target, bind_address)?;
        let prefix = resolve_stream_prefix(&device, stream_name)?;
        write_register_fields(
            &client,
            &device,
            &RegisterSelector::name(register_name(&prefix, "MaxPacketSize")),
            &[FieldUpdate::new("enable", 0)],
        )
    }

    fn client_and_device(
        &self,
        target: &ControlTarget,
        bind_address: Option<&str>,
    ) -> Result<(RegisterClient, DeviceConfig), ControlError> {
        let endpoint = parse_device_endpoint(&target.device_uri)?;
        let device = self.backend.load_or_create_device_config(&endpoint)?;
        let client = RegisterClient::new(
            local_bind_addr(
                bind_address,
                self.backend.config().local_port,
                endpoint.addr,
            ),
            endpoint.addr,
        )
        .with_timeout(self.backend.config().request_timeout);
        Ok((client, device))
    }
}

fn summary_from_discovered(device: DiscoveredDevice) -> DeviceSummary {
    DeviceSummary {
        target: ControlTarget {
            device_uri: device.device_uri,
            transport_kind: device.transport_kind,
            auth_scope: device.auth_scope,
        },
        interface_name: device.interface_name,
        interface_address: device.interface_address,
        device_address: device.device_address,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use eefakedev::{FakeDeviceConfig, FakeDeviceServer};
    use eevideo_proto::PixelFormat;
    use tempfile::tempdir;

    use crate::register::RegisterClient;
    use crate::{CoapRegisterBackendConfig, ControlTarget, ControlTransportKind};

    use super::DeviceController;

    const STREAM_WIDTH_ADDR: u32 = 0x0004_001c;
    const STREAM_HEIGHT_ADDR: u32 = 0x0004_0020;

    #[test]
    fn describe_reads_live_stream_mode_when_yaml_cache_exists() {
        let cache_dir = tempdir().unwrap();
        let device = FakeDeviceServer::spawn(FakeDeviceConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            width: 32,
            height: 16,
            pixel_format: PixelFormat::Mono8,
            fps: 24,
            ..FakeDeviceConfig::default()
        })
        .unwrap();
        let controller = DeviceController::new(CoapRegisterBackendConfig {
            request_timeout: Duration::from_millis(250),
            yaml_root: Some(cache_dir.path().to_path_buf()),
            ..CoapRegisterBackendConfig::default()
        });
        let target = ControlTarget {
            device_uri: device.uri(),
            transport_kind: ControlTransportKind::CoapRegister,
            auth_scope: None,
        };

        let initial = controller.describe(&target).unwrap();
        let initial_mode = initial.streams[0].mode.as_ref().unwrap();
        assert_eq!(initial_mode.width, 32);
        assert_eq!(initial_mode.height, 16);
        assert_eq!(initial_mode.fps, 24);

        let client = RegisterClient::new("127.0.0.1:0".parse().unwrap(), device.local_addr())
            .with_timeout(Duration::from_millis(250));
        client.write_u32(STREAM_WIDTH_ADDR, 48).unwrap();
        client.write_u32(STREAM_HEIGHT_ADDR, 24).unwrap();

        let refreshed = controller.describe(&target).unwrap();
        let refreshed_mode = refreshed.streams[0].mode.as_ref().unwrap();
        assert_eq!(refreshed_mode.width, 48);
        assert_eq!(refreshed_mode.height, 24);
        assert_eq!(refreshed_mode.fps, 24);
    }
}

fn read_advertised_streams(
    client: &RegisterClient,
    device: &DeviceConfig,
) -> Result<Vec<AdvertisedStream>, ControlError> {
    stream_prefixes(device)
        .into_iter()
        .map(|prefix| {
            Ok(AdvertisedStream {
                name: prefix.clone(),
                mode: read_advertised_stream_mode(client, device, &prefix)?,
            })
        })
        .collect()
}

fn read_advertised_stream_mode(
    client: &RegisterClient,
    device: &DeviceConfig,
    prefix: &str,
) -> Result<Option<AdvertisedStreamMode>, ControlError> {
    let Some(width) = maybe_read_stream_field(client, device, prefix, "PixelsPerLine", "ppl")?
    else {
        return Ok(None);
    };
    let Some(height) = maybe_read_stream_field(client, device, prefix, "LinesPerFrame", "lpf")?
    else {
        return Ok(None);
    };
    let Some(pixel_format_bits) =
        maybe_read_stream_field(client, device, prefix, "PixelFormat", "bpp")?
    else {
        return Ok(None);
    };
    let Some(fps) = maybe_read_stream_field(client, device, prefix, "FramesPerSecond", "fps")?
    else {
        return Ok(None);
    };

    if width == 0 || height == 0 || pixel_format_bits == 0 || fps == 0 {
        return Ok(None);
    }

    let pixel_format = pixel_format_from_device_bits(pixel_format_bits).ok_or_else(|| {
        ControlError::new(
            ControlErrorKind::InvalidConfiguration,
            format!("device reported unsupported pixel format value 0x{pixel_format_bits:08x}"),
        )
    })?;

    Ok(Some(AdvertisedStreamMode {
        pixel_format,
        width,
        height,
        fps,
    }))
}

fn maybe_read_stream_field(
    client: &RegisterClient,
    device: &DeviceConfig,
    prefix: &str,
    register_suffix: &str,
    field_name: &str,
) -> Result<Option<u32>, ControlError> {
    match read_register_field(
        client,
        device,
        &RegisterSelector::name(register_name(prefix, register_suffix)),
        field_name,
    ) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == ControlErrorKind::InvalidConfiguration => Ok(None),
        Err(error) => Err(error),
    }
}

fn pixel_format_from_device_bits(value: u32) -> Option<eevideo_proto::PixelFormat> {
    use eevideo_proto::PixelFormat;

    PixelFormat::from_pfnc(value).ok().or_else(|| {
        [
            PixelFormat::Mono8,
            PixelFormat::Mono16,
            PixelFormat::BayerGr8,
            PixelFormat::BayerRg8,
            PixelFormat::BayerGb8,
            PixelFormat::BayerBg8,
            PixelFormat::Rgb8,
            PixelFormat::Uyvy,
        ]
        .into_iter()
        .find(|format| format.pfnc() & 0xffff == value)
    })
}
