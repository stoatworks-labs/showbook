//! `cargo run -p showbook-em --example dump -- <store>` — read an Event Master
//! store and print the show model as JSON.
fn main() {
    let path = std::env::args().nth(1).expect("usage: dump <dir|tar.gz|zip|settings.xml>");
    match showbook_em::import_path(std::path::Path::new(&path)) {
        Ok(show) => println!("{}", serde_json::to_string_pretty(&show).unwrap()),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
