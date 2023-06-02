use std::ops::DerefMut;

use cyberpixie_network::{
    core::proto::types::{Hertz, ImageId, PeerInfo},
    SocketAddr,
};
use image::{
    imageops::{self, FilterType},
    RgbImage,
};
use qmetaobject::prelude::*;
use std_embedded_nal::Stack;

type Client = cyberpixie_network::blocking::Client<Stack>;

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
    uploadImage: qt_method!(fn(&mut self, content: QByteArray, refresh_rate: usize)),
    stop: qt_method!(fn(&mut self)),
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
                s.imagesCount = device_info.images_count.0 as usize;

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

    fn uploadImage(&mut self, image: QByteArray, refresh_rate: usize) {
        let image_bytes = image.to_slice().to_owned();
        let nwidth = self.stripLen as u32;
        self.invoke(
            move |inner| {
                let refresh_rate = Hertz(refresh_rate as u32);
                inner.upload_image(&image_bytes, nwidth, refresh_rate)
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

    fn stop(&mut self) {
        self.invoke(move |inner| inner.stop(), move |_, _| {});
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

#[derive(Clone, Copy)]
struct DeviceHandleInner {
    address: SocketAddr,
}

impl Default for DeviceHandleInner {
    fn default() -> Self {
        Self {
            address: SocketAddr::new([192, 168, 71, 1].into(), 1800),
        }
    }
}

impl DeviceHandleInner {
    fn upload_image(
        self,
        buffer: &[u8],
        nwidth: u32,
        refresh_rate: Hertz,
    ) -> anyhow::Result<usize> {
        let origin = image::load_from_memory(buffer)?.to_rgb8();

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
        Client::connect(&mut Stack::default(), self.address).map_err(anyhow::Error::from)
    }

    fn device_info(self) -> anyhow::Result<PeerInfo> {
        self.cyberpixie_client()?
            .peer_info(&mut Stack::default())
            .map_err(anyhow::Error::from)
    }

    fn show_image(self, index: usize) -> anyhow::Result<()> {
        self.cyberpixie_client()?
            .start(&mut Stack::default(), ImageId(index as u16))
            .map_err(anyhow::Error::from)
    }

    fn stop(self) -> anyhow::Result<()> {
        self.cyberpixie_client()?
            .stop(&mut Stack::default())
            .map_err(anyhow::Error::from)
    }

    fn clear(self) -> anyhow::Result<()> {
        self.cyberpixie_client()?
            .clear_images(&mut Stack::default())
            .map_err(anyhow::Error::from)
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

        let id = self.cyberpixie_client()?.add_image(
            &mut Stack::default(),
            refresh_rate,
            strip_len as u16,
            bytes,
        )?;
        Ok(id.0 as usize)
    }
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
