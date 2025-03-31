#![no_std]
#![no_main]
#![allow(unused_imports)]

use core::panic::PanicInfo;
use embassy_executor::Spawner;

// GPIO
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::{PIN_12, PIN_13, PIN_15};
use embassy_rp::pwm::{Config as PwmConfig, Pwm};

// Channel
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::channel::{Channel, Sender};

// USB driver
use core::cell::RefCell;
use core::fmt::Write;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::spi::{Async, Blocking, Spi};
use embassy_rp::{bind_interrupts, spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;

use byte_slice_cast::AsByteSlice;
use core::str::from_utf8;
use cyw43_pio::PioSpi;
use embassy_futures::select::{select, select_slice};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config, IpAddress, IpEndpoint, Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_rp::peripherals::USB;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::usb::{Driver, Endpoint, InterruptHandler as USBInterruptHandler};
use embassy_time::Delay;
use embassy_time::{Duration, Timer};
use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::iso_8859_16::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::renderer::CharacterStyle;
use embedded_graphics::text::Text;
use heapless::String;
use heapless::Vec;
use log::{info, warn};
use project::SPIDeviceInterface;
use st7789::{Orientation, ST7789};
use static_cell::StaticCell;
static CHANNEL: Channel<CriticalSectionRawMutex, ToShow, 1> = Channel::new();

bind_interrupts!(struct Irqs {
    // Use for the serial over USB driver
    USBCTRL_IRQ => USBInterruptHandler<USB>;
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const DISPLAY_FREQ: u32 = 64_000_000;
const WIFI_NETWORK: &str = "WeatherStation";
const WIFI_PASSWORD: &str = "ferriscrab";

// The formula for calculating the actual temperature value (in Celsius) from the raw value
fn calculate_temperature(temperature_adc: u32) -> i32 {
    let t_fine: i32;
    let var1: i32 = ((temperature_adc as i32 >> 3) - (27504 << 1)) * (26435 >> 11);
    let var2: i32 = ((temperature_adc as i32 >> 4) - 27504)
        * (((temperature_adc as i32 >> 4) - 27504) >> 12)
        * (-1000 >> 14);
    t_fine = var1 + var2;
    t_fine
}

fn calculate_pressure(t_fine: i32, pressure_adc: u32) -> i32 {
    let var1_initial: i32 = (t_fine >> 1) - 64000;
    let mut var1: i32;
    let mut var2: i32;

    var2 = (((var1_initial >> 2) * (var1_initial >> 2)) >> 11) * -7;
    var2 += (var1_initial * 140) << 1;
    var2 = (var2 >> 2) + (2855 << 16);

    var1 = (((3024 * (((var1_initial >> 2) * (var1_initial >> 2)) >> 13)) >> 3)
        * (-10685 * var1_initial)
        >> 1)
        >> 18;
    var1 = ((32768 + var1) * 36477) >> 15;

    if var1 == 0 {
        return 0; // Avoid exception caused by division by zero
    }

    let mut p: u32 = (1048576 - pressure_adc - ((var2 >> 12) as u32) * 3125) as u32;

    if p < 0x80000000 {
        p = (p << 1) / var1 as u32;
    } else {
        p = (p / var1 as u32) * 2;
    }

    var1 = (6000 * (((p >> 3) * (p >> 3)) >> 13) >> 12) as i32;
    var2 = ((p >> 2) as i32 * -14600) >> 13;
    p = (p + ((var1 + var2 + 15500) as u32 >> 4)) as u32;

    p as i32
}
enum ToShow {
    Temperature,
    Pressure,
    Both,
    Crustacean,
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn button_a_pressed(mut button_a: Input<'static>) {
    loop {
        button_a.wait_for_rising_edge().await;
        CHANNEL.send(ToShow::Temperature).await;
    }
}

#[embassy_executor::task]
async fn button_b_pressed(mut button: Input<'static>) {
    loop {
        button.wait_for_rising_edge().await;
        CHANNEL.send(ToShow::Pressure).await;
    }
}

#[embassy_executor::task]
async fn button_c_pressed(mut button: Input<'static>) {
    loop {
        button.wait_for_rising_edge().await;
        CHANNEL.send(ToShow::Both).await;
    }
}

#[embassy_executor::task]
async fn button_d_pressed(mut button: Input<'static>) {
    loop {
        button.wait_for_rising_edge().await;
        CHANNEL.send(ToShow::Crustacean).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());

    // Start the serial port over USB driver
    let driver = Driver::new(peripherals.USB, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    // Link CYW43 firmware
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    //Buttons
    let button_a = Input::new(peripherals.PIN_15, Pull::Up);
    let button_b = Input::new(peripherals.PIN_9, Pull::Up);
    let button_c = Input::new(peripherals.PIN_14, Pull::Up);
    let button_d = Input::new(peripherals.PIN_12, Pull::Up);

    // Init SPI for communication with CYW43
    let pwr = Output::new(peripherals.PIN_23, Level::Low);
    let cs = Output::new(peripherals.PIN_25, Level::High);
    let mut pio = Pio::new(peripherals.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        peripherals.PIN_24,
        peripherals.PIN_29,
        peripherals.DMA_CH0,
    );
    // Start Wi-Fi task
    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(wifi_task(runner)).unwrap();

    // Init the device
    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef;

    // Init network stack
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<2>::new()),
        seed,
    ));

    // Start network stack task
    spawner.spawn(net_task(stack)).unwrap();

    loop {
        // Join WPA2 access point
        match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status {}", err.status);
            }
        }
    }

    // Wait for DHCP (not necessary when using static IP)
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up {:?}!", stack.config_v4());

    //network stack
    let mut rx_buffer = [0; 4096];
    let mut rx_metadata_buffer = [PacketMetadata::EMPTY; 3];
    let mut tx_buffer = [0; 4096];
    let mut tx_metadata_buffer = [PacketMetadata::EMPTY; 3];

    // Initialize UDP socket
    let mut socket = UdpSocket::new(
        stack,
        &mut rx_metadata_buffer,
        &mut rx_buffer,
        &mut tx_metadata_buffer,
        &mut tx_buffer,
    );

    info!("Starting server on UDP:1234...");

    // Bind socket to port
    if let Err(e) = socket.bind(1234) {
        warn!("accept error: {:?}", e);
    }

    let mut packet = [0u8, 16];

    let mut bmp280_config = spi::Config::default();
    bmp280_config.frequency = 2_000_000;

    // Display SPI config
    let mut display_config = spi::Config::default();
    display_config.frequency = DISPLAY_FREQ;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;

    let miso = peripherals.PIN_4;
    let mosi = peripherals.PIN_19;
    let clk = peripherals.PIN_18;

    let spi_display: Spi<'_, _, Blocking> =
        Spi::new_blocking(peripherals.SPI0, clk, mosi, miso, display_config.clone());

    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi_display));
    let display_cs = Output::new(peripherals.PIN_17, Level::High);

    let rst = peripherals.PIN_0;
    let dc = peripherals.PIN_16;
    let dc = Output::new(dc, Level::Low);
    let rst = Output::new(rst, Level::Low);
    let display_spi = SpiDeviceWithConfig::new(&spi_bus, display_cs, display_config);
    let di = SPIDeviceInterface::new(display_spi, dc);

    let mut display = ST7789::new(di, rst, 240, 240);
    display.init(&mut Delay).unwrap();
    display.set_orientation(Orientation::Portrait).unwrap();
    display.clear(Rgb565::BLACK).unwrap();

    let mut style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    style.set_background_color(Some(Rgb565::BLACK));

    let miso_bmp = peripherals.PIN_8;
    let mosi_bmp = peripherals.PIN_11;
    let clk_bmp = peripherals.PIN_10;

    let mut spi = Spi::new(
        peripherals.SPI1,
        clk_bmp,
        mosi_bmp,
        miso_bmp,
        peripherals.DMA_CH1,
        peripherals.DMA_CH2,
        bmp280_config.clone(),
    );

    let mut bmp280_cs = Output::new(peripherals.PIN_3, Level::High);

    const REG_ADDR_CTRL_MEAS: u8 = 0xf4;
    const REG_ADDR_PRESS_MSB: u8 = 0xf7;
    const REG_ADDR_TEMP_MSB: u8 = 0xfa;

    spawner.spawn(button_a_pressed(button_a)).unwrap();
    spawner.spawn(button_b_pressed(button_b)).unwrap();
    spawner.spawn(button_c_pressed(button_c)).unwrap();
    spawner.spawn(button_d_pressed(button_d)).unwrap();

    loop {
        Timer::after_millis(1000).await;

        let tx_buf = [!(1 << 7) & REG_ADDR_CTRL_MEAS, 0b001_001_11];
        let mut rx_buf = [0u8; 2];

        let tx_buf1 = [(1 << 7) | REG_ADDR_PRESS_MSB, 0x00, 0x00, 0x00];
        let mut rx_buf1 = [0u8; 4];

        let tx_buf2 = [(1 << 7) | REG_ADDR_TEMP_MSB, 0x00, 0x00, 0x00];
        let mut rx_buf2 = [0u8; 4];

        bmp280_cs.set_low();
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        spi.transfer(&mut rx_buf1, &tx_buf1).await.unwrap();
        bmp280_cs.set_high();

        bmp280_cs.set_low();
        spi.transfer(&mut rx_buf, &tx_buf).await.unwrap();
        spi.transfer(&mut rx_buf2, &tx_buf2).await.unwrap();
        bmp280_cs.set_high();

        let press_msb = rx_buf1[1] as u32;
        let shifted_msb = press_msb << 12;
        let press_lsb = rx_buf1[2] as u32;
        let shifted_lsb = press_lsb << 4;
        let press_xlsb = rx_buf1[3] as u32;
        let shifted_xlsb = press_xlsb >> 4;

        let pressure_raw: u32 = shifted_msb + shifted_lsb + shifted_xlsb;
        info!("Pressure raw {pressure_raw}");

        let temp_msb = rx_buf2[1] as u32;
        let shifted_msb_t = temp_msb << 12;
        let temp_lsb = rx_buf2[2] as u32;
        let shifted_lsb_t = temp_lsb << 4;
        let temp_xlsb = rx_buf2[3] as u32;
        let shifted_xlsb_t = temp_xlsb >> 4;

        let temperature_raw: u32 = shifted_msb_t + shifted_lsb_t + shifted_xlsb_t;
        let temperature = ((calculate_temperature(temperature_raw) * 5 + 128) >> 8) / 100;
        let pressure =
            calculate_pressure(calculate_temperature(temperature_raw), pressure_raw) / 256;

        info!("Temperature {temperature:}");

        match CHANNEL.receive().await {
            ToShow::Temperature => {
                display.clear(Rgb565::BLACK).unwrap();
                let mut text = String::<64>::new();
                write!(text, "Temperature: {temperature:}° C").unwrap();

                Text::new(&text, Point::new(40, 110), style)
                    .draw(&mut display)
                    .unwrap();
            }
            ToShow::Pressure => {
                display.clear(Rgb565::BLACK).unwrap();
                let mut text = String::<64>::new();
                write!(text, "Pressure: {pressure:} kPa").unwrap();

                Text::new(&text, Point::new(40, 110), style)
                    .draw(&mut display)
                    .unwrap();
            }
            ToShow::Both => {
                display.clear(Rgb565::BLACK).unwrap();
                let mut text = String::<64>::new();
                write!(text, "Pressure: {pressure:} kPa").unwrap();

                Text::new(&text, Point::new(40, 110), style)
                    .draw(&mut display)
                    .unwrap();
                let mut text2 = String::<64>::new();
                write!(text2, "Temperature: {temperature:}° C").unwrap();
                Text::new(&text2, Point::new(40, 140), style)
                    .draw(&mut display)
                    .unwrap();
            }
            ToShow::Crustacean => {
                display.clear(Rgb565::BLACK).unwrap();
                let raw_image_data = ImageRawLE::new(include_bytes!("../../assets/ferris.raw"), 86);
                let ferris = Image::new(&raw_image_data, Point::new(77, 77));
                ferris.draw(&mut display).unwrap();
                packet[0] = temperature as u8;
                packet[1] = pressure_raw as u8;
                loop {
                    match socket
                        .send_to(
                            &packet,
                            IpEndpoint::new(IpAddress::v4(192, 168, 255, 70), 1234),
                        )
                        .await
                    {
                        Ok(()) => {
                            info!("Weather data sent!");
                        }
                        Err(e) => {
                            warn!("Send error: {:?}", e);
                        }
                    }
                    Timer::after_millis(1000).await;
                }
            }
        }

        // Small delay for yielding
        Timer::after_millis(10).await;
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
