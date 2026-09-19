//! `cargo run -p showbook-aw --example capture -- <host[:port]>` — read a
//! LivePremier's store into a show and print a summary.
fn main() {
    let host = std::env::args().nth(1).expect("usage: capture <host[:port]>");
    let dev = showbook_aw::Device::connect(&host, None);
    match dev.ping() {
        Ok(v) => eprintln!("system: {v}"),
        Err(e) => eprintln!("ping failed: {e}"),
    }
    let show = dev.read_show().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1)
    });
    let s = showbook_model::summary::Summary::of(&show);
    eprintln!("{}", serde_json::to_string_pretty(&s).unwrap());
    eprintln!("problems: {:?}", show.validate());
    println!("{}", serde_json::to_string_pretty(&show).unwrap());
}
