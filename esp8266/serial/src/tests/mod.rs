use std::fmt::Write;

use embedded_hal::prelude::*;
use serial::EmbeddedSerial;

mod serial;

#[test]
fn test_connect() -> anyhow::Result<()> {
    let port = serialport::new("/dev/ttyUSB0", 115200).open()?;
    let mut serial = EmbeddedSerial::new(port);

    serial.write_str("AT+GMR\r\n")?;
    std::thread::sleep(std::time::Duration::from_secs(1));

    let mut buffer = [0; 512];
    let mut idx = 0;
    loop {
        let sym = nb::block!(serial.read())?;
        buffer[idx] = sym;
        idx += 1;

        if sym == b'\n' {
            eprintln!("{}", String::from_utf8(buffer[0..idx].to_vec()).unwrap());
            idx = 0;
        }
    }
    
    Ok(())
}
