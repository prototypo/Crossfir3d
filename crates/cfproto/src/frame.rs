//! Packet framing: a 2-byte big-endian length (not counting itself), then the payload.

use std::io::{self, Read, Write};

pub fn write_frame(w: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let len = u16::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "packet longer than 65535 bytes"))?;
    let mut buf = Vec::with_capacity(payload.len() + 2);
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(payload);
    w.write_all(&buf)?;
    w.flush()
}

pub fn read_frame(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut len = [0u8; 2];
    r.read_exact(&mut len)?;
    let mut buf = vec![0u8; u16::from_be_bytes(len) as usize];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, b"newmap").unwrap();
        assert_eq!(buf, b"\x00\x06newmap");
        assert_eq!(read_frame(&mut buf.as_slice()).unwrap(), b"newmap");
    }

    #[test]
    fn frame_too_long() {
        assert!(write_frame(&mut Vec::new(), &vec![0u8; 65536]).is_err());
    }
}
