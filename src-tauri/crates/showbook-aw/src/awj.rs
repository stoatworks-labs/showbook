//! A blocking AWJ client on TCP 10606, just enough to read a leaf and write a
//! property. The codec, error codes and the letter rule come from
//! `openrcs-awj`; this adds the socket.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use openrcs_awj::{encode_get, encode_replace, Decoder, Frame};
use serde_json::Value;

use crate::{Error, Result};

pub struct Awj {
    stream: TcpStream,
    decoder: Decoder,
}

impl Awj {
    pub fn connect(host: &str) -> Result<Awj> {
        let addr = if host.contains(':') { host.to_string() } else { format!("{host}:{}", openrcs_awj::PORT) };
        let sock = addr
            .to_socket_addrs()
            .map_err(|e| Error::Awj(format!("{addr}: {e}")))?
            .next()
            .ok_or_else(|| Error::Awj(format!("{addr}: no address")))?;
        let stream = TcpStream::connect_timeout(&sock, Duration::from_secs(5)).map_err(|e| Error::Awj(format!("{addr}: {e}")))?;
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        Ok(Awj { stream, decoder: Decoder::new() })
    }

    /// `get` a leaf. Containers read back as `{}`; unknown paths are `E12`.
    pub fn get(&mut self, path: &str) -> Result<Value> {
        self.stream.write_all(encode_get(path).as_bytes()).map_err(|e| Error::Awj(e.to_string()))?;
        let frame = self.read_frame()?;
        match frame {
            Frame::Value { value, .. } => Ok(value),
            Frame::Error(e) => Err(Error::Awj(format!("{path}: {} {}", e.code.as_str(), e.message))),
        }
    }

    /// `replace` a property. Writes are silent: no reply means it was taken.
    pub fn replace(&mut self, path: &str, value: &Value) -> Result<()> {
        self.stream.write_all(encode_replace(path, value).as_bytes()).map_err(|e| Error::Awj(e.to_string()))?;
        // Give the device a moment to answer with an error frame if it has one.
        self.stream.set_read_timeout(Some(Duration::from_millis(150))).ok();
        let mut buf = [0u8; 4096];
        let r = self.stream.read(&mut buf);
        self.stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        if let Ok(n) = r {
            if n > 0 {
                for f in self.decoder.feed(&buf[..n]) {
                    if let Frame::Error(e) = f {
                        return Err(Error::Awj(format!("{path}: {} {}", e.code.as_str(), e.message)));
                    }
                }
            }
        }
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Frame> {
        let mut buf = [0u8; 65536];
        loop {
            let n = self.stream.read(&mut buf).map_err(|e| Error::Awj(e.to_string()))?;
            if n == 0 {
                return Err(Error::Awj("connection closed".into()));
            }
            let mut frames = self.decoder.feed(&buf[..n]);
            if !frames.is_empty() {
                return Ok(frames.remove(0));
            }
        }
    }
}

/// Store path (`device/screenList/items/S1/control/pp/label`) → AWJ path
/// (`DeviceObject/$screen/@items/S1/control/@props/label`). The two
/// namespaces map mechanically: `device` → `DeviceObject`, `xxxList/items/K`
/// → `$xxx/@items/K`, `pp` → `@props`.
pub fn awj_path(store_path: &str) -> String {
    let mut out: Vec<String> = vec![];
    let segs: Vec<&str> = store_path.trim_start_matches('/').split('/').collect();
    let mut i = 0;
    while i < segs.len() {
        let s = segs[i];
        if i == 0 && s == "device" {
            out.push("DeviceObject".into());
        } else if let Some(base) = s.strip_suffix("List") {
            if segs.get(i + 1) == Some(&"items") {
                out.push(format!("${base}"));
                out.push("@items".into());
                i += 1;
            } else {
                out.push(s.into());
            }
        } else if s == "pp" {
            out.push("@props".into());
        } else {
            out.push(s.into());
        }
        i += 1;
    }
    out.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_store_paths_to_awj() {
        assert_eq!(
            awj_path("device/screenList/items/S1/control/pp/label"),
            "DeviceObject/$screen/@items/S1/control/@props/label"
        );
        assert_eq!(
            awj_path("device/system/configuration/backup/import/apply/cmd/pp/xRequest"),
            "DeviceObject/system/configuration/backup/import/apply/cmd/@props/xRequest"
        );
        assert_eq!(
            awj_path("device/presetBank/bankList/items/12/control/pp/label"),
            "DeviceObject/presetBank/$bank/@items/12/control/@props/label"
        );
    }
}
