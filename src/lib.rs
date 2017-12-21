extern crate futures;

use futures::Future;
use futures::sync::oneshot;

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
        self.buffer.extend(buf.iter());
        if self.buffer.len() < 2 {
            // Buffer too small. Wait for more data.
            return;
        }
        if let Some(byte) = self.buffer.last() {
            if *byte != Commands::RespStkOk as u8 {
                return;
            }
        }
        let byte = *self.buffer.first().unwrap();
        if byte != Commands::RespStkInsync as u8 {
            self.buffer.clear();
            return;
        }
        match self.state {
            State::Idle => {
            }
            State::WaitResponse => {
                let maybe_sender = self.waiting_future.take();
                if let Some(sender) = maybe_sender {
                    sender.send(buf).unwrap();
                }
                self.state = State::Idle;
            }
        }
    }

    fn send_command(&mut self, buf: &Vec<u8>) {
        if let Some(ref cb) = self.write_cb {
            let mut _buf = buf.clone();
            _buf.push(Commands::SyncCrcEop as u8);
            cb(_buf);
        }
    }

    pub fn sign_on(&mut self) -> Box<Future<Item=Vec<u8>, Error=oneshot::Canceled>> {
        self.state = State::WaitResponse;
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        self.waiting_future = Some(tx);
        self.send_command(&vec![Commands::CmndStkGetSignOn as u8]);
        return Box::new(rx);
    }
}

#[cfg(test)]
mod tests {
    extern crate serialport;
    use self::serialport::prelude::*;
    use super::Programmer;
    use std::thread;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use futures::Future;
    #[test]
    fn it_works() {
        let programmer = Arc::new(Mutex::new(Programmer::new()));
        let _programmer = programmer.clone();
        thread::spawn(move || {
            let s = serialport::SerialPortSettings {
                baud_rate: BaudRate::Baud57600,
                data_bits: DataBits::Eight,
                flow_control: FlowControl::None,
                parity: Parity::None,
                stop_bits: StopBits::One,
                timeout: Duration::from_millis(1),
            };
            let mut port = serialport::open_with_settings("/dev/ttyACM0", &s).unwrap();
            let mut buffer = [0; 256];

            loop {
                if let Ok(len) = port.read(&mut buffer) {
                    _programmer.lock().unwrap().deliver( buffer[0..len].to_vec() );
                }
            }
        });

        let f = {
            programmer.lock().unwrap().sign_on()
        };
        f.wait();
    }
}
