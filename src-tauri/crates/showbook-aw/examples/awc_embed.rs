//! `cargo run -p showbook-aw --example awc_embed -- <host[:port]>`
//!
//! Proves, against a real device or a simulator, that an `.awc` carrying an
//! embedded LivePremier Plus config is still a file the device will take:
//! download one, embed a config, upload it back, and report what the importer
//! made of it. Nothing is applied — upload only extracts.
//!
//! Expected on a LivePremier simulator (6.2.73):
//!
//! ```text
//! downloaded AQL_CONFIG_ZZ9999_….awc  4557389 bytes  entries ["bab89b89"]
//! embedded   4558296 bytes  entries ["bab89b89", "livepremier-plus.json"]
//! manifest   unchanged
//! upload     DONE
//! modules    ["GENERAL", "INPUT", "OUTPUT", …]
//! ```
//!
//! Run it against a device only with permission: it leaves a config in the
//! device's import staging area, which the next real restore would overwrite.
fn main() {
    let host = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:3000".into());
    let dev = showbook_aw::Device::connect(&host, None);

    let (name, awc) = dev.download_config(showbook_aw::awc::SHOW_MODULES).expect("download");
    println!("downloaded {name}  {} bytes  entries {:?}", awc.len(), showbook_aw::awc::entry_names(&awc));

    // A real export from LivePremier Plus when one is given, a stub otherwise.
    let cfg = match std::env::args().nth(2) {
        Some(p) => {
            let bytes = std::fs::read(&p).expect("read the config");
            let c = showbook_aw::lpp::parse(&bytes).expect("that file is not a LivePremier Plus config");
            println!("config     {p} — {:?}", c.summary());
            bytes
        }
        None => {
            let mut c = showbook_aw::lpp::LppConfig { exported: showbook_model::now(), ..Default::default() };
            c.app = showbook_aw::lpp::LppApp { name: "LivePremier Plus".into(), version: "0.9.0".into() };
            c.device.address = host.clone();
            c.show.stack = Some(serde_json::json!({"version": 1, "name": "Embedded probe", "cues": []}));
            c.to_bytes()
        }
    };

    let out = showbook_aw::awc::embed(&awc, showbook_aw::lpp::FILE_NAME, &cfg).expect("embed");
    println!("embedded   {} bytes  entries {:?}", out.len(), showbook_aw::awc::entry_names(&out));

    let (a, b) = (showbook_aw::awc::manifest(&awc), showbook_aw::awc::manifest(&out));
    println!("manifest   {}", if a == b { "unchanged" } else { "CHANGED — the device reads this; stop" });
    assert_eq!(a, b);

    let back = showbook_aw::awc::embedded(&out, showbook_aw::lpp::FILE_NAME).expect("read back");
    assert!(showbook_aw::lpp::is_lpp_config(&back), "the embedded config must parse as one");

    match dev.upload_config(&name, &out) {
        Ok(status) => println!("upload     {status}"),
        Err(e) => {
            eprintln!("upload     FAILED: {e}");
            std::process::exit(1);
        }
    }
    match dev.extracted_modules() {
        Ok(m) => println!("modules    {m:?}"),
        Err(e) => eprintln!("modules    could not read extract status: {e}"),
    }
}
