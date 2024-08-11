use anyhow::Result;
use esp_idf_svc::sys::link_patches;
use esp_idf_svc::hal::{
  gpio::{PinDriver, OutputPin, InputPin, Gpio0},
  peripherals::Peripherals,
  prelude::*,
  spi::{SpiConfig, SpiDeviceDriver, SpiDriver, SpiDriverConfig},
};
use display_interface_spi;
use std::{thread, thread::sleep, time::Duration};
use std::net::{TcpListener, TcpStream};
use std::io::{self, Error};
use std::io::Read;
use std::io::Write;
use ili9341::DisplayError;


use esp_idf_svc::{
    hal:: {spi::SpiAnyPins, peripheral::Peripheral},
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
    sys::EspError,
};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::text::*;


use core::convert::TryInto;

use embedded_svc::{
    http::{Headers, Method},
    //io::{Read, Write},
    wifi::{self, AccessPointConfiguration, AuthMethod},
};

use ili9341;

// TODO env with compiler
const SSID: &str = "M5STACK_SSID";
const PASSWORD: &str = "xxxxxx5555";

fn increment(current: i8) -> i8 {
    current.wrapping_add(1)
}

fn draw_sample<T>(lcd: &mut T)
where
    T: DrawTarget<Color = Rgb565, Error = DisplayError>,
{
    // Create a border style
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::BLUE)
        .stroke_width(8)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    // Draw border around the screen
    let _ = lcd.bounding_box().into_styled(border_stroke).draw(lcd);

    // Create text style
    let character_style =
        MonoTextStyle::new(&FONT_10X20, Rgb565::BLACK);
    let text = "Rust and M5Stack !!";

    // Draw text
    let textdrawable = Text::with_alignment(
        text,
        lcd.bounding_box().center() + Point::new(0, 15),
        character_style,
        Alignment::Center,
    );
    let _ = textdrawable.draw(lcd);
}

fn tcp_server() -> Result<(), Error> {
    fn accept() -> Result<(), Error> {
        println!("About to bind a simple echo service to port 8080; do `telnet <ip-from-above>:8080`");

        let listener = TcpListener::bind("0.0.0.0:8080")?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("Accepted client");
                    thread::spawn(move || {
                        handle(stream);
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        unreachable!()
    }

    fn handle(mut stream: TcpStream) {
        // Read 128 bytes at a time from stream echoing back to stream
        loop {
            let mut read = [0; 128];

            match stream.read(&mut read) {
                Ok(n) => {
                    if n == 0 {
                        // connection was closed
                        break;
                    }
                    let _ = stream.write_all(&read[0..n]);
                }
                Err(err) => {
                    panic!("{}", err);
                }
            }
        }
    }

    accept()
}

fn wifi_create() -> Result<esp_idf_svc::wifi::EspWifi<'static>, EspError> {
    use esp_idf_svc::eventloop::*;
    use esp_idf_svc::hal::prelude::Peripherals;
    use esp_idf_svc::nvs::*;
    use esp_idf_svc::wifi::*;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let peripherals = Peripherals::take()?;

    let mut esp_wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs.clone()))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone())?;

    wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
        ssid: SSID.try_into().unwrap(),
        ssid_hidden: false,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: 11,
        ..Default::default()
    }))?;

    wifi.start()?;
    println!("Wifi started");

    wifi.wait_netif_up()?;
    //wifi.connect()?;
    //println!("Wifi connected");

    wifi.wait_netif_up()?;
    println!("Wifi netif up");

    println!(
        "IP info: {:?} {:?}",
        wifi.wifi().sta_netif().get_ip_info().unwrap(),
        wifi.is_connected(),
    );

    Ok(esp_wifi)
}

fn main() -> Result<()> {
  link_patches();
  // Bind the log crate to the ESP Logging facilities
  esp_idf_svc::log::EspLogger::initialize_default();

  let _wifi = wifi_create();
  tcp_server()?;

  let peripherals = Peripherals::take()?;

  let mut pin_lcd_blk = PinDriver::input_output(peripherals.pins.gpio32)?;
  pin_lcd_blk.set_high().unwrap();

  println!("Issue LCD Reset by GPIO pin");
  // https://github.com/m5stack/m5-docs/blob/master/docs/ja/core/gray.md
  let mut lcd_reset_pin = PinDriver::output(peripherals.pins.gpio33)?;
  lcd_reset_pin.set_low().unwrap();
  sleep(Duration::from_millis(100));
  lcd_reset_pin.set_high().unwrap();
  sleep(Duration::from_millis(2000));

  let mut pin_cs = PinDriver::output(peripherals.pins.gpio14)?;
  pin_cs.set_low()?;

  let lcd_dc_pin = PinDriver::output(peripherals.pins.gpio27)?;


  //SPI Driver
  let spiconf = SpiConfig::default().baudrate(10.MHz().into());
  let o18  = peripherals.pins.gpio18;
  let o23 = peripherals.pins.gpio23;

  let spi_d = SpiDriver::new(
      peripherals.spi2,
      o18,
      o23,
      None as Option<Gpio0>,
      &SpiDriverConfig::default()
  )?;

  // SPI device
  let spi = SpiDeviceDriver::new(
      spi_d,
      Some(peripherals.pins.gpio15), //CS
      &spiconf,
  )?;

  let spidisplayinterface = display_interface_spi::SPIInterface::new(spi, lcd_dc_pin);

  let mut lcd = ili9341::Ili9341::new(
      spidisplayinterface,
      lcd_reset_pin,
      &mut esp_idf_hal::delay::FreeRtos,
      ili9341::Orientation::Landscape,
      ili9341::DisplaySize240x320,
  )
      .expect("Failed to initialize LCD ILI9341.");

  println!("ILI9341 display: {}x{}", lcd.width(), lcd.height());

  //lcd.invert_mode(ili9341::ModeState::On).expect("Failed to InvertOn.");

  //lcd.command(ili9341::Command::MemoryAccessControl, &[0x00 | 0x08])
  //    .expect("Failed to issue MemoryAccessControl command");
  //let color:u16 = Rgb565::new(0, 255, 255).into();

  //let huga:DrawTarget = lcd;
  let _ = lcd.fill_solid(
      &mut Rectangle::new(Point::new(0, 0), Size::new(320, 240)),
      Rgb565::new(0, 255, 0),
  );

  //draw_sample(&mut lcd);
  //draw_btn_status(&mut lcd, false, false, false);


  let pin_btn_a = PinDriver::input(peripherals.pins.gpio39)?;
  let pin_btn_b = PinDriver::input(peripherals.pins.gpio38)?;
  let pin_btn_c = PinDriver::input(peripherals.pins.gpio37)?;

  let mut counter: i8 = 0;
  let mut prev_ticks: u32 = 0;
  let mut prev_btn_a = false;
  let mut prev_btn_b = false;
  let mut prev_btn_c = false;
  let mut btnstatusupdate_timer_sec: f32 = 0.0f32;
  let mut serialout_timer_sec: f32 = 0.0f32;

  loop {
         let now_ticks: esp_idf_sys::TickType_t;
         unsafe {
             now_ticks = esp_idf_sys::xTaskGetTickCount();
         }
         let delta_ticks: u32 = now_ticks - prev_ticks;
         unsafe {
             prev_ticks = esp_idf_sys::xTaskGetTickCount();
         }
         let delta_sec: f32 =
             (delta_ticks as f32) * (1.0f32 / (esp_idf_sys::configTICK_RATE_HZ as f32));

         let btn_a = pin_btn_a.is_low();
         let btn_b = pin_btn_b.is_low();
         let btn_c = pin_btn_c.is_low();

         println!("buttonABC: {} {} {}", btn_a, btn_b, btn_c);

         btnstatusupdate_timer_sec += delta_sec;

         if btnstatusupdate_timer_sec > 0.025f32 {
             btnstatusupdate_timer_sec = 0.0f32;

             if btn_a != prev_btn_a || btn_b != prev_btn_b || btn_c != prev_btn_c {
                 //draw_btn_status(&mut lcd, btn_a, btn_b, btn_c);
                 prev_btn_a = btn_a;
                 prev_btn_b = btn_b;
                 prev_btn_c = btn_c;
             }
         }

         serialout_timer_sec += delta_sec;

         if serialout_timer_sec > 1.0 {
             serialout_timer_sec = 0.0f32;

             if btn_a {
                 counter = counter.wrapping_add(10);
             }
             if btn_c {
                 counter = counter.wrapping_sub(16);
             }

             if !btn_a {
                 // BtnA not pressed.
                 println!("Hello world counter={}", counter);
             } else {
                 // BtnA pressed.
                 println!("BtnA Pressed !! counter={}", counter);
             }
             counter = increment(counter);
         }
         sleep(Duration::from_millis(10));
  }
}

