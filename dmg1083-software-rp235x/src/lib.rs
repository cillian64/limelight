#![no_std]

use cortex_m::asm::nop;
use dmg1083::{PanelData, ADDRESSES, IC_COUNT, OUTPUTS, SCAN_LINES};
use embedded_hal::digital::v2::OutputPin;
use mbi5153::PanelState;
use rp235x_hal::gpio::{DynPinId, DynPullType, FunctionSioOutput, Pin, PullType};
use rp235x_hal::multicore::{Multicore, Stack};
use rp235x_hal::pac::{PPB, PSM};
use rp235x_hal::sio::SioFifo;
use rp235x_hal::{pac, Sio};

type Outpin = Pin<DynPinId, FunctionSioOutput, DynPullType>;

const CMD_STOP_GCLK: u32 = 0x42;
const CMD_RESTART_GCLK: u32 = 0x43;
const RESP_ACK: u32 = 0x11;
const RESP_ONLINE: u32 = 0x12;

/// `Scanouter` has the address lines and grayscale clock line (GCLK).
struct Scanouter {
    /// Addresses A through E.
    addresses: [Outpin; 5],
    gclk: Outpin,
    fifo: Option<SioFifo>,
    gclk_multiplier: bool,
}

impl Scanouter {
    fn handle_stop(&mut self) {
        self.fifo.as_mut().unwrap().write_blocking(RESP_ACK);
        self.gclk.set_low().unwrap();
        for a in self.addresses.iter_mut() {
            a.set_low().unwrap()
        }
        assert_eq!(
            self.fifo.as_mut().unwrap().read_blocking(),
            CMD_RESTART_GCLK
        );
        self.fifo.as_mut().unwrap().write_blocking(RESP_ACK);
    }

    fn run(&mut self) {
        let gclks_per_line = if self.gclk_multiplier { 257 } else { 513 };
        if let Some(fifo) = self.fifo.as_mut() {
            fifo.write_blocking(RESP_ONLINE);
        }
        loop {
            for line in 0..SCAN_LINES {
                for i in 0..5 {
                    self.addresses[i]
                        .set_state(((line >> i) & 1 > 0).into())
                        .unwrap();
                }

                for _ in 0..gclks_per_line {
                    self.gclk.set_low().unwrap();
                    nop();
                    self.gclk.set_high().unwrap();
                    nop();
                }

                self.gclk.set_low().unwrap();
                for i in 0..5 {
                    self.addresses[i].set_low().unwrap();
                }

                for _ in 0..10 {
                    nop();
                }
            }

            if let Some(fifo) = self.fifo.as_mut() {
                if fifo.is_read_ready() {
                    let data = fifo.read();
                    match data {
                        Some(CMD_STOP_GCLK) => self.handle_stop(),
                        Some(_x) => panic!("unexpected cmd {_x} on core1"),
                        None => {} // that's odd
                    }
                }
            }
        }
    }
}

pub struct Dmg1083 {
    dclk: Outpin,
    le: Outpin,
    /// SDI inputs for R/G/B, 1 through 4.
    sdi_rgb: [Outpin; OUTPUTS],
    scanout: Option<Scanouter>,
    fifo: SioFifo,
}

impl Dmg1083 {
    #[allow(clippy::too_many_arguments)]
    pub fn new<U: PullType>(
        dclk: Pin<DynPinId, FunctionSioOutput, U>,
        gclk: Pin<DynPinId, FunctionSioOutput, U>,
        latch: Pin<DynPinId, FunctionSioOutput, U>,
        outputs: [Pin<DynPinId, FunctionSioOutput, U>; OUTPUTS],
        addresses: [Pin<DynPinId, FunctionSioOutput, U>; ADDRESSES],
        fifo: SioFifo,
        psm: &mut PSM,
        ppb: &mut PPB,
        gclk_multiplier: bool,
    ) -> Self {
        let mut ret = Self {
            scanout: Some(Scanouter {
                addresses: addresses.map(|x| x.into_pull_type()),
                gclk: gclk.into_pull_type(),
                fifo: None,
                gclk_multiplier,
            }),
            dclk: dclk.into_pull_type(),
            le: latch.into_pull_type(),
            sdi_rgb: outputs.map(|x| x.into_pull_type()),
            fifo,
        };
        ret.spawn_second_core(psm, ppb);
        ret
    }

    fn stop_gclk(&mut self) {
        self.fifo.write_blocking(CMD_STOP_GCLK);
        assert_eq!(self.fifo.read_blocking(), RESP_ACK);
    }

    fn restart_gclk(&mut self) {
        self.fifo.write_blocking(CMD_RESTART_GCLK);
        assert_eq!(self.fifo.read_blocking(), RESP_ACK);
    }

    pub fn transmit_frame(&mut self, data: &PanelData) {
        let state = PanelState::new(
            [
                &data.0[0].red,
                &data.0[0].green,
                &data.0[0].blue,
                &data.0[1].red,
                &data.0[1].green,
                &data.0[1].blue,
                &data.0[2].red,
                &data.0[2].green,
                &data.0[2].blue,
                &data.0[3].red,
                &data.0[3].green,
                &data.0[3].blue,
            ],
            IC_COUNT as _,
            0b1111_1111_1111_1100,
        );
        for outputs in state {
            for (i, sdi) in outputs.sdis.into_iter().enumerate() {
                self.sdi_rgb[i].set_state(sdi.into()).unwrap();
            }
            self.le.set_state(outputs.le.into()).unwrap();
            self.dclk.set_state(outputs.dclk.into()).unwrap();
        }

        self.le.set_low().unwrap();
        self.dclk.set_low().unwrap();
    }

    pub fn vsync(&mut self) {
        self.stop_gclk();

        self.le.set_high().unwrap();
        nop();
        self.dclk.set_high().unwrap();
        nop();
        self.dclk.set_low().unwrap();
        nop();
        self.dclk.set_high().unwrap();
        nop();
        self.dclk.set_low().unwrap();
        nop();
        self.le.set_low().unwrap();
        nop();

        self.restart_gclk();
    }

    fn spawn_second_core(&mut self, psm: &mut PSM, ppb: &mut PPB) {
        let scanouter = self.scanout.take().unwrap();
        let mut mc = Multicore::new(psm, ppb, &mut self.fifo);
        mc.cores()[1]
            .spawn(unsafe { &mut CORE1_STACK.mem }, move || {
                core1_main(scanouter)
            })
            .unwrap();

        assert_eq!(self.fifo.read_blocking(), RESP_ONLINE);
    }
}

static mut CORE1_STACK: Stack<4096> = Stack::new();

fn core1_main(mut scanouter: Scanouter) {
    let pac = unsafe { pac::Peripherals::steal() };
    // let _core = unsafe { pac::CorePeripherals::steal() };
    let sio = Sio::new(pac.SIO);
    // FIXME(eta): why can't I initialise the pins here??
    scanouter.fifo = Some(sio.fifo);
    scanouter.run();
    unreachable!();
}
