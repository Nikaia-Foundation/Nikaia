//! **TLS for `net`, on `rustls`** ([ADR-303](../../../docs/specification/adr/adr-303.md)
//! D2: TLS is never ours; [ADR-289](../../../docs/specification/adr/adr-289.md)
//! D34).
//!
//! `net::Tls` is what `tls:` takes: a server's certificate and key read from
//! PEM files (`Tls::files`), or a client's trust in one certificate besides
//! the system's roots (`Tls::trusting`). A [`Session`] is one connection's
//! `rustls` state, driven here by the same readiness waits a plain socket uses
//! ([`crate::rt::io`]), so a handshake, a read and a write give the thread up
//! where the kernel says *not yet* at both values of `user_parallelism`.
//!
//! **`rustls` runs on the `ring` provider** (D34), named on every config rather
//! than installed as a process default, so no other crate's choice can change
//! it.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Instant;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};

use crate::io::IoError;
use crate::net::ReadFailed;
use crate::rt::{Interest, io as readiness};

/// How much one `read` hands back at most, as a plain socket's read does.
const CHUNK: usize = 64 * 1024;

/// **What `tls:` takes** (ADR-289 D34): a server's certificate and key, or a
/// client's trust. Cloning it shares the config, which `rustls` reads and
/// never changes.
#[derive(Clone)]
pub struct Tls {
    side: Side,
}

#[derive(Clone)]
enum Side {
    Server(Arc<rustls::ServerConfig>),
    Client(Arc<rustls::ClientConfig>),
}

impl std::fmt::Debug for Tls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.side {
            Side::Server(_) => f.write_str("net::Tls(server)"),
            Side::Client(_) => f.write_str("net::Tls(client)"),
        }
    }
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// What `rustls` said, with the file it was about.
fn about(file: &str, what: impl std::fmt::Display) -> IoError {
    IoError::Other(format!("{file}: {what}"))
}

/// The bytes of a PEM file, or the error that names it.
fn pem(file: &str) -> Result<Vec<u8>, IoError> {
    std::fs::read(file).map_err(|error| IoError::of(error, file))
}

/// Every certificate in a PEM file; none is an error naming the file.
fn certificates(file: &str) -> Result<Vec<CertificateDer<'static>>, IoError> {
    let bytes = pem(file)?;
    let chain = CertificateDer::pem_slice_iter(&bytes)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| about(file, format!("not a PEM certificate ({error})")))?;
    if chain.is_empty() {
        return Err(about(
            file,
            "no certificate in it (a PEM `BEGIN CERTIFICATE` block is wanted)",
        ));
    }
    Ok(chain)
}

impl Tls {
    /// **A server's TLS**: the certificate chain and the private key, each read
    /// from a PEM file. The errors name the file and what is wrong with it.
    pub async fn files(certificate: &str, key: &str) -> Result<Tls, IoError> {
        let chain = certificates(certificate)?;
        let bytes = pem(key)?;
        let private = PrivateKeyDer::from_pem_slice(&bytes)
            .map_err(|error| about(key, format!("no private key in it ({error})")))?;
        let config = rustls::ServerConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()
            .map_err(|error| IoError::Other(format!("TLS: {error}")))?
            .with_no_client_auth()
            .with_single_cert(chain, private)
            .map_err(|error| about(key, format!("does not suit {certificate} ({error})")))?;
        Ok(Tls {
            side: Side::Server(Arc::new(config)),
        })
    }

    /// **A client's TLS**: it trusts the certificates in a PEM file besides
    /// the system's roots (D34), so a test can talk to its own server.
    pub async fn trusting(certificate: &str) -> Result<Tls, IoError> {
        let mut roots = rustls::RootCertStore::empty();
        // A system without a root store trusts only the file: what could not be
        // read there is not this call's error.
        for root in rustls_native_certs::load_native_certs().certs {
            let _ = roots.add(root);
        }
        for trusted in certificates(certificate)? {
            roots.add(trusted).map_err(|error| {
                about(certificate, format!("not a usable certificate ({error})"))
            })?;
        }
        let config = rustls::ClientConfig::builder_with_provider(provider())
            .with_safe_default_protocol_versions()
            .map_err(|error| IoError::Other(format!("TLS: {error}")))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(Tls {
            side: Side::Client(Arc::new(config)),
        })
    }

    /// The server half, for `net::listen`.
    pub(crate) fn server(self) -> Result<Arc<rustls::ServerConfig>, IoError> {
        match self.side {
            Side::Server(config) => Ok(config),
            Side::Client(_) => Err(IoError::Other(
                "a listener takes `net::Tls::files`, not `net::Tls::trusting`".into(),
            )),
        }
    }

    /// A client session for `address`, whose host part is the name checked
    /// against the server's certificate.
    pub(crate) fn client(self, address: &str) -> Result<Session, IoError> {
        let Side::Client(config) = self.side else {
            return Err(IoError::Other(
                "`net::connect` takes `net::Tls::trusting`, not `net::Tls::files`".into(),
            ));
        };
        let host = address.rsplit_once(':').map_or(address, |(host, _)| host);
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let name = ServerName::try_from(host.to_string())
            .map_err(|error| about(address, format!("not a name TLS can check ({error})")))?;
        let session = rustls::ClientConnection::new(config, name)
            .map_err(|error| about(address, format!("TLS: {error}")))?;
        Ok(Session(Box::new(session.into())))
    }
}

/// **One connection's `rustls` state**: what the peer sent, decrypted, and
/// what this end has to send, encrypted.
pub struct Session(Box<rustls::Connection>);

impl Session {
    /// A server session, its handshake still to come.
    pub(crate) fn server(config: &Arc<rustls::ServerConfig>) -> Result<Session, IoError> {
        let session = rustls::ServerConnection::new(Arc::clone(config))
            .map_err(|error| IoError::Other(format!("TLS: {error}")))?;
        Ok(Session(Box::new(session.into())))
    }

    /// **The handshake**, awaiting the peer where it has not answered yet. A
    /// connection's first read or write runs it too; `Listener::serve` runs it
    /// in the connection's task before the handler, so a failed one ends that
    /// connection alone.
    pub(crate) async fn handshake(&mut self, socket: &TcpStream) -> Result<(), IoError> {
        while self.0.is_handshaking() {
            self.flush(socket).await?;
            if !self.0.is_handshaking() {
                break;
            }
            if !self.0.wants_read() {
                return Err(IoError::Other("TLS: the handshake stalled".into()));
            }
            if !self.fill(socket, None).await.map_err(plain)? {
                return Err(IoError::Other(
                    "TLS: the peer closed during the handshake".into(),
                ));
            }
        }
        self.flush(socket).await
    }

    /// The next plaintext the peer sent; empty where it closed. `deadline`
    /// bounds the waits, as `read(within:)` does on a plain socket.
    pub(crate) async fn read(
        &mut self,
        socket: &TcpStream,
        deadline: Option<Instant>,
    ) -> Result<Vec<u8>, ReadFailed> {
        if self.0.is_handshaking() {
            self.handshake(socket).await?;
        }
        let mut buffer = vec![0_u8; CHUNK];
        loop {
            match self.0.reader().read(&mut buffer) {
                Ok(read) => {
                    buffer.truncate(read);
                    return Ok(buffer);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    self.flush(socket).await?;
                    if !self.fill(socket, deadline).await? {
                        return Ok(Vec::new());
                    }
                }
                // The peer closed the socket without saying so: a close all the
                // same, as on a plain socket.
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(Vec::new());
                }
                Err(error) => return Err(IoError::of(error, "a connection").into()),
            }
        }
    }

    /// All of `bytes`, encrypted and sent, awaiting room where there is none.
    pub(crate) async fn write(&mut self, socket: &TcpStream, bytes: &[u8]) -> Result<(), IoError> {
        if self.0.is_handshaking() {
            self.handshake(socket).await?;
        }
        let mut at = 0;
        while at < bytes.len() {
            let taken = self
                .0
                .writer()
                .write(&bytes[at..])
                .map_err(|error| IoError::of(error, "a connection"))?;
            if taken == 0 && !self.0.wants_write() {
                return Err(IoError::Other("a connection closed while writing".into()));
            }
            at += taken;
            self.flush(socket).await?;
        }
        Ok(())
    }

    /// Says `close_notify` where the socket has room for it now; a close is
    /// never held up for it.
    pub(crate) fn close(&mut self, socket: &TcpStream) {
        self.0.send_close_notify();
        let mut socket = socket;
        while self.0.wants_write() {
            if !matches!(self.0.write_tls(&mut socket), Ok(n) if n > 0) {
                break;
            }
        }
    }

    /// Sends what `rustls` has to send, awaiting room.
    async fn flush(&mut self, socket: &TcpStream) -> Result<(), IoError> {
        while self.0.wants_write() {
            let mut writing = socket;
            match self.0.write_tls(&mut writing) {
                Ok(0) => return Err(IoError::Other("a connection closed while writing".into())),
                Ok(_) => {}
                Err(error) if would_block(&error) => wait(socket, Interest::Writable, None)
                    .await
                    .map_err(plain)?,
                Err(error) => return Err(IoError::of(error, "a connection")),
            }
        }
        Ok(())
    }

    /// Reads what the socket has and decrypts it; `false` where the peer
    /// closed. A record `rustls` refuses sends its alert and is the error.
    async fn fill(
        &mut self,
        socket: &TcpStream,
        deadline: Option<Instant>,
    ) -> Result<bool, ReadFailed> {
        loop {
            let mut reading = socket;
            match self.0.read_tls(&mut reading) {
                Ok(0) => return Ok(false),
                Ok(_) => {
                    if let Err(error) = self.0.process_new_packets() {
                        let _ = self.flush(socket).await;
                        return Err(IoError::Other(format!("TLS: {error}")).into());
                    }
                    return Ok(true);
                }
                Err(error) if would_block(&error) => {
                    wait(socket, Interest::Readable, deadline).await?;
                }
                Err(error) => return Err(IoError::of(error, "a connection").into()),
            }
        }
    }
}

/// A [`ReadFailed`] where no deadline was set, which is always the socket's.
fn plain(failed: ReadFailed) -> IoError {
    match failed {
        ReadFailed::Io(error) => error,
        ReadFailed::TimedOut(_) => IoError::Other("a connection timed out".into()),
    }
}

/// Give the thread up until the socket is ready, or until `deadline`.
async fn wait(
    socket: &TcpStream,
    interest: Interest,
    deadline: Option<Instant>,
) -> Result<(), ReadFailed> {
    let left = match deadline {
        Some(deadline) => {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(ReadFailed::TimedOut(crate::task::TimedOut));
            }
            Some(left)
        }
        None => None,
    };
    let ready = readiness::waiting(socket, interest, left)
        .await
        .map_err(|error| IoError::of(error, "waiting for a socket"))?;
    if !ready {
        return Err(ReadFailed::TimedOut(crate::task::TimedOut));
    }
    Ok(())
}

/// Whether the kernel said *not yet* (or a signal interrupted the call).
fn would_block(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
    )
}
