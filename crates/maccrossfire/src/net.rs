//! TCP connection to a Crossfire server: a reader thread and a writer thread.

use std::io;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use cfproto::{ClientMsg, ServerMsg};

#[derive(Debug)]
pub enum NetEvent {
    Msg(ServerMsg),
    Disconnected(String),
}

pub struct Net {
    pub tx: Sender<ClientMsg>,
    pub rx: Receiver<NetEvent>,
}

/// The server drops clients that send nothing for 3 s once `setup beat` is on.
const BEAT_AFTER: Duration = Duration::from_millis(2500);

pub fn connect(addr: &str) -> io::Result<Net> {
    let stream = TcpStream::connect(addr)?;
    stream.set_nodelay(true)?;
    let mut reader = stream.try_clone()?;
    let mut writer = stream;
    let (tx, out_rx) = mpsc::channel::<ClientMsg>();
    let (in_tx, rx) = mpsc::channel();
    let dump = std::env::var_os("MACCROSSFIRE_DUMP").map(PathBuf::from);

    thread::spawn(move || {
        let reason = loop {
            let payload = match cfproto::read_frame(&mut reader) {
                Ok(p) => p,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break "server closed the connection".into(),
                Err(e) => break format!("connection lost: {e}"),
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
    });

    thread::spawn(move || {
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
    });

    Ok(Net { tx, rx })
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
    fn disconnect_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::write_frame(&mut s, b"version 1023 1030 x").unwrap();
        });
        let net = connect(&addr).unwrap();
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
        let net = connect(&addr).unwrap();
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
        let net = connect(&addr).unwrap();
        let wait = Duration::from_secs(2);
        assert!(matches!(net.rx.recv_timeout(wait).unwrap(), NetEvent::Msg(ServerMsg::NewMap)));
    }

    #[test]
    fn writes_framed_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            cfproto::read_frame(&mut s).unwrap()
        });
        let net = connect(&addr).unwrap();
        net.tx.send(ClientMsg::AskFace(3)).unwrap();
        assert_eq!(server.join().unwrap(), b"askface 3");
    }
}
