# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## Unreleased

### Added

 - Make sure that if a strobe length of 0 is decoded, an error is raised instead of
   trying to work with an invalid strobe length.
 - Added experimental support for TSIC 206, TSIC 316, TSIC 506, 516.

## v0.2.1 - 2020-10-12

### Added

 - Added `TemperatureOutOfRange` error and perform range check on TSIC 306.

### Changed

 - Reduce the initial VDD delay down to `50µs`, because at higher temperatures it looks
   like the initial delay reduces on the sensor quite a bit.

## v0.2.0 - 2020-10-12

### Added

 - Added support for VDD management. This is now the recommended approach.
 - Added `SensorType` so that we leave the door open to more sensor support in the future.

### Changed

 - Changed `new` to both `without_vdd_control` or `with_vdd_control` with the new VDD management.
 - Renamed `TsicReadError` to `TsicError` to make it more generic.

## v0.1.0 - 2020-09-28

Initial release