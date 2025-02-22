#![no_std]
#![no_main]

// Halt on panic
use panic_halt as _;

use rp235x_hal as hal;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

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
    loop {
        led1_pin.set_high().unwrap();
        led2_pin.set_low().unwrap();
        timer.delay_ms(500);
        led1_pin.set_low().unwrap();
        led2_pin.set_high().unwrap();
        timer.delay_ms(500);
    }
}
