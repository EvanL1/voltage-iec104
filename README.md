# voltage_iec104

IEC 60870-5-104 protocol implementation for Rust.

## Overview

`voltage_iec104` is a pure Rust implementation of the IEC 60870-5-104 protocol, commonly used in SCADA systems for telecontrol applications in power systems.

## Features

- Async/await based on Tokio
- Client implementation for IEC 104 communication
- Support for standard ASDU types (M_SP_NA, M_DP_NA, M_ME_NA, etc.)
- Configurable connection parameters
- Optional tracing support for debugging

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
voltage_iec104 = "0.1"
```

## Quick Start

```rust
use voltage_iec104::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::new("192.168.1.100:2404");
    let mut client = Client::new(config);

    client.connect().await?;
    client.start_data_transfer().await?;

    // Receive data...

    Ok(())
}
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
