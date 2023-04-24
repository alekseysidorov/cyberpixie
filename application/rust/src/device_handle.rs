use std::{
    net::SocketAddr,
    ops::DerefMut,
    path::{Path, PathBuf},
};

use cyberpixie_core::proto::types::{ImageId, PeerInfo, Hertz};
use cyberpixie_std_network::{connect_to, display_err, Client};
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
    busy: qt_property!(bool; NOTIFY busyChanged),

    // Signals part.
    imageUploaded: qt_signal!(index: usize),
    error: qt_signal!(message: QString),

    stripLenChanged: qt_signal!(),
    imagesCountChanged: qt_signal!(),
    currentImageChanged: qt_signal!(),
    busyChanged: qt_signal!(),

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
            move |inner| inner.device_info(),
            move |s, value| {
                let device_info = value.device_info.unwrap();
                s.stripLen = device_info.strip_len as usize;
                s.imagesCount = device_info.images_count as usize;

                s.stripLenChanged();
                s.imagesCountChanged();
            },
        );
    }

    fn setImage(&mut self, index: usize) {
        self.invoke(
            move |inner| inner.show_image(index),
            move |s, _| {
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
        let nwidth = self.stripLen as u32;
        self.invoke(
            move |inner| {
                let refresh_rate = Hertz::from(refresh_rate as u16);
                inner.upload_image(&path, nwidth, refresh_rate)
            },
            move |s, index| {
                s.imagesCount += 1;

                s.imageUploaded(index);
                s.imagesCountChanged();
            },
        );
    }

    fn clearImages(&mut self) {
        self.invoke(
            move |inner| inner.clear(),
            move |s, _| {
                s.currentImage = 0;
                s.imagesCount = 0;

                s.currentImageChanged();
                s.imagesCountChanged();
            },
        );
    }

    fn invoke<F, R, T>(&mut self, method: F, then: T)
    where
        F: FnOnce(DeviceHandleInner) -> anyhow::Result<R> + Send + 'static,
        T: FnOnce(&mut Self, R) + Send + 'static + Copy,
        R: Send + 'static,
    {
        self.busy = true;
        self.busyChanged();

        let qptr = QPointer::from(&*self);

        let set_value = qmetaobject::queued_callback(move |value: anyhow::Result<R>| {
            if let Some(this) = qptr.as_pinned() {
                let mut ref_mut = this.borrow_mut();
                match value {
                    Ok(value) => then(ref_mut.deref_mut(), value),
                    Err(err) => {
                        let err_str = err.to_string();
                        ref_mut.deref_mut().error(err_str.into());
                    }
                }

                ref_mut.busy = false;
                ref_mut.busyChanged();
            }
        });

        let inner = self.inner;
        std::thread::spawn(move || {
            let value = method(inner);
            set_value(value);
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct DeviceHandleInner {
    address: SocketAddr,
}

impl Default for DeviceHandleInner {
    fn default() -> Self {
        Self {
            address: SocketAddr::new([192, 168, 71, 1].into(), 333),
        }
    }
}

impl DeviceHandleInner {
    fn upload_image(
        self,
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

    fn cyberpixie_client(self) -> anyhow::Result<Client> {
        let stream = connect_to(&self.address)?;
        Client::connect(stream)
    }

    fn device_info(self) -> anyhow::Result<PeerInfo> {
        self.cyberpixie_client()?.handshake().map_err(display_err)
    }

    fn show_image(self, index: usize) -> anyhow::Result<()> {
        self.cyberpixie_client()?
            .show_image(ImageId(index as u16))?
            .map_err(display_err)
    }

    fn clear(self) -> anyhow::Result<()> {
        self.cyberpixie_client()?
            .clear_images()?
            .map_err(display_err)
    }

    fn add_image(
        self,
        strip_len: usize,
        refresh_rate: Hertz,
        bytes: &[u8],
    ) -> anyhow::Result<usize> {
        assert!(
            bytes.len() % 3 == 0,
            "Bytes amount to read must be a multiple of 3."
        );

        self.cyberpixie_client()?
            .add_image(refresh_rate, strip_len, bytes)?
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
