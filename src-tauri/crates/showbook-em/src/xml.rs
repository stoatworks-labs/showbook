//! Small helpers over roxmltree for the Event Master store's shape: every
//! value is an element with text, every item has an `id` attribute, and a
//! missing or `-1` index means "none".

use roxmltree::Node;

pub fn child<'a, 'i>(n: Node<'a, 'i>, tag: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.is_element() && c.tag_name().name() == tag)
}

pub fn children<'a, 'i>(n: Node<'a, 'i>, tag: &'a str) -> impl Iterator<Item = Node<'a, 'i>> + 'a {
    n.children().filter(move |c| c.is_element() && c.tag_name().name() == tag)
}

/// Walk a `/`-separated path of child element names.
pub fn at<'a, 'i>(n: Node<'a, 'i>, path: &str) -> Option<Node<'a, 'i>> {
    let mut cur = n;
    for seg in path.split('/') {
        cur = child(cur, seg)?;
    }
    Some(cur)
}

pub fn text<'a>(n: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    child(n, tag).and_then(|c| c.text()).map(str::trim)
}

pub fn string(n: Node, tag: &str) -> String {
    text(n, tag).unwrap_or("").to_string()
}

pub fn int(n: Node, tag: &str) -> Option<i64> {
    text(n, tag).and_then(|t| t.parse::<i64>().ok().or_else(|| t.parse::<f64>().ok().map(|f| f as i64)))
}

/// An index field where `-1` means none.
pub fn index(n: Node, tag: &str) -> Option<u32> {
    int(n, tag).filter(|v| *v >= 0).map(|v| v as u32)
}

pub fn float(n: Node, tag: &str) -> Option<f64> {
    text(n, tag).and_then(|t| t.parse::<f64>().ok())
}

pub fn flag(n: Node, tag: &str) -> bool {
    int(n, tag).map(|v| v != 0).unwrap_or(false)
}

pub fn id_attr<'a>(n: Node<'a, '_>) -> Option<&'a str> {
    n.attribute("id")
}

pub fn id_num(n: Node) -> Option<u32> {
    id_attr(n).and_then(|s| s.parse().ok())
}

/// Every scalar child of `n`, as JSON, for an `extra` bag. Nested elements
/// are skipped: the caller maps those explicitly or not at all.
pub fn scalars(n: Node) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    for c in n.children().filter(|c| c.is_element()) {
        if c.children().any(|g| g.is_element()) {
            continue;
        }
        let t = c.text().unwrap_or("").trim();
        let v = if let Ok(i) = t.parse::<i64>() {
            serde_json::Value::from(i)
        } else if let Ok(f) = t.parse::<f64>() {
            serde_json::Value::from(f)
        } else {
            serde_json::Value::from(t)
        };
        m.insert(c.tag_name().name().to_string(), v);
    }
    m
}
