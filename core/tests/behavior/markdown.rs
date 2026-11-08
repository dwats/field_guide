use crate::common::*;

#[test]
fn preview_resolves_links_through_titles_and_aliases_and_skips_trashed_notes() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = note(&mut v, &g, "Oak Log", "");
    v.add_alias(&oak, "Logs").unwrap();
    let gone = note(&mut v, &g, "Gone", "");
    v.trash_note(&gone).unwrap();
    let page = note(
        &mut v,
        &g,
        "Page",
        "[[Oak Log]] [[logs|some logs]] [[Gone]] [[Nothing]]",
    );

    let r = v.render_note(&page).unwrap();
    assert_eq!(r.linked_note_ids, std::slice::from_ref(&oak));
    assert_eq!(r.unresolved_links, ["Gone", "Nothing"]);
    assert!(
        r.html
            .contains(&format!("data-note-id=\"{}\">Oak Log</a>", oak.as_str())),
        "{}",
        r.html
    );
    assert!(
        r.html
            .contains(&format!("data-note-id=\"{}\">some logs</a>", oak.as_str())),
        "{}",
        r.html
    );
    assert!(r.html.contains("data-target=\"Nothing\""), "{}", r.html);
}

#[test]
fn preview_of_unsaved_text_resolves_against_the_guide_and_not_across_guides() {
    let mut v = vault();
    let a = guide(&mut v, "A");
    let b = guide(&mut v, "B");
    let oak = note(&mut v, &a, "Oak Log", "");

    let same_guide = v.render_markdown(&a, "draft: [[Oak Log]]").unwrap();
    assert_eq!(same_guide.linked_note_ids, [oak]);

    let other_guide = v.render_markdown(&b, "draft: [[Oak Log]]").unwrap();
    assert!(other_guide.linked_note_ids.is_empty());
    assert_eq!(other_guide.unresolved_links, ["Oak Log"]);
}
