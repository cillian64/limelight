#![no_std]
#![no_main]

// Halt on panic
use panic_halt as _;

use rp235x_hal as hal;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

use dmg1083::{PanelData, SCAN_LINES};
use dmg1083_software_rp235x::Dmg1083;
use mbi5153::{Mbi5153Config, Mbi5153Config2};
use misc_hacks::{InvertedPin, PinGroup, PinRef};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{
        Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
    },
    text::{Alignment, Text},
    mock_display::MockDisplay,
};

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// External high-speed crystal on the Raspberry Pi Pico 2 board is 12 MHz.
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led1_pin = pins.gpio26.into_push_pull_output();
    let mut led2_pin = pins.gpio0.into_push_pull_output();
    led1_pin.set_high().unwrap();

//    loop {
//        led1_pin.set_high().unwrap();
//        led2_pin.set_low().unwrap();
//        timer.delay_ms(500);
//        led1_pin.set_low().unwrap();
//        led2_pin.set_high().unwrap();
//        timer.delay_ms(500);
//    }

    // Address lines
    let addr_a = pins.gpio2.into_push_pull_output().into_dyn_pin();
    let addr_b = pins.gpio3.into_push_pull_output().into_dyn_pin();
    let addr_c = pins.gpio4.into_push_pull_output().into_dyn_pin();
    let addr_d = pins.gpio5.into_push_pull_output().into_dyn_pin();
    let addr_e = pins.gpio6.into_push_pull_output().into_dyn_pin();

    // Clocks
    let gclk = pins.gpio7.into_push_pull_output().into_dyn_pin();
    let mut dclk = pins.gpio8.into_push_pull_output().into_dyn_pin();
    let mut latch = pins.gpio9.into_push_pull_output().into_dyn_pin();

    // Data lines
    let mut r1_sdi = pins.gpio10.into_push_pull_output().into_dyn_pin();
    let mut g1_sdi = pins.gpio11.into_push_pull_output().into_dyn_pin();
    let mut b1_sdi = pins.gpio12.into_push_pull_output().into_dyn_pin();
    let mut r2_sdi = pins.gpio13.into_push_pull_output().into_dyn_pin();
    let mut g2_sdi = pins.gpio14.into_push_pull_output().into_dyn_pin();
    let mut b2_sdi = pins.gpio15.into_push_pull_output().into_dyn_pin();
    // note schematic fuckery:
    // G3 actually controls R3
    // (R3 actually controls G3)
    // B4 actually controls B3
    // (B3 actually controls B4)
    // G4 actually controls R4
    // (R4 actually controls G4)
    let mut r3_sdi = pins.gpio17.into_push_pull_output().into_dyn_pin();
    let mut g3_sdi = pins.gpio16.into_push_pull_output().into_dyn_pin();
    let mut b3_sdi = pins.gpio21.into_push_pull_output().into_dyn_pin();
    let mut r4_sdi = pins.gpio20.into_push_pull_output().into_dyn_pin();
    let mut g4_sdi = pins.gpio19.into_push_pull_output().into_dyn_pin();
    let mut b4_sdi = pins.gpio18.into_push_pull_output().into_dyn_pin();
    // (end schematic fuckery)

    // SR pin that needs to be low to work
    let mut sr = pins.gpio22.into_push_pull_output().into_dyn_pin();
    sr.set_high().unwrap();

    let sdis = PinGroup([
        InvertedPin(PinRef(&mut r1_sdi)),
        InvertedPin(PinRef(&mut r2_sdi)),
        InvertedPin(PinRef(&mut r3_sdi)),
        InvertedPin(PinRef(&mut r4_sdi)),
        InvertedPin(PinRef(&mut g1_sdi)),
        InvertedPin(PinRef(&mut g2_sdi)),
        InvertedPin(PinRef(&mut g3_sdi)),
        InvertedPin(PinRef(&mut g4_sdi)),
        InvertedPin(PinRef(&mut b1_sdi)),
        InvertedPin(PinRef(&mut b2_sdi)),
        InvertedPin(PinRef(&mut b3_sdi)),
        InvertedPin(PinRef(&mut b4_sdi)),
    ]);

    let config = Mbi5153Config {
        scan_line_count: SCAN_LINES as _,
        current_gain: 255,
        gclk_multiplier: true,
        lower_ghost_elimination: true,
        ..Default::default()
    };

    dmg1083::configure_panel(
        config,
        InvertedPin(PinRef(&mut dclk)),
        InvertedPin(PinRef(&mut latch)),
        sdis,
    )
    .unwrap();

    dmg1083::configure_panel_2(
        InvertedPin(PinRef(&mut dclk)),
        InvertedPin(PinRef(&mut latch)),
        Mbi5153Config2::appnote_secret_sauce_r(),
        PinGroup([
            InvertedPin(PinRef(&mut r1_sdi)),
            InvertedPin(PinRef(&mut r2_sdi)),
            InvertedPin(PinRef(&mut r3_sdi)),
            InvertedPin(PinRef(&mut r4_sdi)),
        ]),
        Mbi5153Config2::appnote_secret_sauce_gb(),
        PinGroup([
            InvertedPin(PinRef(&mut g1_sdi)),
            InvertedPin(PinRef(&mut g2_sdi)),
            InvertedPin(PinRef(&mut g3_sdi)),
            InvertedPin(PinRef(&mut g4_sdi)),
        ]),
        Mbi5153Config2::appnote_secret_sauce_gb(),
        PinGroup([
            InvertedPin(PinRef(&mut b1_sdi)),
            InvertedPin(PinRef(&mut b2_sdi)),
            InvertedPin(PinRef(&mut b3_sdi)),
            InvertedPin(PinRef(&mut b4_sdi)),
        ]),
    )
    .unwrap();
    // rprintln!("[+] initialising software driver");

    let mut app = Dmg1083::new(
        dclk,
        gclk,
        latch,
        [
            r1_sdi, g1_sdi, b1_sdi, r2_sdi, g2_sdi, b2_sdi, r3_sdi, g3_sdi, b3_sdi, r4_sdi, g4_sdi,
            b4_sdi,
        ],
        [addr_a, addr_b, addr_c, addr_d, addr_e],
        sio.fifo,
        &mut pac.PSM,
        &mut pac.PPB,
        config.gclk_multiplier,
    );

    // SENDING DATA
    let mut panel: PanelData;

    let fill = PrimitiveStyle::with_fill(Rgb888::BLUE);

    loop {
        panel = PanelData::default();

        Rectangle::new(Point::new(10, 10), Size::new(6, 6))
            .into_styled(fill)
            .draw(&mut panel)
            .unwrap();

        Circle::new(Point::new(40, 40), 5)
            .into_styled(fill)
            .draw(&mut panel)
            .unwrap();

        app.transmit_frame(&panel);
        app.vsync();
    }

}
