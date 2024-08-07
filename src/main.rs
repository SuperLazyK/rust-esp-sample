//fn main() {
//    // It is necessary to call this function once. Otherwise some patches to the runtime
//    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
//    esp_idf_svc::sys::link_patches();
//
//    // Bind the log crate to the ESP Logging facilities
//    esp_idf_svc::log::EspLogger::initialize_default();
//
//    //log::info!("Hello, world!");
//    println!("Hello, world!");
//}

use anyhow::Result;
use esp_idf_svc::{
  hal::{gpio::PinDriver, peripherals::Peripherals},
  sys::link_patches,
};
use std::{thread::sleep, time::Duration};

fn main() -> Result<()> {
  link_patches();

  // Bind the log crate to the ESP Logging facilities
  esp_idf_svc::log::EspLogger::initialize_default();

  let peripherals = Peripherals::take()?;
  let pin_btn_a = PinDriver::input(peripherals.pins.gpio39)?;
  let pin_btn_b = PinDriver::input(peripherals.pins.gpio38)?;
  let pin_btn_c = PinDriver::input(peripherals.pins.gpio37)?;

  loop {
      let btn_a = pin_btn_a.is_low();
      let btn_b = pin_btn_b.is_low();
      let btn_c = pin_btn_c.is_low();

      println!("buttonABC: {} {} {}", btn_a, btn_b, btn_c);
      sleep(Duration::from_secs(2));
  }
  //// LED (GPIO2)
  //let mut led1 = PinDriver::output(peripherals.pins.gpio2)?;
  //loop {
  //    led1.set_high()?;
  //    sleep(Duration::from_secs(2));
  //    led1.set_low()?;
  //    sleep(Duration::from_secs(2));
  //}
}

