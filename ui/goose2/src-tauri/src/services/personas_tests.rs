use super::PersonaStore;
use crate::types::agents::{Avatar, Persona};

fn make_persona(id: &str, avatar: Option<Avatar>) -> Persona {
    Persona {
        id: id.to_string(),
        display_name: id.to_string(),
        avatar,
        system_prompt: "You are helpful.".to_string(),
        provider: None,
        model: None,
        is_builtin: false,
        is_from_disk: true,
        source_path: None,
        created_at: "2026-04-01T00:00:00Z".to_string(),
        updated_at: "2026-04-01T00:00:00Z".to_string(),
    }
}

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

#[test]
fn local_avatar_reference_check_counts_remaining_personas() {
    let personas = vec![
        make_persona("one", Some(Avatar::Local("shared.png".to_string()))),
        make_persona("two", Some(Avatar::Url("https://example.test/avatar.png".to_string()))),
        make_persona("three", Some(Avatar::Local("other.png".to_string()))),
    ];

    assert!(PersonaStore::is_local_avatar_referenced(
        "shared.png",
        &personas
    ));
    assert!(!PersonaStore::is_local_avatar_referenced(
        "missing.png",
        &personas
    ));
}
