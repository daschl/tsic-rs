//! Allows to read the current temperature from the TSIC temperature sensors.
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
//! Please see the comments on both the `with_vdd_control` and `without_vdd_control` constructors for
//! their usage and upsides/downsides.
//!
//! If the library should control both the signal and the vdd pins (recommended):
//!
//! ```ignore
//! use tsic::{SensorType, Tsic};
//!
//! let sensor = Tsic::with_vdd_control(SensorType::Tsic306, /* your hal signal input pin */, /* your vdd output pin */);
//!
//! let mut delay = /* your hal delay */();
//!
//! match sensor.read(&mut delay) {
//!   Ok(t) => defmt::info!("Temp is: {:f32}", t.as_celsius()),
//!   Err(e) => defmt::warn!("Getting sensor data failed: {:?}", e),
//! };
//! ```
//!
//! If the library should just control the signal pin:
//!
//! ```ignore
//! use tsic::{SensorType, Tsic};
//!
//! let sensor = Tsic::without_vdd_control(SensorType::Tsic306, /* your hal signal input pin */);
//!
//! let mut delay = /* your hal delay */();
//!
//! match sensor.read(&mut delay) {
//!   Ok(t) => defmt::info!("Temp is: {:f32}", t.as_celsius()),
//!   Err(e) => defmt::warn!("Getting sensor data failed: {:?}", e),
//! };
//! ```
#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tsic/0.3.0")]
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]

use core::time::Duration;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::{InputPin, OutputPin};

/// The spec defines the sample rate as 128kHz, which is 7.8 microseconds. Since
/// we can only sleep for a round number of micros, 8 micros should be close enough.
static STROBE_SAMPLING_RATE: Duration = Duration::from_micros(8);

/// After power up, an initial power up stabilization delay is needed to
/// get reliable measurements.
static VDD_POWER_UP_DELAY: Duration = Duration::from_micros(50);

/// The `Tsic` struct is the main entry point when trying to get a temperature reading from a
/// TSIC 306 sensor.
pub struct Tsic<I: InputPin, O: OutputPin> {
    /// Right now the sensor type is unused since we only support one, but it provides a forward
    /// compatible API in case we add support for more in the future.
    _sensor_type: SensorType,
    signal_pin: I,
    vdd_pin: Option<O>,
}

impl<I: InputPin> Tsic<I, DummyOutputPin> {
    /// Constructs a new `Tsic` without explicit control over the voltage (VDD) pin.
    ///
    /// Use this construction method if you either want to manage the power of your
    /// sensor externally or have it on a permanent voltage connection. Usually in
    /// this case only the signal pin of the sensor is attached to a GPIO pin of
    /// your board.
    ///
    /// *IMPORTANT*: While this sounds like the simpler method, I recommend using
    /// the `with_vdd_control` constructor and also attach the VDD pin of the sensor
    /// to your board. This will reduce the risk of accidentially performing a reading
    /// during the actual temperature transmission. If you still want to use it this
    /// way, you probably want to consider retrying on transient failures when executing
    /// the `read` operation.
    pub fn without_vdd_control(sensor_type: SensorType, signal_pin: I) -> Self {
        Self {
            _sensor_type: sensor_type,
            signal_pin,
            vdd_pin: None,
        }
    }
}

impl<I: InputPin, O: OutputPin> Tsic<I, O> {
    /// Constructs a new `Tsic` with explicit control over the voltage (VDD) pin.
    ///
    /// Use this method if you want the library to control the voltage (VDD) pin of the
    /// sensor as well.
    ///
    /// This is the recommended approach because it saves power and it makes
    /// sure that the readings are very consistent (we do not run the risk of trying to
    /// perform a reading while one is already in-progress, leading to error values).
    ///
    /// Usually you need to assign another GPIO pin as an output pin which can drive around
    /// 3V in high state (see the datasheet for more info), and then the library will control
    /// the power up, initial delay, reading and power down for you transparently. Of course,
    /// you can also use the `without_vdd_control` constructor if you want more manual control
    /// or if you have the sensor on permanent power.
    pub fn with_vdd_control(sensor_type: SensorType, signal_pin: I, vdd_pin: O) -> Self {
        Self {
            _sensor_type: sensor_type,
            signal_pin,
            vdd_pin: Some(vdd_pin),
        }
    }

    /// Attempts to read from the sensor, might fail (see errors for details if so).
    ///
    /// Note that the passed in `Delay` from the HAL needs to be aquired outside of
    /// this struct and passed in as mutable, because to aquire correct data from the
    /// sensor the code needs to pause for a certain amount of microseconds.
    ///
    /// In case there is an error during the read phase and if the `Tsic` has been constructed
    /// to manage the VDD pin as well, it will try to shut it down in a best-effort manner as
    /// well.
    pub fn read<D: DelayUs<u8>>(&mut self, delay: &mut D) -> Result<Temperature, TsicError> {
        self.maybe_power_up_sensor(delay)?;

        let first_packet = match self.read_packet(delay) {
            Ok(packet) => packet,
            Err(err) => {
                self.maybe_power_down_sensor().ok();
                return Err(err);
            }
        };

        let second_packet = match self.read_packet(delay) {
            Ok(packet) => packet,
            Err(err) => {
                self.maybe_power_down_sensor().ok();
                return Err(err);
            }
        };

        self.maybe_power_down_sensor()?;

        Ok(Temperature::new(first_packet, second_packet))
    }

    /// Handle VDD pin power up if set during construction.
    ///
    /// If we are managing the VDD pin for the user, we need to power up the sensor and then
    /// apply an initial delay before the reading can continue.
    fn maybe_power_up_sensor<D: DelayUs<u8>>(&mut self, delay: &mut D) -> Result<(), TsicError> {
        if let Some(ref mut pin) = self.vdd_pin {
            pin.set_high().map_err(|_| TsicError::PinWriteError)?;
            delay.delay_us(VDD_POWER_UP_DELAY.as_micros() as u8);
        }
        Ok(())
    }

    /// Handle VDD pin power down if set during construction.
    ///
    /// If we are managing the VDD pin for the user, at the end of the measurement we need
    /// to power it down at the end as well.
    fn maybe_power_down_sensor(&mut self) -> Result<(), TsicError> {
        if let Some(ref mut pin) = self.vdd_pin {
            pin.set_low().map_err(|_| TsicError::PinWriteError)?;
        }
        Ok(())
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
        self.signal_pin
            .is_high()
            .map_err(|_| TsicError::PinReadError)
    }

    /// Checks if the pin is currently in a low state.
    fn is_low(&self) -> Result<bool, TsicError> {
        self.signal_pin
            .is_low()
            .map_err(|_| TsicError::PinReadError)
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

    /// Failed to read the high/low state of signal the pin.
    PinReadError,

    /// Failed to set the high/low state of the vdd pin.
    PinWriteError,
}

/// Represents a single temperature reading from the TSIC 306 sensor.
pub struct Temperature {
    raw: u16,
}

impl Temperature {
    /// Create a full temperature reading from the two individual half reading packets.
    fn new(first: Packet, second: Packet) -> Self {
        Self {
            raw: (first.value() << 8) | second.value(),
        }
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

/// Refers to the sensor type that is used.
///
/// Note that it does not matter if you use the SOP-8 or the TO92 style
/// sensors as long as the type is correct and the pins are correctly
/// assigned. See the data sheet for more information.
pub enum SensorType {
    /// Use this variant if you use the TSic 306 sensor.
    Tsic306,
}

/// This `OutputPin` is used to satisfy the generics when no explicit pin is provided.
///
/// Note that you do not want to use this struct, I just couldn' figure out a much
/// better way right now, but hopefully it will go away at some point.
pub struct DummyOutputPin {}

impl OutputPin for DummyOutputPin {
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
