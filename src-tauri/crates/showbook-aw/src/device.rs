//! A LivePremier on the network: the Web RCS HTTP server (port 80 on a
//! device, 3000 on the simulator), the documented REST API under
//! `/api/tpp/v1`, and AWJ on 10606.

use std::io::Read;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use showbook_model::{ids, PresetTarget, Show};

use crate::awj::{awj_path, Awj};
use crate::{Error, Result};

#[derive(Clone, Debug)]
pub struct Device {
    /// Host or host:port of the Web RCS HTTP server.
    pub host: String,
    /// Host or host:port of the AWJ interpreter (10606 by default).
    pub awj_host: String,
}

impl Device {
    /// `host` may carry the HTTP port (`192.168.2.140` or `127.0.0.1:3000`);
    /// AWJ is assumed on the same address, port 10606, unless given.
    pub fn connect(host: &str, awj_host: Option<&str>) -> Device {
        let bare = host.split(':').next().unwrap_or(host).to_string();
        Device { host: host.to_string(), awj_host: awj_host.map(str::to_string).unwrap_or(bare) }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.host.trim_end_matches('/'), path)
    }

    fn agent(&self, secs: u64) -> ureq::Agent {
        ureq::AgentBuilder::new().timeout(Duration::from_secs(secs)).build()
    }

    /// Cheap liveness check: the REST API's system call.
    pub fn ping(&self) -> Result<Value> {
        let r = self.agent(5).get(&self.url("/api/tpp/v1/system")).call().map_err(|e| Error::Device(e.to_string()))?;
        r.into_json().map_err(|e| Error::Device(e.to_string()))
    }

    /// The entire device store. Large (~124 MB on a Cmax); allow minutes.
    pub fn fetch_store(&self) -> Result<Value> {
        let r = self.agent(300).get(&self.url("/api/stores/device")).call().map_err(|e| Error::Device(e.to_string()))?;
        r.into_json().map_err(|e| Error::Device(format!("store is not JSON: {e}")))
    }

    pub fn read_show(&self) -> Result<Show> {
        let store = self.fetch_store()?;
        let mut show = crate::store::parse(&store)?;
        show.meta.source = Some(showbook_model::SourceInfo {
            kind: "device".into(),
            origin: self.host.clone(),
            at: showbook_model::now(),
            firmware: Some(show.system.firmware.clone()),
        });
        Ok(show)
    }

    /// Ask the device to export the given modules and download the `.awc`.
    /// Returns the file name the device chose and the bytes.
    pub fn download_config(&self, modules: &[&str]) -> Result<(String, Vec<u8>)> {
        let url = format!("{}?modules={}", self.url("/api/device/hardware/config/download"), modules.join(","));
        let r = self.agent(600).get(&url).call().map_err(|e| Error::Device(e.to_string()))?;
        let name = r
            .header("Content-Disposition")
            .and_then(|cd| cd.split("filename=").nth(1))
            .map(|f| f.trim_matches('"').trim().to_string())
            .unwrap_or_else(|| "config.awc".into());
        let mut buf = vec![];
        r.into_reader().read_to_end(&mut buf)?;
        if !crate::awc::is_awc(&buf) {
            return Err(Error::Device(format!("the device did not return an .awc ({} bytes)", buf.len())));
        }
        Ok((name, buf))
    }

    /// Upload an `.awc`. The Web RCS server extracts it on the device and
    /// answers with the extract status (`DONE` / `DONE_VERSION_WARNING`);
    /// nothing is applied until [`Device::apply_config`].
    pub fn upload_config(&self, filename: &str, bytes: &[u8]) -> Result<String> {
        let boundary = format!("----showbook{}", Instant::now().elapsed().as_nanos());
        let mut body = vec![];
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"FILE\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let r = self
            .agent(600)
            .post(&self.url("/api/device/hardware/config/upload"))
            .set("Content-Type", &format!("multipart/form-data; boundary={boundary}"))
            .send_bytes(&body)
            .map_err(|e| Error::Device(e.to_string()))?;
        r.into_string().map_err(|e| Error::Device(e.to_string()))
    }

    /// Which modules the last uploaded file holds, from the extract status.
    pub fn extracted_modules(&self) -> Result<Vec<String>> {
        let mut awj = Awj::connect(&self.awj_host)?;
        let v = awj.get(&awj_path("device/system/configuration/backup/import/extract/status/pp/module"))?;
        Ok(v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default())
    }

    /// Apply the extracted configuration's modules. The device reboots for
    /// most module sets; this returns once the status leaves `IN_PROGRESS`
    /// or the device stops answering (which is the reboot).
    pub fn apply_config(&self, modules: &[&str], still_option: &str) -> Result<String> {
        let mut awj = Awj::connect(&self.awj_host)?;
        awj.replace(&awj_path("device/system/configuration/backup/import/apply/cmd/pp/stillOption"), &json!(still_option))?;
        awj.replace(&awj_path("device/system/configuration/backup/import/apply/cmd/pp/xRequest"), &json!(modules))?;
        let status_path = awj_path("device/system/configuration/backup/import/apply/status/pp/status");
        let start = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            match awj.get(&status_path) {
                Ok(v) => {
                    let st = v.as_str().unwrap_or("").to_string();
                    if st != "IN_PROGRESS" && st != "NO_REQUEST" {
                        return Ok(st);
                    }
                    if st == "NO_REQUEST" && start.elapsed() > Duration::from_secs(5) {
                        return Ok(st);
                    }
                }
                Err(_) => return Ok("REBOOT_STARTED".into()),
            }
            if start.elapsed() > Duration::from_secs(300) {
                return Err(Error::Device("apply did not finish in 5 minutes".into()));
            }
        }
    }

    // ---- REST (documented) ------------------------------------------

    fn post_json(&self, path: &str, body: Value) -> Result<Value> {
        let r = self.agent(15).post(&self.url(path)).send_json(body).map_err(|e| Error::Device(e.to_string()))?;
        let text = r.into_string().unwrap_or_default();
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    /// Recall memory `slot` on screen `n` to preview or program.
    pub fn recall_screen_memory(&self, screen: u32, slot: u32, program: bool) -> Result<()> {
        self.post_json(&format!("/api/tpp/v1/screens/{screen}/load-memory"), json!({"memoryId": slot, "target": if program { "program" } else { "preview" }}))?;
        Ok(())
    }

    pub fn recall_aux_memory(&self, aux: u32, slot: u32, program: bool) -> Result<()> {
        self.post_json(&format!("/api/tpp/v1/auxiliary-screens/{aux}/load-memory"), json!({"memoryId": slot, "target": if program { "program" } else { "preview" }}))?;
        Ok(())
    }

    pub fn recall_master_memory(&self, slot: u32, program: bool) -> Result<()> {
        self.post_json("/api/tpp/v1/load-master-memory", json!({"memoryId": slot, "target": if program { "program" } else { "preview" }}))?;
        Ok(())
    }

    /// TAKE on the given screens and auxes (all of them when both are empty).
    pub fn take(&self, screens: &[u32], auxes: &[u32]) -> Result<()> {
        let mut body = json!({});
        if !screens.is_empty() {
            body["screenIds"] = json!(screens);
        }
        if !auxes.is_empty() {
            body["auxiliaryScreenIds"] = json!(auxes);
        }
        self.post_json("/api/tpp/v1/take", body)?;
        Ok(())
    }

    /// Read a screen layer's preset side (`preview`/`program`) over REST.
    pub fn read_layer(&self, screen: u32, layer: u32, program: bool) -> Result<Value> {
        let r = self
            .agent(15)
            .get(&self.url(&format!("/api/tpp/v1/screens/{screen}/layers/{layer}/presets/{}", if program { "program" } else { "preview" })))
            .call()
            .map_err(|e| Error::Device(e.to_string()))?;
        r.into_json().map_err(|e| Error::Device(e.to_string()))
    }

    // ---- AWJ (labels and single properties) -------------------------

    pub fn set_screen_label(&self, screen_key: &str, label: &str) -> Result<()> {
        let mut awj = Awj::connect(&self.awj_host)?;
        let list = if screen_key.starts_with('A') { "auxiliaryList" } else { "screenList" };
        awj.replace(&awj_path(&format!("device/{list}/items/{screen_key}/control/pp/label")), &json!(label))
    }

    pub fn set_input_label(&self, input_key: &str, label: &str) -> Result<()> {
        let mut awj = Awj::connect(&self.awj_host)?;
        awj.replace(&awj_path(&format!("device/inputList/items/{input_key}/control/pp/label")), &json!(label))
    }

    pub fn set_output_label(&self, output_key: &str, label: &str) -> Result<()> {
        let mut awj = Awj::connect(&self.awj_host)?;
        awj.replace(&awj_path(&format!("device/outputList/items/{output_key}/control/pp/label")), &json!(label))
    }

    pub fn set_memory_label(&self, slot: u32, label: &str) -> Result<()> {
        let mut awj = Awj::connect(&self.awj_host)?;
        awj.replace(&awj_path(&format!("device/presetBank/bankList/items/{slot}/control/pp/label")), &json!(label))
    }

    /// Put a source on a layer of a screen's preview: `IN_3`, `STILL_1`, `NONE`.
    pub fn set_layer_source(&self, screen_key: &str, letter: char, layer_key: &str, source: &str) -> Result<()> {
        let mut awj = Awj::connect(&self.awj_host)?;
        let list = if screen_key.starts_with('A') { "auxiliaryList" } else { "screenList" };
        awj.replace(
            &awj_path(&format!("device/{list}/items/{screen_key}/presetList/items/{letter}/layerList/items/{layer_key}/source/pp/inputNum")),
            &json!(source),
        )
    }

    /// Read a memory's layer values by recalling it into the preview of each
    /// screen it applies to and reading that preview back from the store.
    /// **This writes to the device's preview buffers.** The caller decides
    /// which screens; nothing is taken to program.
    pub fn deep_capture(&self, show: &mut Show, screens: &[String]) -> Result<usize> {
        let mut captured = 0;
        let slots: Vec<(String, u32)> = show.presets.iter().filter_map(|p| p.number.map(|n| (p.id.clone(), n))).collect();
        for (pid, slot) in slots {
            let mut targets = vec![];
            for sid in screens {
                let key = ids::tail(sid).to_string();
                let Ok(n) = key.trim_start_matches(['S', 'A']).parse::<u32>() else { continue };
                let is_aux = key.starts_with('A');
                let ok = if is_aux { self.recall_aux_memory(n, slot, false) } else { self.recall_screen_memory(n, slot, false) };
                if ok.is_err() {
                    continue;
                }
                std::thread::sleep(Duration::from_millis(400));
                let store = self.fetch_store()?;
                let fresh = crate::store::parse(&store)?;
                if let Some(sc) = fresh.screens.iter().find(|x| &x.id == sid) {
                    if let Some(pv) = sc.extra.get("previewState") {
                        if let Ok(t) = serde_json::from_value::<PresetTarget>(pv.clone()) {
                            targets.push(t);
                        }
                    }
                }
            }
            if let Some(p) = show.presets.iter_mut().find(|p| p.id == pid) {
                if !targets.is_empty() {
                    p.targets = targets;
                    captured += 1;
                }
            }
        }
        Ok(captured)
    }
}
