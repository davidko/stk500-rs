use bytes::{BufMut, BytesMut};
use futures::future::{Executor, Future};
use super::Command;
use super::tokio_io::{AsyncWrite, AsyncRead};
use super::tokio_io::codec::{Encoder, Decoder, Framed};
use std::io;
use std::sync::{Arc};
use tokio_core::reactor::{Handle};
use tokio_proto::pipeline::{ClientProto, ClientService};
use tokio_proto::{BindClient};
use tokio_service::{Service};

pub struct Packet {
    command: Command,
    payload: Vec<u8>,
}

pub struct Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    inner: ClientService<T, Stk500Proto>,
}

impl<T> Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    fn new(&self, handle: &Handle, io_transport: T) -> Client<T>
    {
        let proto = Stk500Proto;
        Client{ inner: proto.bind_client(handle, io_transport) }
    }
}

impl<T> Service for Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    type Request = Packet;
    type Response = BytesMut;
    type Error = io::Error;
    type Future = Box<Future<Item = BytesMut, Error = io::Error>>;

    fn call(&self, req: Packet) -> Self::Future {
        Box::new(self.inner.call(req))
    }
}

pub struct Stk500Codec {
    last_command: Command,
    expected_response_len: usize
}

impl Stk500Codec {
    fn new() -> Stk500Codec {
        Stk500Codec{last_command: Command::CmndStkGetSync, expected_response_len: 0}
    }
}

impl Encoder for Stk500Codec {
    type Item = Packet;
    type Error = io::Error;

    fn encode(
        &mut self, 
        item: Self::Item, 
        dst: &mut BytesMut
    ) -> Result<(), Self::Error> {
        self.last_command = item.command;
        self.expected_response_len = expected_response_len(&item);
        dst.put_u8(item.command as u8);
        dst.put(item.payload);
        dst.put(Command::SyncCrcEop as u8);
        Ok(())
    }
}

fn expected_response_len(packet: &Packet) -> usize {
    let command = packet.command;
    match command {
        Command::CmndStkGetSignOn => 9,
        Command::CmndStkGetParameter => 3,
        Command::CmndStkReadFlash => 4,
        Command::CmndStkReadData => 3,
        Command::CmndStkReadFuse => 4,
        Command::CmndStkReadFuseExt => 5,
        Command::CmndStkReadLock => 3,
        Command::CmndStkReadPage => {
            let bytes_high = packet.payload[0];
            let bytes_low = packet.payload[1];
            let bytes_len:usize = (bytes_high as usize) << 8 + (bytes_low as usize);
            bytes_len
        }
        Command::CmndStkReadSign => 5,
        Command::CmndStkReadOsccal => 3,
        Command::CmndStkReadOsccalExt => 3,
        Command::CmndStkUniversal => 3,
        _ => 2
    }
}

impl Decoder for Stk500Codec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        if src[0] != Command::RespStkInsync as u8 {
            Err(io::Error::new(io::ErrorKind::Other, "Stk500 Response Error: Expected first byte to be RespStkInsync"))
        }
        else if src.len() < self.expected_response_len {
            Ok(None)
        } else if src[self.expected_response_len-1] != Command::RespStkOk as u8 {
            Err(io::Error::new(io::ErrorKind::Other, "Stk500 Response Error: Expected last byte to be RespStkOk"))
        } else {
            let mut resp = src.split_to(self.expected_response_len);
            resp.split_to(1); // Get rid of the first byte
            let len = resp.len();
            resp.truncate( len - 1 );
            Ok(Some(resp))
        }
        
    }
}

struct Stk500Proto;

impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for Stk500Proto {
    type Request = Packet;
    type Response = BytesMut;
    type Transport = Framed<T, Stk500Codec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(Stk500Codec::new()))
    }
}

fn get_sync() -> Packet {
    Packet{command: Command::CmndStkGetSync, payload: vec![]}
}

fn set_device(payload: &Option<Vec<u8>>) -> Packet {
    let s = match *payload{
        Some(ref buf) => buf.clone(),
        None => vec![0x86, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01,
        0x03, 0xff, 0xff, 0xff, 0xff, 0x00, 0x80, 0x04, 0x00,
        0x00, 0x00, 0x80, 0x00]
    };
    Packet{command: Command::CmndStkSetDevice, payload: s}
}

fn set_device_ext(payload: &Option<Vec<u8>>) -> Packet {
    let s = match *payload{
        Some(ref buf) => buf.clone(),
        None => vec![0x05, 0x04, 0xd7, 0xc2, 0x00]
    };
    Packet{command: Command::CmndStkSetDeviceExt, payload: s}
}

/*
pub fn enter_prog_mode() -> Packet {
    Packet{ command: Command::CmndStkEnterProgMode, payload: vec![] }
}
*/
