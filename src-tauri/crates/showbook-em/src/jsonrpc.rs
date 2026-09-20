//! The Event Master JSON-RPC API: HTTP POST of a JSON-RPC 2.0 envelope to
//! `http://<frame>:9999/`, one method per call, as Barco's *Event Master
//! JSON-RPC API* guide documents it. Every reply is `{"jsonrpc":"2.0",
//! "result":{"success":0,"response":…},"id":…}`; `success` is 0 for OK and
//! non-zero for an error, with `response` holding the payload or a message.
//!
//! The simulators that ship with the toolset do not serve this API, so the
//! client here is verified against the guide and the Bitfocus module's use of
//! it, not against a frame.

use serde_json::{json, Value};

use crate::{Error, Result};

/// ureq 3 stops reading a body at 10 MB unless told otherwise, and both an
/// Encore store reply and a frame's backup archive are bigger than that.
pub(crate) const BODY_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

pub const PORT: u16 = 9999;

#[derive(Clone, Debug)]
pub struct Client {
    pub url: String,
    pub timeout_secs: u64,
}

impl Client {
    pub fn new(host: &str) -> Client {
        let url = if host.contains("://") {
            host.trim_end_matches('/').to_string()
        } else if host.contains(':') {
            format!("http://{host}")
        } else {
            format!("http://{host}:{PORT}")
        };
        Client { url, timeout_secs: 10 }
    }

    /// One call. Returns the `response` field of a successful result.
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc": "2.0", "id": "1", "method": method, "params": params});
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(self.timeout_secs))).build(),
        );
        let mut resp = agent
            .post(&self.url)
            .header("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| Error::Device(format!("{method}: {e}")))?;
        // ureq 3 stops at 10 MB by default; a frame's reply to a store query is bigger.
        let v: Value = resp
            .body_mut()
            .with_config()
            .limit(BODY_LIMIT)
            .read_json()
            .map_err(|e| Error::Device(format!("{method}: bad JSON reply: {e}")))?;
        parse_reply(method, v)
    }
}

pub fn parse_reply(method: &str, v: Value) -> Result<Value> {
    if let Some(err) = v.get("error") {
        return Err(Error::Device(format!("{method}: {err}")));
    }
    let result = v.get("result").cloned().ok_or_else(|| Error::Device(format!("{method}: reply has no result")))?;
    let success = result.get("success").and_then(Value::as_i64).unwrap_or(0);
    if success != 0 {
        return Err(Error::Device(format!(
            "{method}: frame answered success={success}: {}",
            result.get("response").map(|r| r.to_string()).unwrap_or_default()
        )));
    }
    Ok(result.get("response").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unwraps_the_envelope() {
        let v = json!({"jsonrpc":"2.0","result":{"success":0,"response":[{"id":0,"Name":"P1"}]},"id":"1"});
        let r = parse_reply("listPresets", v).unwrap();
        assert_eq!(r[0]["Name"], "P1");
    }

    #[test]
    fn reports_failure() {
        let v = json!({"jsonrpc":"2.0","result":{"success":-1,"response":"no such preset"},"id":"1"});
        assert!(parse_reply("activatePreset", v).is_err());
    }

    #[test]
    fn url_forms() {
        assert_eq!(Client::new("192.168.0.175").url, "http://192.168.0.175:9999");
        assert_eq!(Client::new("192.168.0.175:9999").url, "http://192.168.0.175:9999");
        assert_eq!(Client::new("http://e2.local:9999/").url, "http://e2.local:9999");
    }
}
