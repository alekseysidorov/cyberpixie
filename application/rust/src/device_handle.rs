use std::net::SocketAddr;

use cyberpixie_proto::{FirmwareInfo, Hertz, Service};
use cyberpixie_std_transport::{display_err, TcpTransport};
use qmetaobject::prelude::*;

#[allow(non_snake_case)]
#[derive(Default, QObject)]
pub struct DeviceHandle {
    // Real implementation.
    inner: DeviceHandleInner,

    // Binding to Qml.
    base: qt_base_class!(trait QObject),

    // Properties
    stripLen: qt_property!(usize; NOTIFY stripLenChanged),
    imagesCount: qt_property!(usize; NOTIFY imagesCountChanged),
    currentImage: qt_property!(usize; NOTIFY currentImageChanged),

    // Signals part.
    imageUploaded: qt_signal!(index: usize),
    error: qt_signal!(message: QString),

    stripLenChanged: qt_signal!(),
    imagesCountChanged: qt_signal!(),
    currentImageChanged: qt_signal!(),

    // Qt methods
    deviceInfo: qt_method!(fn(&mut self)),
}

#[allow(non_snake_case)]
impl DeviceHandle {
    fn deviceInfo(&mut self) {
        self.invoke(
            |s| s.inner.device_info(),
            |s, value| {
                s.stripLen = value.strip_len as usize;
                s.imagesCount = value.images_count as usize;

                s.stripLenChanged();
                s.imagesCountChanged();
            },
        );
    }

    fn invoke<F, R, T>(&mut self, method: F, then: T)
    where
        F: Fn(&Self) -> anyhow::Result<R>,
        T: Fn(&mut Self, R),
    {
        match method(self) {
            Ok(value) => then(self, value),
            Err(err) => {
                let err_str = err.to_string();
                self.error(err_str.into());
            }
        }
    }
}

struct DeviceHandleInner {
    address: SocketAddr,
}

impl Default for DeviceHandleInner {
    fn default() -> Self {
        Self {
            address: SocketAddr::new([192, 168, 4, 1].into(), 333),
        }
    }
}

impl DeviceHandleInner {
    fn cyberpixie_service(&self) -> anyhow::Result<Service<TcpTransport>> {
        cyberpixie_std_transport::create_service(self.address)
    }

    fn device_info(&self) -> anyhow::Result<FirmwareInfo> {
        self.cyberpixie_service()?
            .request_firmware_info(self.address)?
            .map_err(display_err)
    }

    fn show_image(&self, index: usize) -> anyhow::Result<()> {
        self.cyberpixie_service()?
            .show_image(self.address, index)?
            .map_err(display_err)
    }

    fn clear(&self) -> anyhow::Result<()> {
        self.cyberpixie_service()?
            .clear_images(self.address)?
            .map_err(display_err)
    }

    fn add_image(
        &self,
        strip_len: usize,
        refresh_rate: Hertz,
        bytes: &[u8],
    ) -> anyhow::Result<usize> {
        self.cyberpixie_service()?
            .add_image(self.address, refresh_rate, strip_len, bytes.iter().copied())?
            .map_err(display_err)
    }
}
