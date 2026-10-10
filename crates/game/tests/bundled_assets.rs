//! Integrated checks on files the game loads from disk at run time.

use std::path::PathBuf;

const TRUETYPE_SIGNATURE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

fn asset_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(relative)
}

fn starts_with_truetype_signature(relative: &str) -> bool {
    std::fs::read(asset_path(relative))
        .map(|bytes| bytes.starts_with(&TRUETYPE_SIGNATURE))
        .unwrap_or(false)
}

#[test]
fn given_bundled_reading_font_when_read_from_disk_then_it_is_truetype() {
    assert!(starts_with_truetype_signature("fonts/Andika-Regular.ttf"));
}

#[test]
fn given_bundled_bold_font_when_read_from_disk_then_it_is_truetype() {
    assert!(starts_with_truetype_signature("fonts/Andika-Bold.ttf"));
}

#[test]
fn given_bundled_fonts_when_listing_licences_then_the_open_font_licence_ships_with_them() {
    let licence = std::fs::read_to_string(asset_path("fonts/OFL.txt")).unwrap_or_default();
    assert!(licence.contains("SIL Open Font License"));
}
