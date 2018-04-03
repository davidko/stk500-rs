use bytes::{BufMut, BytesMut};
use super::Command;
use super::tokio_io;
use std::io;

pub struct Packet {
    command: Command,
    payload: Vec<u8>,
}

pub struct Stk500Codec {
    last_command: Command,
    expected_response_len: usize
}

impl tokio_io::codec::Encoder for Stk500Codec {
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

impl tokio_io::codec::Decoder for Stk500Codec {
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
