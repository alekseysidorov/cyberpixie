use cyberpixie_proto::Service;

use crate::tcp_transport::TransportImpl;

pub struct App {
    pub service: Service<TransportImpl>,
}

impl App {
    pub async fn new(service: Service<TransportImpl>) -> Result<Self, anyhow::Error> {
        Ok(Self { service })
    }
}
