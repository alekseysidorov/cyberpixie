use core::fmt::Debug;

use cyberpixie_proto::{Message, PacketReader, PayloadError, Service, ServiceEvent};
use embedded_hal::serial::{Read, Write};
use esp8266_softap::{BytesIter, Event, SoftAp};

pub fn into_service<Rx, Tx>(ap: SoftAp<Rx, Tx>) -> impl Service<Address = usize>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    Rx::Error: Debug,
    Tx::Error: Debug,
{
    ServiceImpl(ap)
}

#[derive(Debug)]
enum Error<R: Debug, W: Debug> {
    Read(R),
    Write(W),
    Payload(PayloadError),
}

struct ServiceImpl<Rx, Tx>(SoftAp<Rx, Tx>)
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    Rx::Error: Debug,
    Tx::Error: Debug;

impl<Rx, Tx> Service for ServiceImpl<Rx, Tx>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    Rx::Error: Debug,
    Tx::Error: Debug,
{
    type Error = Error<Rx::Error, Tx::Error>;

    type Address = usize;
    type BytesReader<'a> = BytesIter<'a, Rx>;

    fn poll_next(
        &mut self,
    ) -> nb::Result<ServiceEvent<Self::Address, Self::BytesReader<'_>>, Self::Error> {
        let event = self
            .0
            .poll_next_event()
            .map_err(|x| x.map(Self::Error::Read))?;

        Ok(match event {
            Event::Connected { link_id } => ServiceEvent::Connected { address: link_id },
            Event::Closed { link_id } => ServiceEvent::Disconnected { address: link_id },
            Event::DataAvailable {
                link_id,
                mut reader,
            } => {
                let mut packet_reader = PacketReader::default();
                let (header_len, payload_len) = packet_reader.read_message_len(&mut reader);

                let bytes = BytesIter::new(link_id, reader, payload_len + header_len);
                let msg = packet_reader
                    .read_message(bytes, header_len)
                    .map_err(Self::Error::Payload)?;

                ServiceEvent::Data {
                    address: link_id,
                    payload: msg,
                }
            }
        })
    }

    fn send_message<I>(&mut self, to: Self::Address, message: Message<I>) -> Result<(), Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        Ok(())
    }
}
