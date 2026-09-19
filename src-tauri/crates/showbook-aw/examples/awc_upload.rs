//! `cargo run -p showbook-aw --example awc_upload -- <host[:port]> file.awc`
//! — upload an .awc to a device and report the extract status. Nothing is
//! applied; see `Device::apply_config` for that (it reboots the device).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (host, file) = (&args[0], &args[1]);
    let bytes = std::fs::read(file).expect("read");
    let dev = showbook_aw::Device::connect(host, None);
    match dev.upload_config(std::path::Path::new(file).file_name().unwrap().to_str().unwrap(), &bytes) {
        Ok(status) => println!("extract: {status}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1)
        }
    }
    match dev.extracted_modules() {
        Ok(m) => println!("modules in file: {m:?}"),
        Err(e) => eprintln!("could not read extract status over AWJ: {e}"),
    }
}
