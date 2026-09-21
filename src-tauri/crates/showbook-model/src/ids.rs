//! ID spellings. Drivers build IDs through these so every platform agrees on
//! the prefixes and the inspector can tell an input from an output by eye.

pub fn input(n: impl std::fmt::Display) -> String {
    format!("in:{n}")
}
pub fn source(n: impl std::fmt::Display) -> String {
    format!("src:{n}")
}
pub fn output(n: impl std::fmt::Display) -> String {
    format!("out:{n}")
}
pub fn screen(n: impl std::fmt::Display) -> String {
    format!("scr:{n}")
}
pub fn aux(n: impl std::fmt::Display) -> String {
    format!("aux:{n}")
}
pub fn layer(n: impl std::fmt::Display) -> String {
    format!("layer:{n}")
}
pub fn preset(n: impl std::fmt::Display) -> String {
    format!("pre:{n}")
}
pub fn master(n: impl std::fmt::Display) -> String {
    format!("master:{n}")
}
/// A layer memory / Event Master user key.
pub fn layer_memory(n: impl std::fmt::Display) -> String {
    format!("lmem:{n}")
}
pub fn cue(n: impl std::fmt::Display) -> String {
    format!("cue:{n}")
}
pub fn multiviewer(n: impl std::fmt::Display) -> String {
    format!("mv:{n}")
}
pub fn layout(mv: impl std::fmt::Display, n: impl std::fmt::Display) -> String {
    format!("mvl:{mv}.{n}")
}
pub fn still(n: impl std::fmt::Display) -> String {
    format!("still:{n}")
}
pub fn frame(n: impl std::fmt::Display) -> String {
    format!("frame:{n}")
}
/// `conn:<frame>.<slot>.<connector>`
pub fn connector(frame: impl std::fmt::Display, slot: impl std::fmt::Display, conn: impl std::fmt::Display) -> String {
    format!("conn:{frame}.{slot}.{conn}")
}

/// The part after the prefix, for display: `pre:12` → `12`.
pub fn tail(id: &str) -> &str {
    id.split_once(':').map(|(_, t)| t).unwrap_or(id)
}
