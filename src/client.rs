//! IEC 60870-5-104 client implementation.
//!
//! This module provides an asynchronous client for connecting to IEC 104 servers.

use std::time::Duration;

use bytes::Bytes;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};
use tokio_util::codec::Framed;

use futures::{SinkExt, StreamExt};

use crate::codec::{Apdu, Iec104Codec};
use crate::error::{Iec104Error, Result};
use crate::types::{Asdu, AsduHeader, Cot, Cp56Time2a, InformationObject, Ioa, TypeId, UFunction};

/// Default IEC 104 port.
pub const DEFAULT_PORT: u16 = 2404;

/// Default T1 timeout (send confirmation) in seconds.
pub const DEFAULT_T1_TIMEOUT: u64 = 15;

/// Default T2 timeout (no data acknowledgment) in seconds.
pub const DEFAULT_T2_TIMEOUT: u64 = 10;

/// Default T3 timeout (test frame) in seconds.
pub const DEFAULT_T3_TIMEOUT: u64 = 20;

/// Default K parameter (max unconfirmed I-frames).
pub const DEFAULT_K: u16 = 12;

/// Default W parameter (max unconfirmed receives before sending S-frame).
pub const DEFAULT_W: u16 = 8;

/// Client configuration.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server address (host:port)
    pub address: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// T1 timeout: time to wait for send confirmation
    pub t1_timeout: Duration,
    /// T2 timeout: time to wait before sending S-frame when no data
    pub t2_timeout: Duration,
    /// T3 timeout: time to wait for test frame response
    pub t3_timeout: Duration,
    /// K parameter: max unconfirmed I-frames
    pub k: u16,
    /// W parameter: max unconfirmed receives before sending S-frame
    pub w: u16,
}

impl ClientConfig {
    /// Create a new configuration with the given address.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            connect_timeout: Duration::from_secs(10),
            t1_timeout: Duration::from_secs(DEFAULT_T1_TIMEOUT),
            t2_timeout: Duration::from_secs(DEFAULT_T2_TIMEOUT),
            t3_timeout: Duration::from_secs(DEFAULT_T3_TIMEOUT),
            k: DEFAULT_K,
            w: DEFAULT_W,
        }
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set T1 timeout.
    pub fn t1_timeout(mut self, timeout: Duration) -> Self {
        self.t1_timeout = timeout;
        self
    }

    /// Set T2 timeout.
    pub fn t2_timeout(mut self, timeout: Duration) -> Self {
        self.t2_timeout = timeout;
        self
    }

    /// Set T3 timeout.
    pub fn t3_timeout(mut self, timeout: Duration) -> Self {
        self.t3_timeout = timeout;
        self
    }
}

/// Connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// TCP connected, waiting for STARTDT
    Connected,
    /// Data transfer active (STARTDT confirmed)
    Active,
    /// Stopping data transfer
    Stopping,
}

/// Events emitted by the client.
#[derive(Debug, Clone)]
pub enum Iec104Event {
    /// Connected to server
    Connected,
    /// Disconnected from server
    Disconnected,
    /// Data transfer started
    DataTransferStarted,
    /// Data transfer stopped
    DataTransferStopped,
    /// Data update with parsed data points
    DataUpdate(Vec<crate::types::DataPoint>),
    /// Received raw ASDU (for types that don't produce DataPoints)
    AsduReceived(Asdu),
    /// Command confirmed
    CommandConfirm {
        /// Information object address
        ioa: u32,
        /// Whether the command was successful
        success: bool,
    },
    /// Interrogation terminated
    InterrogationComplete {
        /// Common address
        common_address: u16,
    },
    /// Error occurred
    Error(String),
}

/// IEC 60870-5-104 client.
pub struct Iec104Client {
    config: ClientConfig,
    state: ConnectionState,
    send_seq: u16,
    recv_seq: u16,
    unconfirmed_sends: u16,
    unconfirmed_recvs: u16,
    event_tx: mpsc::Sender<Iec104Event>,
    event_rx: Option<mpsc::Receiver<Iec104Event>>,
    framed: Option<Framed<TcpStream, Iec104Codec>>,
    last_recv_time: Instant,
    last_send_time: Instant,
}

impl Iec104Client {
    /// Create a new IEC 104 client.
    pub fn new(config: ClientConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            config,
            state: ConnectionState::Disconnected,
            send_seq: 0,
            recv_seq: 0,
            unconfirmed_sends: 0,
            unconfirmed_recvs: 0,
            event_tx,
            event_rx: Some(event_rx),
            framed: None,
            last_recv_time: Instant::now(),
            last_send_time: Instant::now(),
        }
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Subscribe to events.
    ///
    /// This can only be called once. Returns None if already subscribed.
    pub fn subscribe(&mut self) -> Option<mpsc::Receiver<Iec104Event>> {
        self.event_rx.take()
    }

    /// Connect to the server.
    pub async fn connect(&mut self) -> Result<()> {
        if self.state != ConnectionState::Disconnected {
            return Err(Iec104Error::Connection(std::borrow::Cow::Borrowed("Already connected")));
        }

        let stream = timeout(
            self.config.connect_timeout,
            TcpStream::connect(&self.config.address),
        )
        .await
        .map_err(|_| Iec104Error::ConnectionTimeout)?
        .map_err(Iec104Error::Io)?;

        // Disable Nagle's algorithm for low latency
        stream.set_nodelay(true).ok();

        self.framed = Some(Framed::new(stream, Iec104Codec::new()));
        self.state = ConnectionState::Connected;
        self.send_seq = 0;
        self.recv_seq = 0;
        self.unconfirmed_sends = 0;
        self.unconfirmed_recvs = 0;
        self.last_recv_time = Instant::now();
        self.last_send_time = Instant::now();

        self.emit_event(Iec104Event::Connected).await;
        Ok(())
    }

    /// Disconnect from the server.
    pub async fn disconnect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Disconnected {
            return Ok(());
        }

        // Send STOPDT if active
        if self.state == ConnectionState::Active {
            self.stop_dt().await.ok();
        }

        self.framed = None;
        self.state = ConnectionState::Disconnected;
        self.emit_event(Iec104Event::Disconnected).await;
        Ok(())
    }

    /// Start data transfer (STARTDT act).
    pub async fn start_dt(&mut self) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(Iec104Error::protocol_static("Not connected or already active"));
        }

        self.send_u_frame(UFunction::StartDtAct).await?;

        // Wait for confirmation
        let response = self.recv_frame_timeout(self.config.t1_timeout).await?;

        match response.apci {
            crate::types::Apci::UFrame { function: UFunction::StartDtCon } => {
                self.state = ConnectionState::Active;
                self.emit_event(Iec104Event::DataTransferStarted).await;
                Ok(())
            }
            _ => Err(Iec104Error::protocol_static("Unexpected response to STARTDT")),
        }
    }

    /// Stop data transfer (STOPDT act).
    pub async fn stop_dt(&mut self) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::protocol_static("Data transfer not active"));
        }

        self.state = ConnectionState::Stopping;
        self.send_u_frame(UFunction::StopDtAct).await?;

        // Wait for confirmation
        let response = self.recv_frame_timeout(self.config.t1_timeout).await?;

        match response.apci {
            crate::types::Apci::UFrame { function: UFunction::StopDtCon } => {
                self.state = ConnectionState::Connected;
                self.emit_event(Iec104Event::DataTransferStopped).await;
                Ok(())
            }
            _ => Err(Iec104Error::protocol_static("Unexpected response to STOPDT")),
        }
    }

    /// Send general interrogation command.
    pub async fn general_interrogation(&mut self, common_address: u16) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        // QOI = 20 (station interrogation)
        let asdu = Asdu::interrogation_command(common_address, 20);
        self.send_i_frame(asdu).await
    }

    /// Send counter interrogation command.
    pub async fn counter_interrogation(&mut self, common_address: u16, group: u8) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        let mut asdu = Asdu::new(AsduHeader::new(
            TypeId::CounterInterrogation,
            1,
            Cot::Activation,
            common_address,
        ));
        asdu.objects.push(InformationObject {
            ioa: Ioa::new(0),
            data: Bytes::copy_from_slice(&[group]),
        });

        self.send_i_frame(asdu).await
    }

    /// Send clock synchronization command.
    pub async fn clock_sync(&mut self, common_address: u16, time: Cp56Time2a) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        let asdu = Asdu::clock_sync_command(common_address, time);
        self.send_i_frame(asdu).await
    }

    /// Send single command.
    pub async fn single_command(
        &mut self,
        common_address: u16,
        ioa: u32,
        value: bool,
        select: bool,
    ) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        let mut asdu = Asdu::new(AsduHeader::new(
            TypeId::SingleCommand,
            1,
            Cot::Activation,
            common_address,
        ));

        // SCO (Single Command Output):
        // Bit 0: SCS (single command state) - 0=OFF, 1=ON
        // Bit 7: S/E (select/execute) - 0=execute, 1=select
        let sco = if value { 0x01 } else { 0x00 } | if select { 0x80 } else { 0x00 };

        asdu.objects.push(InformationObject {
            ioa: Ioa::new(ioa),
            data: Bytes::copy_from_slice(&[sco]),
        });

        self.send_i_frame(asdu).await
    }

    /// Send double command.
    pub async fn double_command(
        &mut self,
        common_address: u16,
        ioa: u32,
        value: u8,
        select: bool,
    ) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        let mut asdu = Asdu::new(AsduHeader::new(
            TypeId::DoubleCommand,
            1,
            Cot::Activation,
            common_address,
        ));

        // DCO (Double Command Output):
        // Bits 0-1: DCS (double command state) - 1=OFF, 2=ON
        // Bit 7: S/E (select/execute) - 0=execute, 1=select
        let dco = (value & 0x03) | if select { 0x80 } else { 0x00 };

        asdu.objects.push(InformationObject {
            ioa: Ioa::new(ioa),
            data: Bytes::copy_from_slice(&[dco]),
        });

        self.send_i_frame(asdu).await
    }

    /// Send setpoint command (short floating point).
    pub async fn setpoint_float(
        &mut self,
        common_address: u16,
        ioa: u32,
        value: f32,
        select: bool,
    ) -> Result<()> {
        if self.state != ConnectionState::Active {
            return Err(Iec104Error::NotConnected);
        }

        let mut asdu = Asdu::new(AsduHeader::new(
            TypeId::SetpointFloat,
            1,
            Cot::Activation,
            common_address,
        ));

        // Value (4 bytes) + QOS (1 byte)
        let value_bytes = value.to_le_bytes();
        let qos = if select { 0x80 } else { 0x00 };
        let data = [value_bytes[0], value_bytes[1], value_bytes[2], value_bytes[3], qos];

        asdu.objects.push(InformationObject {
            ioa: Ioa::new(ioa),
            data: Bytes::copy_from_slice(&data),
        });

        self.send_i_frame(asdu).await
    }

    /// Process incoming frames.
    ///
    /// This should be called in a loop to handle incoming data.
    pub async fn poll(&mut self) -> Result<Option<Iec104Event>> {
        if self.state == ConnectionState::Disconnected {
            return Err(Iec104Error::NotConnected);
        }

        // Check timeouts and determine actions needed
        let need_test_frame = self.last_recv_time.elapsed() > self.config.t3_timeout;
        let need_s_frame =
            self.unconfirmed_recvs > 0 && self.last_recv_time.elapsed() > self.config.t2_timeout;

        // Check T3 timeout (need to send test frame)
        if need_test_frame {
            self.send_u_frame(UFunction::TestFrAct).await?;
        }

        // Check T2 timeout (need to send S-frame)
        if need_s_frame {
            self.send_s_frame().await?;
        }

        // Try to receive a frame with a short timeout
        let framed = self.framed.as_mut().ok_or(Iec104Error::NotConnected)?;
        match timeout(Duration::from_millis(100), framed.next()).await {
            Ok(Some(Ok(apdu))) => {
                self.last_recv_time = Instant::now();
                self.handle_apdu(apdu).await
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => {
                // Connection closed
                self.state = ConnectionState::Disconnected;
                Err(Iec104Error::Connection(std::borrow::Cow::Borrowed("Connection closed by peer")))
            }
            Err(_) => Ok(None), // Timeout, no data
        }
    }

    // Internal methods

    async fn emit_event(&self, event: Iec104Event) {
        let _ = self.event_tx.send(event).await;
    }

    async fn send_u_frame(&mut self, function: UFunction) -> Result<()> {
        let framed = self.framed.as_mut().ok_or(Iec104Error::NotConnected)?;
        let apdu = Apdu::u_frame(function);
        framed
            .send(apdu)
            .await
            .map_err(|e| Iec104Error::Codec(std::borrow::Cow::Owned(e.to_string())))?;
        self.last_send_time = Instant::now();
        Ok(())
    }

    async fn send_s_frame(&mut self) -> Result<()> {
        let framed = self.framed.as_mut().ok_or(Iec104Error::NotConnected)?;
        let apdu = Apdu::s_frame(self.recv_seq);
        framed
            .send(apdu)
            .await
            .map_err(|e| Iec104Error::Codec(std::borrow::Cow::Owned(e.to_string())))?;
        self.last_send_time = Instant::now();
        self.unconfirmed_recvs = 0;
        Ok(())
    }

    async fn send_i_frame(&mut self, asdu: Asdu) -> Result<()> {
        if self.unconfirmed_sends >= self.config.k {
            return Err(Iec104Error::TooManyUnconfirmed(self.config.k));
        }

        let framed = self.framed.as_mut().ok_or(Iec104Error::NotConnected)?;
        let apdu = Apdu::i_frame(self.send_seq, self.recv_seq, asdu);
        framed
            .send(apdu)
            .await
            .map_err(|e| Iec104Error::Codec(std::borrow::Cow::Owned(e.to_string())))?;

        self.send_seq = (self.send_seq + 1) % 32768;
        self.unconfirmed_sends += 1;
        self.last_send_time = Instant::now();
        self.unconfirmed_recvs = 0; // Piggyback acknowledgment
        Ok(())
    }

    async fn recv_frame_timeout(&mut self, timeout_duration: Duration) -> Result<Apdu> {
        let framed = self.framed.as_mut().ok_or(Iec104Error::NotConnected)?;

        match timeout(timeout_duration, framed.next()).await {
            Ok(Some(Ok(apdu))) => {
                self.last_recv_time = Instant::now();
                Ok(apdu)
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Err(Iec104Error::Connection(std::borrow::Cow::Borrowed("Connection closed"))),
            Err(_) => Err(Iec104Error::T1Timeout),
        }
    }

    async fn handle_apdu(&mut self, apdu: Apdu) -> Result<Option<Iec104Event>> {
        match &apdu.apci {
            crate::types::Apci::IFrame { send_seq, recv_seq } => {
                // Update acknowledgment
                self.acknowledge_up_to(*recv_seq);

                // Validate sequence number
                if *send_seq != self.recv_seq {
                    return Err(Iec104Error::SequenceMismatch {
                        expected: self.recv_seq,
                        actual: *send_seq,
                    });
                }

                self.recv_seq = (self.recv_seq + 1) % 32768;
                self.unconfirmed_recvs += 1;

                // Send S-frame if W threshold reached
                if self.unconfirmed_recvs >= self.config.w {
                    self.send_s_frame().await?;
                }

                // Process ASDU
                if let Some(asdu) = apdu.asdu {
                    return Ok(Some(self.process_asdu(asdu)));
                }
            }

            crate::types::Apci::SFrame { recv_seq } => {
                self.acknowledge_up_to(*recv_seq);
            }

            crate::types::Apci::UFrame { function } => {
                match function {
                    UFunction::TestFrAct => {
                        // Respond with TESTFR con
                        self.send_u_frame(UFunction::TestFrCon).await?;
                    }
                    UFunction::TestFrCon => {
                        // Test frame acknowledged
                    }
                    _ => {
                        // Other U-frames handled elsewhere
                    }
                }
            }
        }

        Ok(None)
    }

    fn acknowledge_up_to(&mut self, recv_seq: u16) {
        // Calculate number of acknowledged frames
        let acked = if recv_seq >= self.send_seq - self.unconfirmed_sends {
            recv_seq - (self.send_seq - self.unconfirmed_sends)
        } else {
            // Wrap around
            (32768 - (self.send_seq - self.unconfirmed_sends)) + recv_seq
        };

        if acked <= self.unconfirmed_sends {
            self.unconfirmed_sends -= acked;
        }
    }

    /// Process received ASDU and convert to appropriate event.
    fn process_asdu(&self, asdu: Asdu) -> Iec104Event {
        use crate::types::TypeId;

        // Check for special COT values
        match asdu.header.cot {
            Cot::ActivationConfirm | Cot::DeactivationConfirm => {
                // Command confirmation - extract IOA from first object if available
                if !asdu.raw_data.is_empty() && asdu.raw_data.len() >= 3 {
                    let ioa = asdu.raw_data[0] as u32
                        | ((asdu.raw_data[1] as u32) << 8)
                        | ((asdu.raw_data[2] as u32) << 16);
                    return Iec104Event::CommandConfirm {
                        ioa,
                        success: !asdu.header.negative,
                    };
                }
            }
            Cot::ActivationTermination => {
                // Interrogation complete
                if asdu.header.type_id == TypeId::InterrogationCommand {
                    return Iec104Event::InterrogationComplete {
                        common_address: asdu.header.common_address,
                    };
                }
            }
            _ => {}
        }

        // Check for negative confirmation (error response)
        if asdu.header.negative {
            return Iec104Event::Error(format!(
                "Negative confirmation for {} (COT={})",
                asdu.header.type_id, asdu.header.cot
            ));
        }

        // Try to parse data points
        match crate::parser::parse_asdu(&asdu) {
            Ok(points) if !points.is_empty() => Iec104Event::DataUpdate(points),
            Ok(_) => {
                // No data points (command types, etc.) - return raw ASDU
                Iec104Event::AsduReceived(asdu)
            }
            Err(e) => {
                // Parse error - return as error event
                Iec104Event::Error(format!("ASDU parse error: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config() {
        let config = ClientConfig::new("192.168.1.100:2404")
            .connect_timeout(Duration::from_secs(5))
            .t1_timeout(Duration::from_secs(10));

        assert_eq!(config.address, "192.168.1.100:2404");
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.t1_timeout, Duration::from_secs(10));
        assert_eq!(config.t2_timeout, Duration::from_secs(DEFAULT_T2_TIMEOUT));
    }

    #[test]
    fn test_client_initial_state() {
        let config = ClientConfig::new("localhost:2404");
        let client = Iec104Client::new(config);

        assert_eq!(client.state(), ConnectionState::Disconnected);
    }
}
