# tsic-rs

[![Crate][crate-image]][crate-link]
[![Docs][docs-image]][docs-link]
[![Safety Dance][safety-image]][safety-link]

This crate provides a platform-agnostic driver for the TSIC temperature sensors using on top of the [embedded-hal] traits.

## Usage

Add this to your dependencies:

```toml
tsic = "0.2"
```

## Tested Sensors

While more sensors should work, right now I only have access (and therefore tested):

- TSIC 306

I also added experimental support for the following sensors, but note that I do not have access to them so it might not be working (I worked off of the official spec):

- TSIC 206
- TSIC 316
- TSIC 506
- TSIC 516

Note that this driver right now only has support for the digital protocol (ZACWire), so the analog sensors are not supported (201, 301, 203, 303).

[Documentation][docs-link]

## Requirements

In order to run this driver, your actual board needs to provide implementation for these two traits:

- [DelayUs]
- [InputPin]

Also, if you want it to manage your VDD as well (recommended), then:

- [OutputPin]

## Code of Conduct

We abide by the [Contributor Covenant][cc] and ask that you do as well.

For more information, please see [CODE_OF_CONDUCT.md].

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

[//]: # (links)

[safety-image]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety-link]: https://github.com/rust-secure-code/safety-dance/
[crate-link]: https://crates.io/crates/tsic
[crate-image]: https://img.shields.io/crates/v/tsic.svg
[embedded-hal]: https://github.com/rust-embedded/embedded-hal
[cc]: https://contributor-covenant.org
[docs-image]: https://docs.rs/tsic/badge.svg
[docs-link]: https://docs.rs/tsic/
[DelayUs]: https://docs.rs/embedded-hal/0.2.4/embedded_hal/blocking/delay/trait.DelayUs.html
[InputPin]: https://docs.rs/embedded-hal/0.2.4/embedded_hal/digital/v2/trait.InputPin.html
[OutputPin]: https://docs.rs/embedded-hal/0.2.4/embedded_hal/digital/v2/trait.OutputPin.html
[nrf-hal]: https://github.com/nrf-rs/nrf-hal