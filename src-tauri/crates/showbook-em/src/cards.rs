//! Card and frame type codes, as the simulator's `simhw*.xml` files and the
//! Encore3 `hwconfig.xml` name them. Codes not listed here are reported as
//! `Card type N` rather than guessed.

use showbook_model::{ConnectorKind, Direction};

/// What a card type code means: its name and the connectors it carries, in
/// panel order. `None` connectors means the card has none we map (VPU, link).
pub struct CardInfo {
    pub name: &'static str,
    pub connectors: &'static [(ConnectorKind, Direction, &'static str)],
}

const SDI3G: (ConnectorKind, Direction, &str) = (ConnectorKind::Sdi, Direction::In, "3G-SDI");
const SDI12G: (ConnectorKind, Direction, &str) = (ConnectorKind::Sdi, Direction::In, "12G-SDI");
const SDI3G_OUT: (ConnectorKind, Direction, &str) = (ConnectorKind::Sdi, Direction::Out, "3G-SDI");
const SDI12G_OUT: (ConnectorKind, Direction, &str) = (ConnectorKind::Sdi, Direction::Out, "12G-SDI");
const HDMI14: (ConnectorKind, Direction, &str) = (ConnectorKind::Hdmi, Direction::In, "HDMI 1.4a");
const HDMI20: (ConnectorKind, Direction, &str) = (ConnectorKind::Hdmi, Direction::In, "HDMI 2.0");
const HDMI14_OUT: (ConnectorKind, Direction, &str) = (ConnectorKind::Hdmi, Direction::Out, "HDMI 1.4a");
const HDMI20_OUT: (ConnectorKind, Direction, &str) = (ConnectorKind::Hdmi, Direction::Out, "HDMI 2.0");
const DP11: (ConnectorKind, Direction, &str) = (ConnectorKind::DisplayPort, Direction::In, "DisplayPort 1.1");
const DP12: (ConnectorKind, Direction, &str) = (ConnectorKind::DisplayPort, Direction::In, "DisplayPort 1.2");
const DP12_OUT: (ConnectorKind, Direction, &str) = (ConnectorKind::DisplayPort, Direction::Out, "DisplayPort 1.2");
const DVI: (ConnectorKind, Direction, &str) = (ConnectorKind::Dvi, Direction::In, "DVI-I");
const LINK: (ConnectorKind, Direction, &str) = (ConnectorKind::Link, Direction::In, "Link");

pub fn card(code: i64) -> Option<CardInfo> {
    Some(match code {
        0 => CardInfo { name: "2x DVI input card", connectors: &[DVI, DVI] },
        1 => CardInfo { name: "4x 3G-SDI input card", connectors: &[SDI3G, SDI3G, SDI3G, SDI3G] },
        2 => CardInfo { name: "2x HDMI 1.4a + 2x DP 1.1 input card", connectors: &[HDMI14, HDMI14, DP11, DP11] },
        3 => CardInfo {
            name: "Tri-Combo input card",
            connectors: &[DP12, HDMI20, SDI12G, SDI12G, SDI12G, SDI12G],
        },
        4 => CardInfo { name: "4x HDMI 2.0 input card", connectors: &[HDMI20, HDMI20, HDMI20, HDMI20] },
        5 => CardInfo { name: "4x DisplayPort 1.2 input card", connectors: &[DP12, DP12, DP12, DP12] },
        11 => CardInfo { name: "PDS-4K HDMI/SDI input", connectors: &[] },
        12 => CardInfo { name: "PDS-4K DP/Dante mezzanine", connectors: &[] },
        21 => CardInfo { name: "4x 3G-SDI output card", connectors: &[SDI3G_OUT, SDI3G_OUT, SDI3G_OUT, SDI3G_OUT] },
        22 => CardInfo { name: "4x HDMI 1.4a output card", connectors: &[HDMI14_OUT, HDMI14_OUT, HDMI14_OUT, HDMI14_OUT] },
        23 => CardInfo { name: "4x DisplayPort output card", connectors: &[DP12_OUT, DP12_OUT, DP12_OUT, DP12_OUT] },
        25 => CardInfo {
            name: "Tri-Combo output card",
            connectors: &[DP12_OUT, HDMI20_OUT, SDI12G_OUT, SDI12G_OUT, SDI12G_OUT, SDI12G_OUT],
        },
        26 => CardInfo { name: "4x HDMI 2.0 output card", connectors: &[HDMI20_OUT, HDMI20_OUT, HDMI20_OUT, HDMI20_OUT] },
        30 => CardInfo { name: "PDS-4K HDMI/SDI output", connectors: &[] },
        40 => CardInfo { name: "Multiviewer output card", connectors: &[HDMI14_OUT, HDMI14_OUT] },
        42 => CardInfo { name: "4x HDMI 2.0 multiviewer output card", connectors: &[HDMI20_OUT, HDMI20_OUT, HDMI20_OUT, HDMI20_OUT] },
        43 => CardInfo { name: "PDS-4K multiviewer output", connectors: &[] },
        50 => CardInfo { name: "VPU (video processing unit)", connectors: &[] },
        70 => CardInfo { name: "Link / expansion card", connectors: &[LINK, LINK, LINK, LINK] },
        _ => return None,
    })
}

pub fn card_name(code: i64) -> String {
    match card(code) {
        Some(c) => c.name.to_string(),
        None => format!("Card type {code}"),
    }
}

/// Frame type codes, from the simulator's `<frametype type=N>`.
pub fn frame_model(code: i64) -> &'static str {
    match code {
        0 => "E2",
        1 => "S3-4K",
        2 => "EX",
        3 => "ImagePRO-4K",
        5 => "E2 Gen 2",
        6 => "PDS-4K",
        8 => "Encore3",
        _ => "Event Master",
    }
}
