#![no_std]
#![no_main]

use heapless::String;
use core::write;
use core::fmt::Write;
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{self, Input},
    rtc::{DateTime, DayOfWeek, Rtc},
    bind_interrupts,
    pio::{InterruptHandler as PioIrHandler, Pio},
    peripherals::{DMA_CH0, PIO0, USB},
    dma,
    clocks::RoscRng,
    usb::{Driver, InterruptHandler as UsbIrHandler},
};
use embassy_time::{Duration, Timer};

use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    {Config, StackResources},
};
use reqwless::{
    request::Method,
    client::HttpClient,
};
use core::str::from_utf8;
use serde::Deserialize;
use serde_json_core::from_slice;
use static_cell::StaticCell;
use datealgo::secs_to_datetime;

use embassy_usb::{
    UsbDevice,
    class::cdc_acm::{CdcAcmClass, State},
};

use portable_atomic::{AtomicU16, Ordering};

static CONVERTED_TIME: AtomicU16 = AtomicU16::new(0); //4 digits, hour and minute

bind_interrupts!(struct Irqs {
    RTC_IRQ => embassy_rp::rtc::InterruptHandler;

    PIO0_IRQ_0 => PioIrHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;

    USBCTRL_IRQ => UsbIrHandler<USB>;
});

const WIFI_NETWORK: &str = "pelu's Nothing Phone";
const WIFI_PASSWORD: &str = "kws8b8tj";

//for wifi
#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>) -> ! {
    runner.run().await
}
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

//to treat 7 segment
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
                self.f.set_high(); //there stil remains a strong debate about wheather f is needed, at least on me
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
                self.a.set_high(); //0 and sth other than 0~9
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
//continue to light 7 segment behind main
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

//for seriel
type MyUsbDriver = Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;
#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}


#[embassy_executor::main]
async fn main(spawner: Spawner) {
//initialization
    let p = embassy_rp::init(Default::default());

    //rtc
    let mut rtc = Rtc::new(p.RTC, Irqs);
    if !rtc.is_running() {
        let birthday = DateTime {
            year: 2007,
            month: 4,
            day: 16,
            day_of_week: DayOfWeek::Monday,
            hour: 7,
            minute: 0, //can someone let me know what time was I born 
            second: 0,
        };
        rtc.set_datetime(birthday).unwrap();
    }

    //wifi
    let fw = aligned_bytes!("../firmware/43439A0.bin");
    let clm = aligned_bytes!("../firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../firmware/nvram_rp2040.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(unwrap!(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    let mut rng = RoscRng;
    let seed = rng.next_u64();
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    spawner.spawn(unwrap!(net_task(runner)));

    while let Err(err) = control
        .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
        .await
    {
        info!("join failed: {:?}", err);
    }
    stack.wait_link_up().await;
    stack.wait_config_up().await;

    //seriel (to debug
    let driver = Driver::new(p.USB, Irqs);
    let config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };
    let usb = builder.build();
    spawner.spawn(unwrap!(usb_task(usb)));


//pinout
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
    spawner.spawn(seven_segment_task(seven_segment).unwrap());
    
    let button = Input::new(p.PIN_0, gpio::Pull::Up); //you need to connect kinda, pin to button to gnd

//run
    loop {
        //get current time when button is pushed
        if button.is_low() {
            let mut rx_buffer = [0; 4096];
            let client_state = TcpClientState::<1, 4096, 4096>::new();
            let tcp_client = TcpClient::new(stack, &client_state);
            let dns_client = DnsSocket::new(stack);
            let mut http_client = HttpClient::new(&tcp_client, &dns_client);
            let url = "http://3fe5a5f690efc790d4764f1c528a4ebb89fa4168.nict.go.jp/cgi-bin/json";

            let mut request = match http_client.request(Method::GET, url).await {
                Ok(req) => req,
                Err(e) => {
                    let mut buf: String<64> = String::new();
                    write!(&mut buf, "Error req: {:?}\r\n", e).ok();
                    let _ = class.write_packet(buf.as_bytes()).await;
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };
            let response = match request.send(&mut rx_buffer).await {
                Ok(resp) => resp,
                Err(e) => {
                    let mut buf: String<64> = String::new();
                    write!(&mut buf, "Error res: {:?}\r\n", e).ok();
                    let _ = class.write_packet(buf.as_bytes()).await;
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };
            let body_bytes = match response.body().read_to_end().await {
                Ok(b) => b,
                Err(_e) => {
                    let mut buf: String<64> = String::new();
                    write!(&mut buf, "Error byte: {:?}\r\n", _e).ok();
                    let _ = class.write_packet(buf.as_bytes()).await;
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };
            let body = match from_utf8(body_bytes) {
                Ok(b) => b,
                Err(_e) => {
                    let mut buf: String<64> = String::new();
                    write!(&mut buf, "Error body: {:?}\r\n", _e).ok();
                    let _ = class.write_packet(buf.as_bytes()).await;
                    Timer::after(Duration::from_secs(5)).await;
                    continue;
                }
            };
            /*
            curl http://3fe5a5f690efc790d4764f1c528a4ebb89fa4168.nict.go.jp/cgi-bin/json
            {
            "id": "ntp-a1.nict.go.jp",
            "it": 0.000,
            "st": 1771404213.672,
            "leap": 36,
            "next": 1483228800,
            "step": 1
            }
             */
            #[allow(unused)]
            #[derive(Deserialize)]
            struct UnixTime<'a> {
                id: &'a str,
                it: f64,
                st: f64,
                leap: u8,
                next: u64,
                step: u8,
            }

            let bytes = body.as_bytes();
            match from_slice::<UnixTime>(bytes) {
                Ok((unix, _used)) => {
                    let mut st = unix.st;
                    st += 9. * 3600.; //JST
                    let (year, month, day, hour, minute, second) = { secs_to_datetime(st as i64) };
                    
                    let date = DateTime {
                        year: year as u16,
                        month: month,
                        day: day,
                        day_of_week: DayOfWeek::Monday, //I dont need it so no matter what its ok
                        hour: hour,
                        minute: minute,
                        second: second,
                    };
                    rtc.set_datetime(date).unwrap();
                }
                Err(e) => {
                    let mut buf: String<64> = String::new();
                    write!(&mut buf, "Error buf: {:?}\r\n", e).ok();
                    let _ = class.write_packet(buf.as_bytes()).await;
                }
            }
        }

        //send time from rtc to 7 segment
        if let Ok(dt) = rtc.now() {
            CONVERTED_TIME.store((dt.hour as u16 * 100 as u16 + dt.minute as u16) as u16, Ordering::Relaxed);
        }


        Timer::after_millis(200).await;
    }
}