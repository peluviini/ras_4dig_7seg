#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{self, Input},
    rtc::{DateTime, DayOfWeek, Rtc},
    bind_interrupts,
};
use embassy_time::Timer;
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use portable_atomic::{AtomicU16, Ordering};

static CONVERTED_TIME: AtomicU16 = AtomicU16::new(0); //4 digits, hour and minute

bind_interrupts!(struct Irqs {
    RTC_IRQ => embassy_rp::rtc::InterruptHandler;
});

pub struct SevenSegment<'a> {
    seg_1_gnd: Output<'a>, //assuming pins connecting mosfet's gate
    seg_2_gnd: Output<'a>, //so that cathode is connected to gnd through drain and source
    seg_3_gnd: Output<'a>,
    seg_4_gnd: Output<'a>,
    
    a: Output<'a>,
    b: Output<'a>,
    c: Output<'a>,
    d: Output<'a>,
    e: Output<'a>,
    f: Output<'a>,
    g: Output<'a>,
    dp: Output<'a>,
}
impl<'a> SevenSegment<'a> {

    pub fn display_digit_number(
        &mut self,
        digit: u8,
        number: u8,
    ) {
        self.turn_off_all();

        match digit {
            4 => self.seg_4_gnd.set_high(),
            3 => self.seg_3_gnd.set_high(),
            2 => self.seg_2_gnd.set_high(),
            1 => self.seg_1_gnd.set_high(),
            _ => self.seg_1_gnd.set_high(),
        }
        match number {
            1 => {
                self.b.set_high();
                self.c.set_high();
            },
            2 => {
                self.a.set_high();
                self.b.set_high();
                self.g.set_high();
                self.e.set_high();
                self.d.set_high();
            },
            3 => {
                self.a.set_high();
                self.b.set_high();
                self.g.set_high();
                self.c.set_high();
                self.d.set_high();
            },
            4 => {
                self.f.set_high();
                self.g.set_high();
                self.b.set_high();
                self.c.set_high();
            },
            5 => {
            self.a.set_high();
            self.f.set_high();
            self.g.set_high();
            self.c.set_high();
            self.d.set_high();
            },
            6 => {
            self.a.set_high();
            self.f.set_high();
            self.g.set_high();
            self.c.set_high();
            self.d.set_high();
            self.e.set_high();                
            },
            7 => {
            self.f.set_high();
            self.a.set_high();
            self.b.set_high();
            self.c.set_high();
            },
            8 => {
            self.a.set_high();
            self.b.set_high();
            self.c.set_high();
            self.d.set_high();
            self.e.set_high();
            self.f.set_high();
            self.g.set_high();
            },
            9 => {
            self.a.set_high();
            self.f.set_high();
            self.b.set_high();
            self.g.set_high();
            self.c.set_high();
            self.d.set_high();
            },
            10 => {
            self.dp.set_high();
            },
            _ => {
            self.a.set_high();
            self.b.set_high();
            self.c.set_high();
            self.d.set_high();
            self.e.set_high();
            self.f.set_high();
             }
        }
    }

    fn turn_off_all(&mut self) {
        self.seg_1_gnd.set_low();
        self.seg_2_gnd.set_low();
        self.seg_3_gnd.set_low();
        self.seg_4_gnd.set_low();
        self.a.set_low();
        self.b.set_low();
        self.c.set_low();
        self.d.set_low();
        self.e.set_low();
        self.f.set_low();
        self.g.set_low();
        self.dp.set_low();
    }
}

#[embassy_executor::task]
async fn seven_segment_task(
    mut seven_segment: SevenSegment<'static>,
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
    let p = embassy_rp::init(Default::default());

    let mut rtc = Rtc::new(p.RTC, Irqs);
    if !rtc.is_running() {
        let birthday = DateTime {
            year: 2007,
            month: 4,
            day: 16,
            day_of_week: DayOfWeek::Monday,
            hour: 7,
            minute: 0,
            second: 0,
        };
        rtc.set_datetime(birthday).unwrap();
    }
    
    let seg_1_gnd = Output::new(p.PIN_14, Level::Low);
    let seg_2_gnd = Output::new(p.PIN_15, Level::Low);
    let seg_3_gnd = Output::new(p.PIN_16, Level::Low);
    let seg_4_gnd = Output::new(p.PIN_17, Level::Low);

    let a = Output::new(p.PIN_18, Level::Low);
    let b = Output::new(p.PIN_20, Level::Low);
    let c = Output::new(p.PIN_21, Level::Low);
    let d = Output::new(p.PIN_27, Level::Low);
    let e = Output::new(p.PIN_28, Level::Low);
    let f = Output::new(p.PIN_19, Level::Low);
    let g = Output::new(p.PIN_22, Level::Low);
    let dp = Output::new(p.PIN_26, Level::Low);

    let seven_segment = SevenSegment{
        seg_1_gnd, seg_2_gnd, seg_3_gnd, seg_4_gnd,
        a, b, c, d, e, f, g, dp
    };

    let button = Input::new(p.PIN_0, gpio::Pull::Up);

    spawner.spawn(seven_segment_task(seven_segment).unwrap());

    loop {
        if let Ok(dt) = rtc.now() {
            CONVERTED_TIME.store((dt.hour as u16 * 100 as u16 + dt.minute as u16) as u16, Ordering::Relaxed);
        }
        Timer::after_millis(200).await;
    }
}