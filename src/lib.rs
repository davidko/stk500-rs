extern crate bytes;
extern crate futures;
extern crate futures_timer;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;
#[macro_use] extern crate log;

use futures::Future;
use futures::future::{loop_fn, Loop};
use futures::sync::oneshot;

use std::sync::{Arc, Mutex};
use std::time::Duration;

pub mod codec;
pub use codec::{Stk500Codec, Client};

pub type Response = Box<Future<Item=Vec<u8>, Error=oneshot::Canceled>>;

#[derive(Clone, Copy)]
pub enum Command {
        RespStkOk = 0x10,
        RespStkFailed = 0x11,
        RespStkUnknown = 0x12,
        RespStkNodevice = 0x13,
        RespStkInsync = 0x14,
        RespStkNosync = 0x15,

        RespAdcChannelError = 0x16,
        RespAdcMeasureOk = 0x17,
        RespPwmChannelError = 0x18,
        RespPwmAdjustOk = 0x19,

//# *****************[ STK Special Constants ]***************************

        SyncCrcEop = 0x20,

//# *****************[ STK Command Constants ]***************************

        CmndStkGetSync = 0x30,
        CmndStkGetSignOn = 0x31,

        CmndStkSetParameter = 0x40,
        CmndStkGetParameter = 0x41,
        CmndStkSetDevice = 0x42,
        CmndStkSetDeviceExt = 0x45,

        CmndStkEnterProgmode = 0x50,
        CmndStkLeaveProgmode = 0x51,
        CmndStkChipErase = 0x52,
        CmndStkCheckAutoinc = 0x53,
        CmndStkLoadAddress = 0x55,
        CmndStkUniversal = 0x56,
        CmndStkUniversalMulti = 0x57,

        CmndStkProgFlash = 0x60,
        CmndStkProgData = 0x61,
        CmndStkProgFuse = 0x62,
        CmndStkProgLock = 0x63,
        CmndStkProgPage = 0x64,
        CmndStkProgFuseExt = 0x65,

        CmndStkReadFlash = 0x70,
        CmndStkReadData = 0x71,
        CmndStkReadFuse = 0x72,
        CmndStkReadLock = 0x73,
        CmndStkReadPage = 0x74,
        CmndStkReadSign = 0x75,
        CmndStkReadOsccal = 0x76,
        CmndStkReadFuseExt = 0x77,
        CmndStkReadOsccalExt = 0x78,

//# *****************[ STK Parameter Constants ]***************************

        ParmStkHwVer = 0x80,
        ParmStkSwMajor = 0x81,
        ParmStkSwMinor = 0x82,
        ParmStkLeds = 0x83,
        ParmStkVtarget = 0x84,
        ParmStkVadjust = 0x85,
        ParmStkOscPscale = 0x86,
        ParmStkOscCmatch = 0x87,
        ParmStkResetDuration = 0x88,
        ParmStkSckDuration = 0x89,

        ParmStkBufsizel = 0x90,
        ParmStkBufsizeh = 0x91,
        ParmStkDevice = 0x92,
        ParmStkProgmode = 0x93,
        ParmStkParamode = 0x94,
        ParmStkPolling = 0x95,
        ParmStkSelftimed = 0x96,
}

pub enum StkStatus {


//# *****************[ STK status bit definitions ]***************************

        StatStkInsync = 0x01,
        StatStkProgmode = 0x02,
        StatStkStandalone = 0x04,
        StatStkReset = 0x08,
        StatStkProgram = 0x10,
        StatStkLedg = 0x20,
        StatStkLedr = 0x40,
        StatStkLedblink = 0x80,


//# *****************************[ End Of COMMAND.H ]**************************
}

enum State {
    Idle,
    WaitResponse,
}

pub struct Programmer {
    inner: Arc<Mutex<Inner>>
}

unsafe impl Send for Programmer {}
unsafe impl Sync for Programmer {}

impl Programmer {
    pub fn new() -> Programmer {
        Programmer { 
            inner: Arc::new(Mutex::new(Inner::new()))
        }
    }

	pub fn set_write_cb<F>(&mut self, callback: F) 
        where F: Fn(Vec<u8>),
              F: 'static
    {
        self.inner.lock().unwrap().set_write_cb(callback)
    }

    pub fn deliver(&mut self, buf: Vec<u8>) {
        self.inner.lock().unwrap().deliver(buf)
    }

    pub fn get_sync(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.inner.lock().unwrap().get_sync()
    }

    pub fn sign_on(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.inner.lock().unwrap().sign_on()
    }

    pub fn read_sign(&mut self) -> Response {
        debug!("Programmer::read_sign()");
        self.inner.lock().unwrap().read_sign()
    }

    pub fn set_device(&mut self, settings: &Option<Vec<u8>>) -> Response {
        self.inner.lock().unwrap().set_device(settings)
    }

    pub fn set_device_ext(&mut self, settings: &Option<Vec<u8>>) -> Response {
        self.inner.lock().unwrap().set_device_ext(settings)
    }

    pub fn enter_prog_mode(&mut self) -> Response {
        self.inner.lock().unwrap().enter_prog_mode()
    }

    pub fn load_address(&mut self, address: u16) -> Response {
        self.inner.lock().unwrap().load_address(address)
    }

    pub fn prog_page(&mut self, mem_type: char, data: &Vec<u8>) -> Response {
        self.inner.lock().unwrap().prog_page(mem_type, data)
    }

    pub fn prog_memory(&mut self, mem_type: char, page_size: usize, word_size: usize, data: Vec<u8>) -> 
        Box< Future<Item=Vec<u8>, Error=oneshot::Canceled> >
    {
        let p = self.inner.clone();
        let p2 = self.inner.clone();
        let p3 = self.inner.clone();
        let p4 = self.inner.clone();
        let p5 = self.inner.clone();
        let p6 = self.inner.clone();
        let p7 = self.inner.clone();
        let p8 = self.inner.clone();
        let f = p.lock().unwrap().get_sync().and_then(move |_| {
            let mut inner = p2.lock().unwrap();
            inner.set_device(&None)
        }).and_then(move |_| {
            let mut inner = p3.lock().unwrap();
            inner.set_device_ext(&None)
        }).and_then(move |_| {
            let mut inner = p4.lock().unwrap();
            inner.enter_prog_mode()
        }).and_then(move |_| {
            let mut inner = p5.lock().unwrap();
            inner.read_sign()
        }).and_then(move |_| {
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
                let p6 = p6.clone();
                let p7 = p7.clone();
                futures_timer::Delay::new(Duration::from_millis(50)).map_err(|_|{oneshot::Canceled}).and_then( move |_| {
                    let mut inner = p6.lock().unwrap();
                    inner.load_address((index / word_size) as u16)
                }).and_then(move |_| {
                    let mut inner = p7.lock().unwrap();
                    let ref page = _data[index..end];
                    inner.prog_page(mem_type, &page.to_vec())
                }).and_then(move |_| {
                    if end < __data.len() {
                        Ok(Loop::Continue(end))
                    } else {
                        Ok(Loop::Break(end))
                    }
                })
            })
        }).and_then(move |_| {
            let mut inner = p8.lock().unwrap();
            inner.leave_prog_mode()
        });
        Box::new(f)
    }
}

struct Inner {
    write_cb: Option< Box<Fn(Vec<u8>)> >, // Function which writes to the microcontroller being programmed
    buffer: Vec<u8>,
    state: State,
    waiting_future: Option<oneshot::Sender<Vec<u8>>>,
}

impl Inner {
    pub fn new() -> Inner {
        debug!("Programmer::new()");
        Inner { 
            write_cb: None, 
            buffer: Vec::new() ,
            state: State::Idle,
            waiting_future: None
        }
    }

	pub fn set_write_cb<F>(&mut self, callback: F) 
        where F: Fn(Vec<u8>),
              F: 'static
    {
        self.write_cb = Some(Box::new(callback));
    }

    pub fn deliver(&mut self, buf: Vec<u8>) {
        debug!("deliver() received {} bytes", buf.len());
        self.buffer.extend(buf.iter());
        debug!("Current deliver buffer size: {}", self.buffer.len());
        if self.buffer.len() < 2 {
            debug!("Buffer too small. Wait for more data.");
            return;
        }
        if let Some(byte) = self.buffer.last() {
            if *byte != Command::RespStkOk as u8 {
                debug!("Sentinal byte not received. Waiting for more data...");
                return;
            }
        }
        let byte = *self.buffer.first().unwrap();
        if byte != Command::RespStkInsync as u8 {
            debug!("Buffer header byte incorrect. Resetting internal buffer...");
            self.buffer.clear();
			let _maybe_sender = self.waiting_future.take();
            self.state = State::Idle;
            return;
        }
        match self.state {
            State::Idle => {
            }
            State::WaitResponse => {
                let maybe_sender = self.waiting_future.take();
                if let Some(sender) = maybe_sender {
                    sender.send(self.buffer.split_off(0)).unwrap();
                }
                self.state = State::Idle;
                self.buffer.clear();
            }
        }
    }

    fn send_command(&mut self, buf: &Vec<u8>) -> Response {
        self.state = State::WaitResponse;
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        self.waiting_future = Some(tx);
        if let Some(ref cb) = self.write_cb {
            let mut _buf = buf.clone();
            _buf.push(Command::SyncCrcEop as u8);
            debug!("Programmer::send_command: {} bytes.", _buf.len());
            cb(_buf);
        }
        Box::new(rx)
    }

    pub fn get_sync(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.send_command(&vec![Command::CmndStkGetSync as u8])
    }

    pub fn sign_on(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.send_command(&vec![Command::CmndStkGetSignOn as u8])
    }

    pub fn read_sign(&mut self) -> Response {
        debug!("Programmer::read_sign()");
        self.send_command(&vec![Command::CmndStkReadSign as u8])
    }

    pub fn set_device(&mut self, settings: &Option<Vec<u8>>) -> Response {
        debug!("Programmer::set_device()");
        let s = match *settings {
            Some(ref buf) => buf.clone(),
            None => vec![0x86, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01,
                0x03, 0xff, 0xff, 0xff, 0xff, 0x00, 0x80, 0x04, 0x00,
                0x00, 0x00, 0x80, 0x00]
        };
        let mut command = vec![Command::CmndStkSetDevice as u8];
        command.extend(s);
        self.send_command(&command)
    }

    pub fn set_device_ext(&mut self, settings: &Option<Vec<u8>>) -> Response {
        debug!("Programmer::set_device_ext()");
        let s = match *settings {
            Some(ref buf) => buf.clone(),
            None => vec![0x05, 0x04, 0xd7, 0xc2, 0x00]
        };
        let mut command = vec![Command::CmndStkSetDeviceExt as u8];
        command.extend(s);
        self.send_command(&command)
    }

    pub fn enter_prog_mode(&mut self) -> Response {
        debug!("Programmer::enter_prog_mode()");
        self.send_command(&vec![Command::CmndStkEnterProgmode as u8])
    }

    pub fn load_address(&mut self, address: u16) -> Response {
        debug!("Programmer::load_address()");
        let mut command = vec![Command::CmndStkLoadAddress as u8];
        command.push( (address & 0x00ff) as u8 );
        command.push( (address>>8 & 0x00ff) as u8 );
        self.send_command(&command)
    }

    pub fn prog_page(&mut self, mem_type: char, data: &Vec<u8>) -> Response {
        debug!("Programmer::prog_page()");
        let mut command = vec![Command::CmndStkProgPage as u8];
        let size = data.len() as u16;
        command.push( ((size>>8) & 0x00ff) as u8 );
        command.push( (size & 0x00ff) as u8 );
        command.push( mem_type as u8 );
        command.extend(data);
        self.send_command(&command)
    }

    pub fn leave_prog_mode(&mut self) -> Response {
        debug!("Programmer::leave_prog_mode()");
        self.send_command(&vec![Command::CmndStkLeaveProgmode as u8])
    }
}

#[derive(Debug)]
pub enum StkError {
    ParseHexFileError,
}

impl From<std::num::ParseIntError> for StkError {
    fn from(_: std::num::ParseIntError) -> StkError {
        StkError::ParseHexFileError
    }
}

pub fn hex_to_buffer(hex_string: &String) -> Result<Vec<u8>, StkError> {
    fn string_to_u32(s: Option<&str>) -> Result<u32, StkError> {
        match s {
            None => {
                return Err(StkError::ParseHexFileError)
            }
            Some(digits) => {
                let num = u32::from_str_radix(digits, 16)?;
                Ok(num)
            }
        }
    }

    fn process_line(line: &str) -> Result<(u32, u32, u32), StkError>
    {
        // Returns byte_count, addr, and record_type
        let byte_count = string_to_u32(line.get(1..3))?;
        let addr = string_to_u32(line.get(3..7))?;
        let record_type = string_to_u32(line.get(7..9))?;
        Ok((byte_count, addr, record_type))
    }
    // Calculate the maximum size of the firmware file
    let mut base_address = 0;
    let mut size = 0;
    for line in hex_string.lines() {
        let (byte_count, addr, record_type) = process_line(line)?;
        if record_type == 2 {
            let ext = string_to_u32(line.get(9..((9+byte_count*2) as usize)))?;
            base_address = ext*16;
        } else if record_type == 4 {
            let ext = string_to_u32(line.get(9..((9+byte_count*2) as usize)))?;
            base_address = ext << 16;
        } else if record_type == 0 {
            let new_size = addr + base_address + byte_count;
            if new_size > size {
                size = new_size;
            }
        }
    }

    let mut buffer:Vec<u8> = vec![0xff; size as usize];

    base_address = 0;
    for line in hex_string.lines() {
        let (byte_count, addr, record_type) = process_line(line)?;
        if record_type == 2 {
            let ext = string_to_u32(line.get(9..((9+byte_count*2) as usize)))?;
            base_address = ext*16;
        } else if record_type == 4 {
            let ext = string_to_u32(line.get(9..((9+byte_count*2) as usize)))?;
            base_address = ext << 16;
        } else if record_type == 0 {
            let _addr = addr + base_address;
            for i in 0..byte_count {
                let b = string_to_u32(line.get( ((9+i*2) as usize) .. ((11+(i*2)) as usize)))?;
                buffer[(_addr + i) as usize] = b as u8;
            }
        }
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use futures::Future;
    use super::Client;
    use std::fs::File;
    use std::io::{Read};

    #[test]
    fn async_test() {
        extern crate tokio_serial;
        extern crate tokio_core;

        let mut core = tokio_core::reactor::Core::new().unwrap();
        let handle = core.handle();
        {
            let mut f = File::open("/share/linkbot-firmware/v4.6.1.hex").expect("Firmware file not found.");
            let mut eeprom = File::open("/share/linkbot-firmware/v4.6.1.eeprom").expect("Firmware file not found.");

            let mut contents = String::new();
            let mut eeprom_contents = String::new();

            f.read_to_string(&mut contents).expect("Error reading firmware file.");
            eeprom.read_to_string(&mut eeprom_contents).expect("Error reading eeprom file.");

            let buf = super::hex_to_buffer(&contents).unwrap();
            let eeprom_buf = super::hex_to_buffer(&eeprom_contents).unwrap();
           
            let mut settings = tokio_serial::SerialPortSettings::default();
            settings.baud_rate = tokio_serial::BaudRate::Baud57600;
            let mut port = tokio_serial::Serial::from_path("/dev/ttyACM0", &settings, &handle).unwrap();
            port.set_exclusive(false).unwrap();
            let client = Client::new(&handle, port);
            let _client = client.clone();
            let task = client.prog_memory('F', 0x0100, 2, buf)
                .and_then(move |_| {
                    _client.prog_memory('E', 0x0100, 2, eeprom_buf)
                });
            core.run(task).unwrap();
        }
        // Let everything fall out of scope and see if I can open the serial port again.
        let mut settings = tokio_serial::SerialPortSettings::default();
        settings.baud_rate = tokio_serial::BaudRate::Baud115200;
        let port = tokio_serial::Serial::from_path("/dev/ttyACM0", &settings, &handle).unwrap();
    }
}
