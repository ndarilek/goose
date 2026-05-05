use super::{MarkdownFrontmatter, PersonaStore};
use crate::types::agents::{Avatar, Persona, UpdatePersonaRequest};
use std::fs;
use std::sync::Mutex;
use tempfile::tempdir;

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
        make_persona(
            "two",
            Some(Avatar::Url("https://example.test/avatar.png".to_string())),
        ),
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

#[test]
fn markdown_frontmatter_preserves_unknown_fields_when_serialized() {
    let raw = r#"
name: Reviewer
description: Reviews code changes
provider: openai
tags:
  - review
  - code
owner: Morgan
experimental: true
"#;
    let mut frontmatter: MarkdownFrontmatter = serde_yaml::from_str(raw).unwrap();

    frontmatter.name = "Renamed Reviewer".to_string();
    frontmatter.provider = None;

    let markdown =
        PersonaStore::markdown_from_parts(&frontmatter, "Review the diff carefully.").unwrap();

    assert!(markdown.contains("name: Renamed Reviewer"));
    assert!(!markdown.contains("provider:"));
    assert!(markdown.contains("tags:"));
    assert!(markdown.contains("- review"));
    assert!(markdown.contains("owner: Morgan"));
    assert!(markdown.contains("experimental: true"));
}

#[test]
fn update_markdown_persona_clears_provider_model_through_file_round_trip() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("scout.md");
    fs::write(
        &path,
        "---\nname: Scout\nprovider: openai\nmodel: gpt-4.1\n---\n\nYou are helpful.\n",
    )
    .expect("write persona");
    let persona = PersonaStore::parse_markdown_persona(&path).expect("parse persona");
    let store = PersonaStore {
        personas: Mutex::new(vec![persona]),
    };

    let updated = store
        .update(
            "md-scout",
            UpdatePersonaRequest {
                display_name: None,
                avatar: None,
                system_prompt: None,
                provider: Some(None),
                model: Some(None),
            },
        )
        .expect("update persona");

    assert_eq!(updated.provider, None);
    assert_eq!(updated.model, None);
    let contents = fs::read_to_string(&path).expect("read updated persona");
    assert!(!contents.contains("provider:"));
    assert!(!contents.contains("model:"));

    let reloaded = PersonaStore::parse_markdown_persona(&path).expect("reload persona");
    assert_eq!(reloaded.provider, None);
    assert_eq!(reloaded.model, None);
    assert_eq!(store.get("md-scout").unwrap().provider, None);
    assert_eq!(store.get("md-scout").unwrap().model, None);
}
