use cyberpixie_esp32c3::DefaultStorage;
use esp_idf_svc::log::EspLogger;
use esp_idf_sys as _;
use log::info; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let mut storage = DefaultStorage::take()?;
    // Write some string to the storage
    storage.set("test", &"Hello world!")?;

    let s: String = storage.get("test")?.unwrap();
    info!("test content: `{}`", s);

    for i in 1..=80 {
        let len = 256 * i;

        let big_buf = vec![1_u8; len];
        storage.set("big_buf", &big_buf)?;
        drop(big_buf);

        let big_buf: Vec<u8> = storage.get("big_buf")?.unwrap();
        info!("test big_buf len: `{}`", big_buf.len());
        drop(big_buf);

        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    Ok(())
}
