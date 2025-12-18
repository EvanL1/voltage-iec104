//! # voltage_iec104
//!
//! IEC 60870-5-104 protocol implementation for Rust.
//!
//! This crate provides a complete implementation of the IEC 60870-5-104
//! telecontrol protocol, commonly used in power systems and SCADA applications.
//!
//! ## Features
//!
//! - **Event-driven**: Asynchronous data reception via channels
//! - **Full Protocol Support**: I-frames, S-frames, U-frames
//! - **Standard Timeouts**: T1, T2, T3, K, W parameters
//! - **Type Safe**: Strong typing for TypeID, COT, IOA
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use voltage_iec104::{Iec104Client, ClientConfig};
//!
//! #[tokio::main]
//! async fn main() -> voltage_iec104::Result<()> {
//!     let config = ClientConfig::new("192.168.1.100:2404");
//!     let mut client = Iec104Client::new(config);
//!
//!     // Connect and start data transfer
//!     client.connect().await?;
//!     client.start_dt().await?;
//!
//!     // Request general interrogation
//!     client.general_interrogation(1).await?;
//!
//!     // Subscribe to events
//!     let mut events = client.subscribe();
//!     while let Some(event) = events.recv().await {
//!         println!("Event: {:?}", event);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Protocol Overview
//!
//! IEC 60870-5-104 uses TCP/IP for communication (default port 2404).
//! The protocol defines three frame types:
//!
//! - **I-frame**: Information transfer (contains ASDU)
//! - **S-frame**: Supervisory (acknowledgment)
//! - **U-frame**: Unnumbered (control: STARTDT, STOPDT, TESTFR)
//!
//! ### APDU Structure
//!
//! ```text
//! APCI (6 bytes):
//! +--------+--------+--------+--------+--------+--------+
//! | 0x68   | Length | Control Field (4 bytes)           |
//! +--------+--------+--------+--------+--------+--------+
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod client;
pub mod codec;
pub mod error;
pub mod parser;
pub mod types;

// Re-export main types
pub use client::{ClientConfig, ConnectionState, Iec104Client, Iec104Event};
pub use codec::{Apdu, Iec104Codec};
pub use error::{Iec104Error, Result};
pub use parser::parse_asdu;
pub use types::*;
