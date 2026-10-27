//! TCP connection to a Crossfire server: a reader thread and a writer thread. TLS (a
//! Crossfir3d gateway, its certificate checked against the usual web roots) or plain
//! TCP (a traditional Crossfire server), as the login screen says.

use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rustls::{ClientConfig, ClientConnection, RootCertStore};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, ServerName};

use cfproto::{ClientMsg, ServerMsg};

#[derive(Debug)]
pub enum NetEvent {
    Msg(ServerMsg),
    Disconnected(String),
}

pub struct Net {
    pub tx: Sender<ClientMsg>,
    pub rx: Receiver<NetEvent>,
    stream: TcpStream,
}

impl Drop for Net {
    /// Closes the socket, so the server logs the player out and the reader thread ends.
    fn drop(&mut self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

/// With `setup beat` on, the server expects something from us every 3 s.
const BEAT_AFTER: Duration = Duration::from_millis(2500);

/// How long to wait for a server to answer before giving up (the window does not move
/// while connecting).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Tries each resolved address in turn and returns the first stream that connects.
fn open_stream(addr: &str) -> io::Result<TcpStream> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, format!("no address for {addr}"));
    for a in addr.to_socket_addrs()? {
        match TcpStream::connect_timeout(&a, CONNECT_TIMEOUT) {
            Ok(s) => return Ok(s),
            Err(e) => last = e,
        }
    }
    Err(last)
}

/// The text shown to the player when reading from the server fails.
fn disconnect_reason(e: &io::Error) -> String {
    if e.kind() == io::ErrorKind::UnexpectedEof { "server closed the connection".into() } else { format!("connection lost: {e}") }
}

/// Reader thread body: parses frames into events until the connection ends, then reports why.
fn read_loop(mut reader: impl Read, in_tx: Sender<NetEvent>, dump: Option<PathBuf>) {
    let reason = loop {
        let payload = match cfproto::read_frame(&mut reader) {
            Ok(p) => p,
            Err(e) => break disconnect_reason(&e),
        };
        if let Some(dir) = &dump {
            dump_packet(dir, &payload);
        }
        match cfproto::parse(&payload) {
            Ok(ServerMsg::Goodbye) => {
                let _ = in_tx.send(NetEvent::Msg(ServerMsg::Goodbye));
                break "server said goodbye".to_string();
            }
            Ok(msg) => {
                if in_tx.send(NetEvent::Msg(msg)).is_err() {
                    return; // app is gone
                }
            }
            Err(e) => eprintln!("dropped bad packet: {}", e.0),
        }
    };
    let _ = in_tx.send(NetEvent::Disconnected(reason));
}

/// Writer thread body: sends queued messages, and a Beat whenever the queue stays quiet.
fn write_loop(mut writer: impl Write, out_rx: Receiver<ClientMsg>) {
    loop {
        let msg = match out_rx.recv_timeout(BEAT_AFTER) {
            Ok(m) => m,
            Err(RecvTimeoutError::Timeout) => ClientMsg::Beat,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if cfproto::write_frame(&mut writer, &msg.encode()).is_err() {
            return; // the reader thread reports the disconnect
        }
    }
}

/// The host part of `host:port` (without an IPv6 address's brackets).
fn host_of(addr: &str) -> &str {
    let host = addr.rsplit_once(':').map_or(addr, |(h, _)| h);
    host.trim_start_matches('[').trim_end_matches(']')
}

/// A server on this computer (a developer's own): plain TCP unless the player says otherwise.
pub fn is_local(addr: &str) -> bool {
    let host = host_of(addr);
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// Trusts the usual web roots, plus the PEM certificates in CROSSFIR3D_CA (a test gateway's own).
fn tls_config() -> io::Result<Arc<ClientConfig>> {
    let mut roots = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    if let Some(path) = std::env::var_os("CROSSFIR3D_CA") {
        for cert in CertificateDer::pem_file_iter(&path).map_err(io::Error::other)? {
            roots.add(cert.map_err(io::Error::other)?).map_err(io::Error::other)?;
        }
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider).with_safe_default_protocol_versions().map_err(io::Error::other)?;
    Ok(Arc::new(config.with_root_certificates(roots).with_no_client_auth()))
}

/// Opens the server connection (TLS or plain) and starts its reader and writer threads.
pub fn connect(addr: &str, tls: bool) -> io::Result<Net> {
    let tls = if tls { Some(tls_config()?) } else { None };
    connect_with(addr, tls)
}

/// The servers the player chose to reach without TLS (traditional Crossfire servers),
/// one `host:port` per line; every other server starts as TLS.
pub fn load_plain_servers(path: &Path) -> HashSet<String> {
    std::fs::read_to_string(path).unwrap_or_default().lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
}

pub fn save_plain_servers(path: &Path, servers: &HashSet<String>) -> io::Result<()> {
    let mut lines: Vec<&str> = servers.iter().map(String::as_str).collect();
    lines.sort_unstable();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, lines.join("\n") + "\n")
}

/// Next to the settings.
pub fn default_plain_servers_path() -> PathBuf {
    crate::settings::data_dir().join("plain-servers.txt")
}

fn connect_with(addr: &str, tls: Option<Arc<ClientConfig>>) -> io::Result<Net> {
    let mut stream = open_stream(addr)?;
    stream.set_nodelay(true)?;
    let (tx, out_rx) = mpsc::channel::<ClientMsg>();
    let (in_tx, rx) = mpsc::channel();
    let dump = std::env::var_os("CROSSFIR3D_DUMP").map(PathBuf::from);
    match tls {
        Some(config) => {
            let conn = Arc::new(Mutex::new(handshake(addr, config, &mut stream)?));
            let reader = TlsRead { conn: conn.clone(), sock: stream.try_clone()? };
            let writer = TlsWrite { conn, sock: stream.try_clone()? };
            thread::spawn(move || read_loop(reader, in_tx, dump));
            thread::spawn(move || write_loop(writer, out_rx));
        }
        None => {
            let (reader, writer) = (stream.try_clone()?, stream.try_clone()?);
            thread::spawn(move || read_loop(reader, in_tx, dump));
            thread::spawn(move || write_loop(writer, out_rx));
        }
    }
    Ok(Net { tx, rx, stream })
}

/// The TLS handshake, before the threads start; a silent server gives up like a connect.
fn handshake(addr: &str, config: Arc<ClientConfig>, stream: &mut TcpStream) -> io::Result<ClientConnection> {
    let name = ServerName::try_from(host_of(addr).to_string()).map_err(io::Error::other)?;
    let mut conn = ClientConnection::new(config, name).map_err(io::Error::other)?;
    stream.set_read_timeout(Some(CONNECT_TIMEOUT))?;
    while conn.is_handshaking() {
        conn.complete_io(stream).map_err(|e| {
            // A traditional server answers in plain text, which TLS cannot read.
            io::Error::new(e.kind(), format!("{e}. A traditional Crossfire server needs Secure (TLS) off"))
        })?;
    }
    stream.set_read_timeout(None)?;
    Ok(conn)
}

fn locked(conn: &Mutex<ClientConnection>) -> std::sync::MutexGuard<'_, ClientConnection> {
    conn.lock().unwrap_or_else(|e| e.into_inner())
}

/// The reading half of a TLS connection the writer thread shares. The socket is read
/// without the lock held, so writing never waits for the server to say something.
struct TlsRead {
    conn: Arc<Mutex<ClientConnection>>,
    sock: TcpStream,
}

impl Read for TlsRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match locked(&self.conn).reader().read(buf) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                r => return r,
            }
            // Small reads: their plaintext always fits rustls's buffer, emptied above.
            let mut raw = [0u8; 4096];
            let n = self.sock.read(&mut raw)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            let mut conn = locked(&self.conn);
            let mut rest = &raw[..n];
            while !rest.is_empty() {
                conn.read_tls(&mut rest)?;
                conn.process_new_packets().map_err(io::Error::other)?;
            }
            while conn.wants_write() {
                conn.write_tls(&mut self.sock)?;
            }
        }
    }
}

struct TlsWrite {
    conn: Arc<Mutex<ClientConnection>>,
    sock: TcpStream,
}

impl Write for TlsWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut conn = locked(&self.conn);
        let n = conn.writer().write(buf)?;
        while conn.wants_write() {
            conn.write_tls(&mut self.sock)?;
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn dump_packet(dir: &Path, payload: &[u8]) {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let cmd: String = payload.iter().take_while(|&&b| b != b' ').take(32).map(|&b| b as char).collect();
    let cmd: String = cmd.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    let _ = std::fs::create_dir_all(dir);
    if let Err(e) = std::fs::write(dir.join(format!("{seq:06}-{cmd}.bin")), payload) {
        eprintln!("packet dump failed: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn unreachable_server_gives_up_quickly() {
        // 10.255.255.1 is a private address nobody answers on.
        let start = std::time::Instant::now();
        assert!(connect("10.255.255.1:13327", false).is_err());
        assert!(start.elapsed() < CONNECT_TIMEOUT + Duration::from_secs(2));
    }

    #[test]
    fn disconnect_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::write_frame(&mut s, b"version 1023 1030 x").unwrap();
        });
        let net = connect(&addr, false).unwrap();
        let wait = Duration::from_secs(2);
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Msg(ServerMsg::Version { .. })));
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Disconnected(_)));
    }

    #[test]
    fn goodbye_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let _server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::write_frame(&mut s, b"goodbye").unwrap();
            std::thread::sleep(Duration::from_secs(3)); // keep the socket open
        });
        let net = connect(&addr, false).unwrap();
        let wait = Duration::from_secs(2);
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Msg(ServerMsg::Goodbye)));
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Disconnected(_)));
    }

    #[test]
    fn bad_packet_dropped_not_fatal() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::write_frame(&mut s, b"comc \x00").unwrap();
            cfproto::write_frame(&mut s, b"newmap").unwrap();
        });
        let net = connect(&addr, false).unwrap();
        let wait = Duration::from_secs(2);
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Msg(ServerMsg::NewMap)));
    }

    #[test]
    fn drop_closes_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut buf = [0u8; 1];
            std::io::Read::read(&mut s, &mut buf).map(|n| n == 0).unwrap_or(false)
        });
        drop(connect(&addr, false).unwrap());
        assert!(server.join().unwrap(), "server did not see the connection close");
    }

    #[test]
    fn plain_servers_are_remembered() {
        let path = std::env::temp_dir().join(format!("cf3d-plain-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(load_plain_servers(&path).is_empty());
        let servers: HashSet<String> = ["old.example.org:13327".to_string(), "lan:13327".to_string()].into();
        save_plain_servers(&path, &servers).unwrap();
        assert_eq!(load_plain_servers(&path), servers);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn plain_tcp_to_a_traditional_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::write_frame(&mut s, b"newmap").unwrap();
            std::thread::sleep(Duration::from_secs(1));
        });
        let net = connect(&addr, false).unwrap();
        assert!(matches!(net.rx.recv_timeout(Duration::from_secs(2)).unwrap(), NetEvent::Msg(ServerMsg::NewMap)));
    }

    #[test]
    fn only_this_computer_is_plain() {
        for a in ["localhost:13327", "LOCALHOST:1", "127.0.0.1:13327", "[::1]:13327"] {
            assert!(is_local(a), "{a}");
        }
        for a in ["play.example.org:13327", "192.168.1.5:13327", "[2001:db8::1]:13327", "localhost.example.org:1"] {
            assert!(!is_local(a), "{a}");
        }
        assert_eq!(host_of("[2001:db8::1]:13327"), "2001:db8::1");
    }

    /// A TLS server on a free port with a certificate for "localhost"; the client config trusts it.
    fn tls_server(serve: impl FnOnce(rustls::StreamOwned<rustls::ServerConnection, TcpStream>) + Send + 'static) -> (String, Arc<ClientConfig>) {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let der = cert.cert.der().clone();
        let key = rustls_pki_types::PrivateKeyDer::try_from(cert.signing_key.serialize_der()).unwrap();
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server = rustls::ServerConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![der.clone()], key)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            let conn = rustls::ServerConnection::new(Arc::new(server)).unwrap();
            serve(rustls::StreamOwned::new(conn, s));
        });
        let mut roots = RootCertStore::empty();
        roots.add(der).unwrap();
        let client = ClientConfig::builder_with_provider(provider).with_safe_default_protocol_versions().unwrap();
        (format!("localhost:{port}"), Arc::new(client.with_root_certificates(roots).with_no_client_auth()))
    }

    #[test]
    fn tls_both_ways() {
        let (done_tx, done_rx) = mpsc::channel();
        let (addr, config) = tls_server(move |mut s| {
            cfproto::write_frame(&mut s, b"newmap").unwrap();
            done_tx.send(cfproto::read_frame(&mut s).unwrap()).unwrap();
            std::thread::sleep(Duration::from_secs(1));
        });
        let net = connect_with(&addr, Some(config)).unwrap();
        let wait = Duration::from_secs(3);
        // Writing while the reader thread waits for more must not block.
        net.tx.send(ClientMsg::AskFace(7)).unwrap();
        assert_eq!(done_rx.recv_timeout(wait).unwrap(), b"askface 7");
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Msg(ServerMsg::NewMap)));
    }

    #[test]
    fn untrusted_certificate_is_refused() {
        let (addr, _) = tls_server(|mut s| drop(cfproto::read_frame(&mut s)));
        let err = connect_with(&addr, Some(tls_config().unwrap())).err().expect("refused");
        assert!(err.to_string().to_lowercase().contains("certificate"), "{err}");
    }

    #[test]
    fn writes_framed_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::read_frame(&mut s).unwrap()
        });
        let net = connect(&addr, false).unwrap();
        net.tx.send(ClientMsg::AskFace(3)).unwrap();
        assert_eq!(server.join().unwrap(), b"askface 3");
    }
}
