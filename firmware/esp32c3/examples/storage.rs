use std::time::Instant;

use cyberpixie_esp32c3::DefaultStorage;
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use log::info;

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

    for i in 1..=40 {
        let name = format!("buf.{i}");
        let len = 256 * i;

        let big_buf = vec![1_u8; len];
        storage.set(&name, &big_buf)?;
        drop(big_buf);

        let strip_len = 144;
        let times = 100;
        let time = Instant::now();
        for _ in 0..times {
            let big_buf: Vec<u8> = storage.get(&name)?.unwrap();
            assert_eq!(len, big_buf.len());
            drop(big_buf);
        }

        let bps = (times * len) as f64 / time.elapsed().as_secs_f64();
        info!(
            "test big_buf len: `{}`, throughput {:.2} KB/sec, {:.4} lines/sec [{strip_len}]",
            len,
            bps / 1024.0,
            bps / (3 * strip_len) as f64,
        );
        storage.remove(&name)?;

        std::thread::sleep(std::time::Duration::from_millis(250));
    }

    Ok(())
}
