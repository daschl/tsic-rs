# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

### Added

### Changed

## v0.2.0 - 2020-10-12

### Added

 - Added support for VDD management. This is now the recommended approach.
 - Added `SensorType` so that we leave the door open to more sensor support in the future.

### Changed

 - Changed `new` to both `without_vdd_control` or `with_vdd_control` with the new VDD management.
 - Renamed `TsicReadError` to `TsicError` to make it more generic.

## v0.1.0 - 2020-09-28

Initial release