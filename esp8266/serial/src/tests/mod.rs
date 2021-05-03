use embedded_hal::serial::{Read, Write};
use serial::EmbeddedSerial;

use crate::adapter::Adapter;

mod serial;

fn print_at_cmd<Rx, Tx>(adapter: &mut Adapter<Rx, Tx>, cmd: impl AsRef<[u8]>)
where
    Rx: Read<u8>,
    Tx: Write<u8>,
{
    let cmd = cmd.as_ref();
    let res = adapter.send_at_command(cmd).unwrap();
    eprintln!("-> {}", String::from_utf8_lossy(cmd));
    match res {
        Ok(msg) => eprint!("ok: {}", String::from_utf8_lossy(msg)),
        Err(msg) => eprint!("err: {}", String::from_utf8_lossy(msg)),
    }
}

#[test]
fn test_connect() {
    let port = serialport::new("/dev/ttyUSB0", 115200).open().unwrap();
    let (rx, tx) = EmbeddedSerial::new(port).into_rx_tx();

    let mut adapter = Adapter::new(rx, tx).unwrap();
    print_at_cmd(&mut adapter, "AT+GMR");
    print_at_cmd(&mut adapter, "AT+CWSAP=\"ESP\",\"12345678\",5,3");
    print_at_cmd(&mut adapter, "AT+CWMODE=3");
    print_at_cmd(&mut adapter, "AT+CIPMUX=1");
    print_at_cmd(&mut adapter, "AT+CIPSERVER=1");
}
