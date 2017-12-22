extern crate futures;
#[macro_use] extern crate log;

use futures::Future;
use futures::sync::oneshot;

pub type Response = Box<Future<Item=Vec<u8>, Error=oneshot::Canceled>>;

enum Commands {
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

enum StkStatus {


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

struct Programmer {
    write_cb: Option< Box<Fn(Vec<u8>)> >, // Function which writes to the microcontroller being programmed
    buffer: Vec<u8>,
    state: State,
    waiting_future: Option<oneshot::Sender<Vec<u8>>>,
}

unsafe impl Send for Programmer {}
unsafe impl Sync for Programmer {}

impl Programmer {
    pub fn new() -> Programmer {
        debug!("Programmer::new()");
        Programmer { 
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
            if *byte != Commands::RespStkOk as u8 {
                debug!("Sentinal byte not received. Waiting for more data...");
                return;
            }
        }
        let byte = *self.buffer.first().unwrap();
        if byte != Commands::RespStkInsync as u8 {
            debug!("Buffer header byte incorrect. Resetting internal buffer...");
            self.buffer.clear();
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
            }
        }
    }

    fn send_command(&mut self, buf: &Vec<u8>) -> Response {
        self.state = State::WaitResponse;
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        self.waiting_future = Some(tx);
        if let Some(ref cb) = self.write_cb {
            let mut _buf = buf.clone();
            _buf.push(Commands::SyncCrcEop as u8);
            debug!("Programmer::send_command: {} bytes.", _buf.len());
            cb(_buf);
        }
        Box::new(rx)
    }

    pub fn get_sync(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.send_command(&vec![Commands::CmndStkGetSync as u8])
    }

    pub fn sign_on(&mut self) -> Response {
        debug!("Programmer::sign_on()");
        self.send_command(&vec![Commands::CmndStkGetSignOn as u8])
    }

    pub fn read_sign(&mut self) -> Response {
        debug!("Programmer::read_sign()");
        self.send_command(&vec![Commands::CmndStkReadSign as u8])
    }

    pub fn set_device(&mut self, settings: &Option<Vec<u8>>) -> Response {
        let s = match *settings {
            Some(ref buf) => buf.clone(),
            None => vec![0x86, 0x00, 0x00, 0x01, 0x01, 0x01, 0x01,
                0x03, 0xff, 0xff, 0xff, 0xff, 0x00, 0x80, 0x04, 0x00,
                0x00, 0x00, 0x80, 0x00]
        };
        let mut command = vec![Commands::CmndStkSetDevice as u8];
        command.extend(s);
        self.send_command(&command)
    }

    pub fn set_device_ext(&mut self, settings: &Option<Vec<u8>>) -> Response {
        let s = match *settings {
            Some(ref buf) => buf.clone(),
            None => vec![0x05, 0x04, 0xd7, 0xc2, 0x00]
        };
        let mut command = vec![Commands::CmndStkSetDevice as u8];
        command.extend(s);
        self.send_command(&command)
    }

    pub fn enter_prog_mode(&mut self) -> Response {
        self.send_command(&vec![Commands::CmndStkEnterProgmode as u8])
    }

    pub fn load_address(&mut self, address: u16) -> Response {
        let mut command = vec![Commands::CmndStkLoadAddress as u8];
        command.push( (address & 0x00ff) as u8 );
        command.push( (address>>8 & 0x00ff) as u8 );
        self.send_command(&command)
    }

    pub fn prog_page(&mut self, data: &Vec<u8>) -> Response {
        let mut command = vec![Commands::CmndStkProgPage as u8];
        let size = data.len() as u16;
        command.push( ((size>>8) & 0x00ff) as u8 );
        command.push( (size & 0x00ff) as u8 );
        command.push( 'F' as u8 );
        command.extend(data);
        self.send_command(&command)
    }
}

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
    extern crate serialport;
    extern crate simple_logger;
    use self::serialport::prelude::*;
    use super::Programmer;
    use std::thread;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::rc::Rc;
    use std::cell::RefCell;

    use futures::Future;
    use futures::sync::oneshot;
    #[test]
    fn it_works() {
        simple_logger::init().unwrap();
        let programmer = Arc::new(Mutex::new(Programmer::new()));
        let _programmer = programmer.clone();

        let (tx, rx) = oneshot::channel::<()>();

        thread::spawn(move || {
            let s = serialport::SerialPortSettings {
                baud_rate: BaudRate::Baud57600,
                data_bits: DataBits::Eight,
                flow_control: FlowControl::None,
                parity: Parity::None,
                stop_bits: StopBits::One,
                timeout: Duration::from_millis(1),
            };
            let mut _port_inner = serialport::open_with_settings("/dev/ttyACM0", &s).unwrap();
            let mut buffer = [0; 256];
            let mut port = Arc::new(Mutex::new(_port_inner));

            // Set the write-callback
            {
                let mut p = _programmer.lock().unwrap();
                let _port = port.clone();
                p.set_write_cb(move |bytes| {
                    debug!("Sending bytes to underlying serial port...");
                    _port.lock().unwrap().write(bytes.as_slice());
                    debug!("Sending bytes to underlying serial port...done");
                });
            }

            tx.send(());

            loop {
                let maybe_len = {
                    port.lock().unwrap().read(&mut buffer)
                };
                if let Ok(len) = maybe_len {
                    debug!("Received {} bytes from serial port. Delivering...", len);
                    _programmer.lock().unwrap().deliver( buffer[0..len].to_vec() );
                    debug!("Received {} bytes from serial port. Delivering...done", len);
                } else {
                    thread::sleep( Duration::from_millis(1) );
                }
            }
        });

        rx.wait().unwrap();
        
        let f = {
            programmer.lock().unwrap().get_sync()
        };
        if let Ok(bytes) = f.wait() {
            println!("Got response from get_sync()... {} bytes", bytes.len());
        }

        let f = {
            programmer.lock().unwrap().sign_on()
        };
        if let Ok(bytes) = f.wait() {
            println!("Got response from sign_on()... {} bytes", bytes.len());
        }

        let f = {
            programmer.lock().unwrap().read_sign()
        };
        if let Ok(bytes) = f.wait() {
            println!("Got response from read_sign()... {} bytes", bytes.len());
        }

    }
}
