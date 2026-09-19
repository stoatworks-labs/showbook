//! `cargo run -p showbook-aw --example awc -- <host[:port]> [out.awc]` —
//! download the device's configuration as an .awc and print its manifest.
//! With a file argument only (`--read file.awc`), print that file's manifest.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--read") {
        let bytes = std::fs::read(&args[1]).expect("read");
        println!("{}", serde_json::to_string_pretty(&showbook_aw::awc::manifest(&bytes)).unwrap());
        return;
    }
    let host = args.first().expect("usage: awc <host[:port]> [out.awc] | --read file.awc");
    let dev = showbook_aw::Device::connect(host, None);
    let (name, bytes) = dev.download_config(showbook_aw::awc::SHOW_MODULES).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1)
    });
    eprintln!("{name}: {} bytes", bytes.len());
    println!("{}", serde_json::to_string_pretty(&showbook_aw::awc::manifest(&bytes)).unwrap());
    if let Some(out) = args.get(1) {
        std::fs::write(out, &bytes).expect("write");
        eprintln!("wrote {out}");
    }
}
