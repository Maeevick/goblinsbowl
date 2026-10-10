//! Integrated check: the `rules.toml` shipped at the repository root is the coded default ruleset.

use std::path::PathBuf;

use goblins_rules::ruleset::Ruleset;

fn shipped_rules_text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../rules.toml");
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn given_shipped_rules_file_when_read_then_it_matches_the_default_ruleset() {
    assert_eq!(
        Ruleset::from_toml_str(&shipped_rules_text()),
        Ok(Ruleset::default())
    );
}
