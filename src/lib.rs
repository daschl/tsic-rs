//! Allows to read the current temperature from the TSIC 306
//!
//! Note that most of this code is ported and heavily modified from C
//! to rust using the code found in [arduino-tsic](https://github.com/Schm1tz1/arduino-tsic)
//! and other places scattered throughout the internet that used the sensor
//! from C.
//!
//! Please also refer to the [Data Sheet](https://www.ist-ag.com/sites/default/files/DTTSic20x_30x_E.pdf)
//! for implementation details.
//!
//! ## Usage
//!
//! ```ignore
//! use tsic::Tsic;
//!
//! let sensor = Tsic::new(/* your hal pin */);
//!
//! let mut delay = /* your hal delay */();
//!
//! match sensor.read(&mut delay) {
//!   Ok(t) => defmt::info!("Temp is: {:f32}", t.as_celsius()),
//!   Err(e) => defmt::warn!("Getting sensor data failed: {:?}", e),
//! };
//! ```

#![forbid(unsafe_code)]
#![no_std]
#![doc(html_root_url = "https://docs.rs/tsic/0.2.0")]
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]

use core::time::Duration;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::InputPin;

/// The spec defines the sample rate as 128kHz, which is 7.8 microseconds. Since
/// we can only sleep for a round number of micros, 8 micros should be close enough.
static STROBE_SAMPLING_RATE: Duration = Duration::from_micros(8);

/// The `Tsic` struct is the main entry point when trying to get a temperature reading from a
/// TSIC 306 sensor.
pub struct Tsic<I: InputPin> {
    pin: I,
}

impl<I: InputPin> Tsic<I> {
    /// Creates a new `Tsic` sensor wrapper and binds it to the input pin given.
    pub fn new(pin: I) -> Self {
        Self { pin }
    }

    /// Attempts to read from the sensor, might fail (see errors for details if so).
    ///
    /// Note that the passed in `Delay` from the HAL needs to be aquired outside of
    /// this struct and passed in as mutable, because to aquire correct data from the
    /// sensor the code needs to pause for a certain amount of microseconds.
    pub fn read<D: DelayUs<u8>>(&self, delay: &mut D) -> Result<Temperature, TsicError> {
        let first_packet = self.read_packet(delay)?;
        let second_packet = self.read_packet(delay)?;
        Ok(Temperature::new(first_packet, second_packet))
    }

    /// Reads the bits off of the sensor port based on the ZACWire protocol.
    ///
    /// From the documentation of the sensor:
    ///
    /// When the falling edge of the start bit occurs, measure the time until the
    /// rising edge of the start bit. This time is the strobe time.  
    /// When the next falling edge occurs, wait for a time period equal to
    /// the strobe time, and then sample the signal. The data present on the signal
    /// at this time is the bit being transmitted. Because every bit starts  
    /// with a falling edge, the sampling window is reset with every bit  
    /// transmission. This means errors will not accrue for bits downstream  
    /// from the start bit, as it would with a protocol such as RS232. It is
    /// recommended, however, that the sampling rate of the signal when acquiring
    /// the start bit be at least 16x the nominal baud rate. Because the nominal
    /// baud rate is 8kHz, a 128kHz sampling rate is recommended when acquiring the
    /// strobe time.
    ///
    /// See https://www.ist-ag.com/sites/default/files/ATTSic_E.pdf for
    /// the full document.
    fn read_packet<D: DelayUs<u8>>(&self, delay: &mut D) -> Result<Packet, TsicError> {
        self.wait_until_low()?;

        let strobe_len = self.strobe_len(delay)?.as_micros() as u8;

        let mut packet_bits: u16 = 0;

        for _ in 0..9 {
            self.wait_until_low()?;

            delay.delay_us(strobe_len);

            packet_bits <<= 1;
            if self.is_high()? {
                packet_bits |= 1;
            }

            self.wait_until_high()?;
        }

        Packet::new(packet_bits)
    }

    /// Measures the strobe length of the sensor.
    ///
    /// According to docs and other code, depending on the temperature the sensor
    /// can change its strobe length so to be sure we'll just check it before every
    /// read attempt.
    ///
    /// The strobe length should be around 60 microseconds.
    fn strobe_len<D: DelayUs<u8>>(&self, delay: &mut D) -> Result<Duration, TsicError> {
        let sampling_rate = STROBE_SAMPLING_RATE.as_micros();

        let mut strobe_len = 0;
        while self.is_low()? {
            strobe_len += sampling_rate;
            delay.delay_us(sampling_rate as u8);
        }

        Ok(Duration::from_micros(strobe_len as u64))
    }

    /// Checks if the pin is currently in a high state.
    fn is_high(&self) -> Result<bool, TsicError> {
        self.pin.is_high().map_err(|_| TsicError::PinReadError)
    }

    /// Checks if the pin is currently in a low state.
    fn is_low(&self) -> Result<bool, TsicError> {
        self.pin.is_low().map_err(|_| TsicError::PinReadError)
    }

    /// Returns only once the pin is in a low state.
    fn wait_until_low(&self) -> Result<(), TsicError> {
        while self.is_high()? {}
        Ok(())
    }

    /// Returns only once the pin is in a high state.
    fn wait_until_high(&self) -> Result<(), TsicError> {
        while self.is_low()? {}
        Ok(())
    }
}

/// Contains all errors that can happen during a reading from the sensor.
#[derive(Debug)]
pub enum TsicError {
    /// The parity check for one of the packets failed.
    ///
    /// This might be a temporary issue, so attempting to perform another
    /// read might resolve the error.
    ParityCheckFailed,

    /// Failed to read the high/low state of the pin.
    PinReadError,
}

/// Represents a single temperature reading from the TSIC 306 sensor.
pub struct Temperature {
    raw: u16,
}

impl Temperature {
    /// Create a full temperature reading from the two individual half reading packets.
    fn new(first: Packet, second: Packet) -> Self {
        let raw = ((first.value() as u16) << 8) | second.value() as u16;
        Self { raw }
    }

    /// Returns the temperature in degree celsius.
    pub fn as_celsius(&self) -> f32 {
        (self.raw as f32 * 200.0 / 2047.0) - 50.0
    }
}

/// A `Packet` represents one half of the full temperature measurement.
struct Packet {
    raw_bits: u16,
}

impl Packet {
    /// Creates a new `Packet` from the raw measured bits.
    ///
    /// Note that this method performs a parity check on the input data and if
    /// that fails returns a `TsicError::ParityCheckFailed`.
    fn new(raw_bits: u16) -> Result<Self, TsicError> {
        if Self::has_even_parity(raw_bits) {
            Ok(Self { raw_bits })
        } else {
            Err(TsicError::ParityCheckFailed)
        }
    }

    /// Returns the actual data without the parity bit.
    fn value(&self) -> u16 {
        self.raw_bits >> 1
    }

    /// Performs parity bit checking on the raw packet value.
    fn has_even_parity(raw: u16) -> bool {
        raw.count_ones() % 2 == 0
    }
}
