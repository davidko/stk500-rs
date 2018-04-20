use bytes::{BytesMut};
use futures::future::{Either, Future, loop_fn, Loop};
use futures_timer;
use super::Command;
use super::tokio_io::{AsyncWrite, AsyncRead};
use super::tokio_io::codec::{Encoder, Decoder, Framed};
use std::io;
use std::sync::{Arc};
use std::time::Duration;
use tokio_core;
use tokio_core::reactor::{Handle};
use tokio_proto::pipeline::{ClientProto, ClientService};
use tokio_proto::{BindClient};
use tokio_service::{Service};

pub type ResponseFuture = Box<Future<Item = BytesMut, Error = io::Error>>;

pub struct Packet {
    command: Command,
    payload: Vec<u8>,
}

/// An Stk500 Client Service
pub struct Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    inner: Arc<Inner<T>>
}

impl<T> Clone for Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    fn clone(&self) -> Client<T> {
        Client{ inner: self.inner.clone() }
    }
}

impl<T> Client<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    pub fn new(handle: &Handle, io_transport: T) -> Client<T> {
        Client{ 
            inner: Arc::new( Inner::new(handle, io_transport) )
        }
    }
    
    pub fn get_sync(&self) -> ResponseFuture {
        debug!("get_sync()");
        self.inner.get_sync()
    }

    pub fn set_device(&self, payload: &Option<Vec<u8>>) -> ResponseFuture {
        debug!("set_device()");
        self.inner.set_device(payload)
    }

    pub fn set_device_ext(&self, payload: &Option<Vec<u8>>) -> ResponseFuture {
        self.inner.set_device_ext(payload)
    }

    pub fn enter_prog_mode(&self) -> ResponseFuture {
        self.inner.enter_prog_mode()
    }

    pub fn read_sign(&self) -> ResponseFuture {
        self.inner.read_sign()
    }

    pub fn load_address(&self, address: u16) -> ResponseFuture {
        self.inner.load_address(address)
    }

    pub fn prog_page(&self, mem_type: char, data: &Vec<u8>) -> ResponseFuture {
        self.inner.prog_page(mem_type, data)
    }

    pub fn prog_memory(&self, mem_type: char, page_size: usize, word_size: usize, data: Vec<u8>)
        -> ResponseFuture
    {
        let step0 = self.get_sync();

        let inner = self.inner.clone();
        let step1 = move |_| {
            inner.set_device(&None)
        };

        let inner = self.inner.clone();
        let step2 = move |_| {
            inner.set_device_ext(&None)
        };

        let inner = self.inner.clone();
        let step3 = move |_| {
            inner.enter_prog_mode()
        };
       
        let inner = self.inner.clone();
        let step4 = move |_| {
            inner.read_sign()
        };

        let inner = self.inner.clone();
        let step5 = move |_| {
            fn check_program(buf: &Vec<u8>) -> bool {
                buf.iter().any(|&x| {
                    x != 0xff
                })
            }
            
            loop_fn(0 as usize, move |mut index| {
                let mut end = index + page_size;
                if end >= data.len() {
                    end = data.len();
                }
                let mut page = &data[index..end];
                let _data = data.clone();
                let __data = data.clone();
                while check_program(&page.to_vec()) == false {
                    index = end;
                    if index > data.len() {
                        break;
                    }
                    end = index + page_size;
                    if end >= data.len() {
                        end = data.len();
                    }
                    page = &data[index..end];
                }
                let _inner = inner.clone();
                let __inner = inner.clone();
                futures_timer::Delay::new(Duration::from_millis(50))
                    .map_err(|_|{io::Error::new(io::ErrorKind::Other, "Timeout")})
                    .and_then( move |_| {
                    _inner.load_address((index / word_size) as u16)
                }).and_then(move |_| {
                    let ref page = _data[index..end];
                    __inner.prog_page(mem_type, &page.to_vec())
                }).and_then(move |_| {
                    if end < __data.len() {
                        Ok(Loop::Continue(end))
                    } else {
                        Ok(Loop::Break(end))
                    }
                })
            })
        };

        let inner = self.inner.clone();
        let step6 = move |_| {
            inner.leave_prog_mode()
        };

        let f = step0
            .and_then( step1 )
            .and_then( step2 )
            .and_then( step3 )
            .and_then( step4 )
            .and_then( step5 )
            .and_then( step6 );

        Box::new(f)
    }
}

struct Inner<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    inner: Timeout<ClientService<T, Stk500Proto>>,
}

impl<T> Inner<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    fn new(handle: &Handle, io_transport: T) -> Inner<T> {
        let proto = Stk500Proto;
        let inner_service = proto.bind_client(handle, io_transport);
        let service = Timeout::new(
            inner_service,
            Duration::from_millis(500),
            handle);
        Inner{ inner: service }
    }

    fn get_sync(&self) -> ResponseFuture {
        let packet = Packet { command: Command::CmndStkGetSync, payload: vec![] };
        self.call(packet)
    }

    fn set_device(&self, payload: &Option<Vec<u8>>) -> ResponseFuture {
        let p = match *payload{
            Some(ref buf) => buf.clone(),
            None => vec![0x86, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01,
            0x03, 0xff, 0xff, 0xff, 0xff, 0x00, 0x80, 0x04, 0x00,
            0x00, 0x00, 0x80, 0x00]
        };
        let packet = Packet{command: Command::CmndStkSetDevice, payload: p};
        self.call(packet)
    }

    fn set_device_ext(&self, payload: &Option<Vec<u8>>) -> ResponseFuture {
        let p = match *payload{
            Some(ref buf) => buf.clone(),
            None => vec![0x05, 0x04, 0xd7, 0xc2, 0x00]
        };
        let packet = Packet{command: Command::CmndStkSetDeviceExt, payload: p};
        self.call(packet)
    }

    fn enter_prog_mode(&self) -> ResponseFuture {
        self.call(
            Packet{ command: Command::CmndStkEnterProgmode, payload: vec![] }
        )
    }

    fn read_sign(&self) -> ResponseFuture {
        self.call(
            Packet{ command: Command::CmndStkReadSign, payload: vec![] }
        )
    }

    fn load_address(&self, address: u16) -> ResponseFuture {
        self.call(
            Packet{ 
                command: Command::CmndStkLoadAddress,
                payload: vec![ 
                    (address & 0x00ff) as u8,
                    (address>>8 & 0x00ff) as u8 ]
            }
        )
    }

    fn prog_page(&self, mem_type: char, data: &Vec<u8>) -> ResponseFuture {
        let size = data.len() as u16;
        let mut payload = vec![ (size>>8) as u8, (size&0x00ff) as u8, mem_type as u8];
        payload.extend(data);
        self.call( Packet{ command: Command::CmndStkProgPage, payload: payload } )
    }

    fn leave_prog_mode(&self) -> ResponseFuture {
        self.call( Packet{ command: Command::CmndStkLeaveProgmode, payload: vec![] } )
    }
}

impl<T> Service for Inner<T>
    where T: AsyncRead + AsyncWrite + 'static
{
    type Request = Packet;
    type Response = BytesMut;
    type Error = io::Error;
    type Future = ResponseFuture;

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
        dst.extend([item.command as u8].iter());
        dst.extend_from_slice(&item.payload.as_slice());
        dst.extend([Command::SyncCrcEop as u8].iter());
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

        if src.len() < self.expected_response_len {
            Ok(None)
        } 
        else if src[0] != Command::RespStkInsync as u8 {
            Err(io::Error::new(io::ErrorKind::Other, "Stk500 Response Error: Expected first byte to be RespStkInsync"))
        }
        else if src[self.expected_response_len-1] != Command::RespStkOk as u8 {
            Err(io::Error::new(io::ErrorKind::Other, "Stk500 Response Error: Expected last byte to be RespStkOk"))
        } 
        else {
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


/*
pub fn enter_prog_mode() -> Packet {
    Packet{ command: Command::CmndStkEnterProgMode, payload: vec![] }
}
*/

struct Timeout<T> {
    delay: Duration,
    upstream: T,
    handle: Handle,
}

impl<T> Timeout<T> {
    fn new(upstream: T, delay: Duration, handle: &Handle) -> Timeout<T> {
        Timeout{
            delay: delay,
            upstream: upstream,
            handle: handle.clone()
        }
    }
}

impl<T> Service for Timeout<T> 
    where T: Service,
          T::Error: From<io::Error> + 'static,
          T::Request: 'static,
          T::Response: 'static,
          T::Future: 'static,
{
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let timeout = tokio_core::reactor::Timeout::new(self.delay, &self.handle).unwrap();

        let work = self.upstream.call(req).select2(timeout).then(|res| {
            match res {
                Ok(Either::A((item, _timeout))) => Ok(item),
                Ok(Either::B((_timeout_error, _item))) => {
                    Err(Self::Error::from(io::Error::new( io::ErrorKind::Other, "Timeout.")))
                }
                Err(Either::A((item_error, _timeout))) => {
                    Err(item_error)
                }
                Err(Either::B((timeout_error, _item))) => Err(Self::Error::from(timeout_error))
            }
        });

        Box::new(work)

    }
}
