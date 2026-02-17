// sth if route

#![no_std]
#![no_main]

use embassy_time::Timer;
use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embedded_hal::digital::{InputPin, OutputPin};
use panic_probe as _;

use rp_pico::{
    hal::{
        gpio::{FunctionSio, SioOutput, Pin as rpPin, PullDown},
        clocks::{Clock, init_clocks_and_plls},
        pac,
        sio::Sio,
        watchdog::Watchdog,
    },
    pac::interrupt,
};


use portable_atomic::{AtomicU16, Ordering};

static CONVERTED_TIME: AtomicU16 = AtomicU16::new(0);


pub struct SevenSegment {
    seg_1_gnd: rpPin<rp_pico::hal::gpio::bank0::Gpio14, FunctionSio<SioOutput>, PullDown>,
    seg_2_gnd: rpPin<rp_pico::hal::gpio::bank0::Gpio15, FunctionSio<SioOutput>, PullDown>,
    seg_3_gnd: rpPin<rp_pico::hal::gpio::bank0::Gpio16, FunctionSio<SioOutput>, PullDown>,
    seg_4_gnd: rpPin<rp_pico::hal::gpio::bank0::Gpio17, FunctionSio<SioOutput>, PullDown>,
    
    a: rpPin<rp_pico::hal::gpio::bank0::Gpio18, FunctionSio<SioOutput>,  PullDown>,
    b: rpPin<rp_pico::hal::gpio::bank0::Gpio19, FunctionSio<SioOutput>, PullDown>,
    c: rpPin<rp_pico::hal::gpio::bank0::Gpio20, FunctionSio<SioOutput>, PullDown>,
    d: rpPin<rp_pico::hal::gpio::bank0::Gpio21, FunctionSio<SioOutput>, PullDown>,
    e: rpPin<rp_pico::hal::gpio::bank0::Gpio22, FunctionSio<SioOutput>, PullDown>,
    f: rpPin<rp_pico::hal::gpio::bank0::Gpio26, FunctionSio<SioOutput>, PullDown>,
    g: rpPin<rp_pico::hal::gpio::bank0::Gpio27, FunctionSio<SioOutput>, PullDown>,
    dp: rpPin<rp_pico::hal::gpio::bank0::Gpio28, FunctionSio<SioOutput>, PullDown>,   
}
impl SevenSegment {

    pub fn display_digit_number(
        &mut self,
        digit: u8,
        number: u8,
    ) {
        self.turn_off_all();

        match digit {
            4 => self.seg_4_gnd.set_high().unwrap(),
            3 => self.seg_3_gnd.set_high().unwrap(),
            2 => self.seg_2_gnd.set_high().unwrap(),
            1 => self.seg_1_gnd.set_high().unwrap(),
            _ => self.seg_1_gnd.set_high().unwrap(),
        }
        match number {
            1 => {
                self.b.set_high().unwrap();
                self.c.set_high().unwrap();
            },
            2 => {
                self.a.set_high().unwrap();
                self.b.set_high().unwrap();
                self.g.set_high().unwrap();
                self.e.set_high().unwrap();
                self.d.set_high().unwrap();
            },
            3 => {
                self.a.set_high().unwrap();
                self.a.set_high().unwrap();
                self.g.set_high().unwrap();
                self.c.set_high().unwrap();
                self.d.set_high().unwrap();
            },
            4 => {
                self.f.set_high().unwrap();
                self.g.set_high().unwrap();
                self.b.set_high().unwrap();
                self.c.set_high().unwrap();
            },
            5 => {
            self.a.set_high().unwrap();
            self.f.set_high().unwrap();
            self.g.set_high().unwrap();
            self.c.set_high().unwrap();
            self.d.set_high().unwrap();
            },
            6 => {
            self.a.set_high().unwrap();
            self.f.set_high().unwrap();
            self.g.set_high().unwrap();
            self.c.set_high().unwrap();
            self.d.set_high().unwrap();
            self.e.set_high().unwrap();                
            },
            7 => {
            self.f.set_high().unwrap();
            self.a.set_high().unwrap();
            self.b.set_high().unwrap();
            self.c.set_high().unwrap();
            },
            8 => {
            self.a.set_high().unwrap();
            self.b.set_high().unwrap();
            self.c.set_high().unwrap();
            self.d.set_high().unwrap();
            self.e.set_high().unwrap();
            self.f.set_high().unwrap();
            self.g.set_high().unwrap();
            },
            9 => {
            self.a.set_high().unwrap();
            self.f.set_high().unwrap();
            self.b.set_high().unwrap();
            self.g.set_high().unwrap();
            self.c.set_high().unwrap();
            self.d.set_high().unwrap();
            },
            10 => {
            self.dp.set_high().unwrap();
            },
            _ => {
            self.a.set_high().unwrap();
            self.b.set_high().unwrap();
            self.c.set_high().unwrap();
            self.d.set_high().unwrap();
            self.e.set_high().unwrap();
            self.f.set_high().unwrap();
             }
        }
    }

    fn turn_off_all(&mut self) {
        self.seg_1_gnd.set_low().unwrap();
        self.seg_2_gnd.set_low().unwrap();
        self.seg_3_gnd.set_low().unwrap();
        self.seg_4_gnd.set_low().unwrap();
        self.a.set_low().unwrap();
        self.b.set_low().unwrap();
        self.c.set_low().unwrap();
        self.d.set_low().unwrap();
        self.e.set_low().unwrap();
        self.f.set_low().unwrap();
        self.g.set_low().unwrap();
        self.dp.set_low().unwrap();
    }
}

#[embassy_executor::task]
async fn seven_segment_task(
    mut seven_segment: SevenSegment,
) {
    loop {
        let converted_time =  CONVERTED_TIME.load(Ordering::Relaxed);
        
        seven_segment.display_digit_number(4, (converted_time / 1000) as u8);
        Timer::after_micros(700).await;
        seven_segment.display_digit_number(3, ((converted_time % 1000) / 100) as u8);
        Timer::after_micros(700).await;
        seven_segment.display_digit_number(2, (((converted_time % 1000) % 100) / 10) as u8);
        Timer::after_micros(700).await;
        seven_segment.display_digit_number(1, (((converted_time % 1000) % 100) % 10) as u8);
        Timer::after_micros(700).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start"); 
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    
    unsafe { embassy_rp::time_driver::init(); }
    
    let seg_1_gnd = pins.gpio14.into_push_pull_output();
    let seg_2_gnd = pins.gpio15.into_push_pull_output();
    let seg_3_gnd = pins.gpio16.into_push_pull_output();
    let seg_4_gnd = pins.gpio17.into_push_pull_output();

    let a = pins.gpio18.into_push_pull_output();
    let b = pins.gpio19.into_push_pull_output();
    let c = pins.gpio20.into_push_pull_output();
    let d = pins.gpio21.into_push_pull_output();
    let e = pins.gpio22.into_push_pull_output();
    let f = pins.gpio26.into_push_pull_output();
    let g = pins.gpio27.into_push_pull_output();
    let dp = pins.gpio28.into_push_pull_output();

    let mut seven_segment = SevenSegment{
        seg_1_gnd, seg_2_gnd, seg_3_gnd, seg_4_gnd,
        a, b, c, d, e, f, g, dp
    };

    let mut button = pins.gpio0.into_pull_up_input();

    spawner.spawn(seven_segment_task(seven_segment).unwrap());


    let mut time: u16 = 1234;
    CONVERTED_TIME.store(time, Ordering::Relaxed);
    loop {
        if button.is_low().unwrap() {
            if time == 1234 {
                CONVERTED_TIME.store(time, Ordering::Relaxed);
                time = 4321;
            } else {
                CONVERTED_TIME.store(time, Ordering::Relaxed);
                time = 1234;
            }
        }
        Timer::after_millis(10).await;
    }
}