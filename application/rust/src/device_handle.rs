use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use cyberpixie_proto::{FirmwareInfo, Hertz, Service};
use cyberpixie_std_transport::{display_err, TcpTransport};
use image::{
    imageops::{self, FilterType},
    io::Reader,
    RgbImage,
};
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
    setImage: qt_method!(fn(&mut self, index: usize)),
    uploadImage: qt_method!(fn(&mut self, path: QString, refresh_rate: usize)),
    clearImages: qt_method!(fn(&mut self)),
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

    fn setImage(&mut self, index: usize) {
        self.invoke(
            |s| s.inner.show_image(index),
            |s, _| {
                s.currentImage = index;

                s.currentImageChanged();
            },
        );
    }

    fn uploadImage(&mut self, image: QString, refresh_rate: usize) {
        let image_url = image.to_string();

        let image_path = image_url
            .strip_prefix("file://")
            .unwrap_or_else(|| image_url.as_str());
        let path = PathBuf::from(image_path);
        
        self.invoke(
            |s| {
                let refresh_rate = Hertz::from(refresh_rate as u32);
                let nwidth = s.stripLen as u32;
                s.inner.upload_image(&path, nwidth, refresh_rate)
            },
            |s, index| {
                s.imagesCount += 1;

                s.imageUploaded(index);
                s.imagesCountChanged();
            },
        );
    }

    fn clearImages(&mut self) {
        self.invoke(
            |s| s.inner.clear(),
            |s, _| {
                s.currentImage = 0;
                s.imagesCount = 0;

                s.currentImageChanged();
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
    fn upload_image(
        &self,
        image_path: &Path,
        nwidth: u32,
        refresh_rate: Hertz,
    ) -> anyhow::Result<usize> {
        let origin = open_image(image_path)?;

        let nheight = origin.height() * nwidth / origin.width();
        let resized = imageops::resize(&origin, nwidth, nheight, FilterType::Lanczos3);

        let raw = image_to_raw(&resized);

        log::debug!(
            "Uploading image with size {}x{} and refresh_rate: {}hz",
            nwidth,
            nheight,
            refresh_rate.0
        );
        self.add_image(nwidth as usize, refresh_rate, &raw)
    }

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

fn open_image(path: impl AsRef<Path>) -> anyhow::Result<RgbImage> {
    Ok(Reader::open(path)?.decode()?.to_rgb8())
}

fn image_to_raw(image: &RgbImage) -> Vec<u8> {
    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }
    raw
}
