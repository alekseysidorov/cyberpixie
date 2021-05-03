
#[derive(Debug)]
pub struct Adapter<Tx, Rx> {
    tx: Tx,
    rx: Rx,
}
