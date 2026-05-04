use super::PersonaStore;

#[test]
fn markdown_persona_path_rejects_parent_segments() {
    assert!(PersonaStore::markdown_persona_path("md-../secret").is_err());
    assert!(PersonaStore::markdown_persona_path("md-..").is_err());
}

#[test]
fn markdown_persona_path_rejects_path_separators() {
    assert!(PersonaStore::markdown_persona_path("md-nested/slug").is_err());
    assert!(PersonaStore::markdown_persona_path(r"md-nested\slug").is_err());
}

#[test]
fn markdown_persona_path_accepts_normal_slug() {
    let path = PersonaStore::markdown_persona_path("md-scout").unwrap();
    let file_name = path.file_name().and_then(|name| name.to_str());
    assert_eq!(file_name, Some("scout.md"));
}
