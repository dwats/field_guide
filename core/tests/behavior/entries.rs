use fieldguide_core::Error;
use fieldguide_core::entry::NewEntry;

use crate::common::*;

fn new_entry(guide_id: &fieldguide_core::GuideId, title: &str) -> NewEntry {
    NewEntry {
        guide_id: guide_id.clone(),
        title: title.to_owned(),
        body_md: String::new(),
        tag_ids: Vec::new(),
    }
}

#[test]
fn titles_are_unique_per_guide_case_insensitively() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let oak = entry(&mut v, &g, "Oak Log", "");
    match v.create_entry(new_entry(&g, " oak log ")) {
        Err(Error::NameTaken {
            name, holder_id, ..
        }) => {
            assert_eq!(name, "oak log");
            assert_eq!(holder_id, oak);
        }
        other => panic!("expected NameTaken, got {other:?}"),
    }
    let other = entry(&mut v, &h, "Oak Log", "");
    assert_ne!(other, oak);
    assert_eq!(browse(&v, &g), ["Oak Log"]);
    assert_eq!(browse(&v, &h), ["Oak Log"]);
}

#[test]
fn aliases_share_the_title_namespace() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    let birch = entry(&mut v, &g, "Birch Log", "");

    // alias against a title
    assert!(matches!(
        v.add_alias(&birch, "oak log"),
        Err(Error::NameTaken { .. })
    ));
    v.add_alias(&oak, "Log").unwrap();
    // alias against an alias, including the entry's own
    assert!(matches!(
        v.add_alias(&birch, "LOG"),
        Err(Error::NameTaken { .. })
    ));
    assert!(matches!(
        v.add_alias(&oak, "log"),
        Err(Error::NameTaken { .. })
    ));
    // title against an alias
    assert!(matches!(
        v.create_entry(new_entry(&g, "log")),
        Err(Error::NameTaken { .. })
    ));
    assert!(matches!(
        v.rename_entry(&birch, "Log"),
        Err(Error::NameTaken { .. })
    ));
    assert_eq!(v.get_entry(&oak).unwrap().aliases, ["Log"]);

    v.remove_alias(&oak, "log").unwrap();
    assert!(v.get_entry(&oak).unwrap().aliases.is_empty());
    assert!(matches!(
        v.remove_alias(&oak, "log"),
        Err(Error::NotFound { .. })
    ));
    entry(&mut v, &g, "Log", "");
}

#[test]
fn wikilinks_resolve_against_titles_and_aliases_case_insensitively() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    v.add_alias(&oak, "Oak Wood").unwrap();
    let found = |text: &str| v.lookup_entry(&g, text).unwrap().map(|n| n.id);
    assert_eq!(found("oak log"), Some(oak.clone()));
    assert_eq!(found(" OAK WOOD "), Some(oak.clone()));
    assert_eq!(found("Oak"), None);
}

#[test]
fn resolution_stays_inside_the_guide_unless_asked() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let h = guide(&mut v, "H");
    let oak = entry(&mut v, &g, "Oak Log", "");
    assert!(v.lookup_entry(&h, "Oak Log").unwrap().is_none());
    let everywhere: Vec<_> = v
        .lookup_entry_any_guide("oak log")
        .unwrap()
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(everywhere, [oak]);
    // A link written in guide H stays unresolved: no mention is recorded.
    let planks = entry(&mut v, &h, "Planks", "made from [[Oak Log]]");
    assert!(v.mentions_from(&planks).unwrap().is_empty());
}

#[test]
fn mentions_follow_the_body() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    let planks = entry(
        &mut v,
        &g,
        "Oak Planks",
        "Crafted from [[Oak Log]] and again [[oak log|logs]]. Not [[Birch Log]] yet.",
    );
    assert_eq!(titles(&v.mentioned_by(&oak).unwrap()), ["Oak Planks"]);
    assert_eq!(titles(&v.mentions_from(&planks).unwrap()), ["Oak Log"]);

    v.update_entry_body(&planks, "Crafted from wood.").unwrap();
    assert!(v.mentioned_by(&oak).unwrap().is_empty());
    assert!(v.mentions_from(&planks).unwrap().is_empty());
}

#[test]
fn a_link_written_before_its_target_exists_resolves_once_the_name_appears() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let planks = entry(&mut v, &g, "Planks", "from [[Oak Log]] or [[Lumber]]");
    assert!(v.mentions_from(&planks).unwrap().is_empty());

    let oak = entry(&mut v, &g, "Oak Log", "");
    assert_eq!(titles(&v.mentioned_by(&oak).unwrap()), ["Planks"]);
    let birch = entry(&mut v, &g, "Birch Log", "");
    v.add_alias(&birch, "Lumber").unwrap();
    assert_eq!(
        titles(&v.mentions_from(&planks).unwrap()),
        ["Birch Log", "Oak Log"]
    );
    v.remove_alias(&birch, "Lumber").unwrap();
    assert_eq!(titles(&v.mentions_from(&planks).unwrap()), ["Oak Log"]);

    // The late-resolved link is a real mention: a rename rewrites it like any other.
    v.rename_entry(&oak, "Oak Wood").unwrap();
    assert_eq!(
        v.get_entry(&planks).unwrap().body_md,
        "from [[Oak Wood]] or [[Lumber]]"
    );

    // Purging unlinks but keeps the text, so a new entry with that name picks the link up again.
    v.trash_entry(&oak).unwrap();
    v.purge_entry(&oak).unwrap();
    assert!(v.mentions_from(&planks).unwrap().is_empty());
    let again = entry(&mut v, &g, "Oak Wood", "");
    assert_eq!(titles(&v.mentioned_by(&again).unwrap()), ["Planks"]);
}

#[test]
fn titles_cannot_be_blank_or_contain_wikilink_syntax() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    for bad in ["", "   ", "a|b", "x]]y", "[[z", "two\nlines"] {
        assert!(
            matches!(
                v.create_entry(new_entry(&g, bad)),
                Err(Error::InvalidName { .. })
            ),
            "{bad:?} was accepted"
        );
    }
    let n = entry(&mut v, &g, "Fine", "");
    assert!(matches!(
        v.add_alias(&n, "a|b"),
        Err(Error::InvalidName { .. })
    ));
}

#[test]
fn a_body_edit_is_stored_verbatim_and_bumps_updated_at() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let id = entry(&mut v, &g, "Entry", "first");
    let before = v.get_entry(&id).unwrap();
    fieldguide_core::clock::advance_for_tests(2);
    let body = "  trailing spaces  \r\nand CRLF\ttabs ";
    let after = v.update_entry_body(&id, body).unwrap();
    assert_eq!(after.body_md, body);
    assert!(after.updated_at > before.updated_at);
    assert_eq!(after.created_at, before.created_at);
}

#[test]
fn a_taken_name_says_which_entry_holds_it_and_how() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stone = entry(&mut v, &g, "Stone Block", "");
    v.add_alias(&stone, "Stone").unwrap();
    let by_title = v.create_entry(new_entry(&g, "stone block")).unwrap_err();
    assert_eq!(
        by_title.to_string(),
        "\"stone block\" is already the title of \"Stone Block\""
    );
    let by_alias = v.create_entry(new_entry(&g, "Stone")).unwrap_err();
    assert_eq!(
        by_alias.to_string(),
        "\"Stone\" is already an alias of \"Stone Block\""
    );

    v.trash_entry(&stone).unwrap();
    let trashed = v.create_entry(new_entry(&g, "Stone Block")).unwrap_err();
    assert_eq!(
        trashed.to_string(),
        "\"Stone Block\" is already the title of \"Stone Block\" (in Trash)"
    );
}

#[test]
fn an_entry_created_with_an_empty_body_starts_as_a_stub_until_its_first_body_edit() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let stubs_only = fieldguide_core::entry::EntryQuery {
        stubs_only: true,
        ..Default::default()
    };
    // New entry, the palette, and a dashed link all create through `create_entry`.
    let empty = v.create_entry(new_entry(&g, "Creeper")).unwrap();
    assert!(empty.is_stub);
    let described = entry(&mut v, &g, "Zombie", "Burns in daylight.");
    assert!(!v.get_entry(&described).unwrap().is_stub);
    assert_eq!(
        titles(&v.list_entries(&g, &stubs_only).unwrap()),
        ["Creeper"]
    );

    v.update_entry_body(&empty.id, "Explodes.").unwrap();
    assert!(!v.get_entry(&empty.id).unwrap().is_stub);
    // Emptying the body again does not make it a stub: it has been described once.
    v.update_entry_body(&empty.id, "").unwrap();
    assert!(!v.get_entry(&empty.id).unwrap().is_stub);
    assert!(v.list_entries(&g, &stubs_only).unwrap().is_empty());
}

#[test]
fn only_http_and_https_web_links_may_be_opened() {
    use fieldguide_core::entry::web_link;
    assert_eq!(
        web_link("  https://example.com/speedrun?x=1#top ").unwrap(),
        "https://example.com/speedrun?x=1#top"
    );
    assert_eq!(
        web_link("HTTP://Example.com").unwrap(),
        "HTTP://Example.com"
    );
    for refused in [
        "file:///C:/Windows/System32/calc.exe",
        "javascript:alert(1)",
        "mailto:someone@example.com",
        "ms-settings:display",
        r"C:\Windows\notepad.exe",
        "example.com",
        "https://",
        "https:///etc/passwd",
        "https://example.com/a b",
        "https://example.com/\n& calc",
        "",
    ] {
        assert!(
            matches!(web_link(refused), Err(Error::Invalid(_))),
            "{refused:?} must be refused"
        );
    }
}
