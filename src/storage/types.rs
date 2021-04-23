use prost::Message;

#[derive(Message)]
pub struct Header {
    #[prost(fixed32, tag="1")]
    pub version: u32,
}
