# filetime

[![Build Status](https://travis-ci.org/alexcrichton/filetime.svg?branch=master)](https://travis-ci.org/alexcrichton/filetime)
[![Build status](https://ci.appveyor.com/api/projects/status/9tatexq47i3ee13k?svg=true)](https://ci.appveyor.com/project/alexcrichton/filetime)

[Documentation](https://docs.rs/filetime)

A helper library for inspecting and seting the various timestamps of files in Rust. This
library takes into account cross-platform differences in terms of where the
timestamps are located, what they are called, and how to convert them into a
platform-independent representation.

```toml
# Cargo.toml
[dependencies]
filetime = "0.2"
```

# Advantages over using `std::fs::Metadata`

This library includes the ability to set this data, which std dose not.

This library, when built with debug_assertions and with the variable `FILETIME_EMULATE_SECOND_ONLY_SYSTOM` set, will return all times rounded down to the second. This emulates the behavior of some file systems, mostly [HFS](https://en.wikipedia.org/wiki/HFS_Plus), allowing debugging on other hardware.

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Filetime by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
