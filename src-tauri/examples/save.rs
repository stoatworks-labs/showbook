//! `cargo run --example save -- <library dir> <show.json> [message]` — save a
//! show file into a library as a new version of the show whose id it carries,
//! the same way the app's Save version does. For scripts that prepare a show
//! outside the app (the demo state the project video films, for one).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: save <library dir> <show.json> [message]");
        std::process::exit(2);
    }
    let lib = showbook_library::Library::open(std::path::Path::new(&args[0])).expect("open library");
    let bytes = std::fs::read(&args[1]).expect("read show");
    let mut show: showbook_model::Show = serde_json::from_slice(&bytes).expect("parse show");
    let problems = show.validate();
    if !problems.is_empty() {
        eprintln!("{} problem(s):", problems.len());
        for p in &problems {
            eprintln!("  {p}");
        }
        std::process::exit(1);
    }
    let message = args.get(2).map(String::as_str).unwrap_or("Edit");
    match lib.save(&mut show, message, Some("allan")) {
        Ok(Some(c)) => println!("{}: version {} — {}", show.meta.name, c.id, c.message),
        Ok(None) => println!("{}: nothing changed", show.meta.name),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
