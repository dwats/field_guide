use fieldguide_core::markdown::{RenderOptions, count_tasks, render, render_with, toggle_task};

use crate::common::*;

#[test]
fn preview_resolves_links_through_titles_and_aliases_and_skips_trashed_entries() {
    let mut v = vault();
    let g = guide(&mut v, "G");
    let oak = entry(&mut v, &g, "Oak Log", "");
    v.add_alias(&oak, "Logs").unwrap();
    let gone = entry(&mut v, &g, "Gone", "");
    v.trash_entry(&gone).unwrap();
    let page = entry(
        &mut v,
        &g,
        "Page",
        "[[Oak Log]] [[logs|some logs]] [[Gone]] [[Nothing]]",
    );

    let r = v.render_entry(&page).unwrap();
    assert_eq!(r.linked_entry_ids, std::slice::from_ref(&oak));
    assert_eq!(r.unresolved_links, ["Gone", "Nothing"]);
    assert!(
        r.html
            .contains(&format!("data-entry-id=\"{}\">Oak Log</a>", oak.as_str())),
        "{}",
        r.html
    );
    assert!(
        r.html
            .contains(&format!("data-entry-id=\"{}\">some logs</a>", oak.as_str())),
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
    let oak = entry(&mut v, &a, "Oak Log", "");

    let same_guide = v.render_markdown(&a, "draft: [[Oak Log]]").unwrap();
    assert_eq!(same_guide.linked_entry_ids, [oak]);

    let other_guide = v.render_markdown(&b, "draft: [[Oak Log]]").unwrap();
    assert!(other_guide.linked_entry_ids.is_empty());
    assert_eq!(other_guide.unresolved_links, ["Oak Log"]);
}

#[test]
fn toggle_task_flips_the_nth_marker_in_document_order() {
    let body = "- [ ] dig\n* [x] mine\n+ [X] smelt\n1. [ ] first\n2) [x] second\n";
    assert_eq!(
        toggle_task(body, 0).unwrap(),
        "- [x] dig\n* [x] mine\n+ [X] smelt\n1. [ ] first\n2) [x] second\n"
    );
    assert_eq!(
        toggle_task(body, 1).unwrap(),
        "- [ ] dig\n* [ ] mine\n+ [X] smelt\n1. [ ] first\n2) [x] second\n",
        "[x] goes back to [ ]"
    );
    assert_eq!(
        toggle_task(body, 2).unwrap(),
        "- [ ] dig\n* [x] mine\n+ [ ] smelt\n1. [ ] first\n2) [x] second\n",
        "[X] goes back to [ ]"
    );
    assert_eq!(
        toggle_task(body, 3).unwrap(),
        "- [ ] dig\n* [x] mine\n+ [X] smelt\n1. [x] first\n2) [x] second\n"
    );
    assert_eq!(
        toggle_task(body, 4).unwrap(),
        "- [ ] dig\n* [x] mine\n+ [X] smelt\n1. [ ] first\n2) [ ] second\n"
    );
    assert_eq!(toggle_task(body, 5), None);
    assert_eq!(toggle_task("", 0), None);
    assert_eq!(count_tasks(body), 5);
}

#[test]
fn toggle_task_reaches_nested_items_and_skips_code_and_plain_brackets() {
    let nested = "- [ ] parent\n  - [ ] child\n    1. [x] grandchild\n";
    assert_eq!(
        toggle_task(nested, 1).unwrap(),
        "- [ ] parent\n  - [x] child\n    1. [x] grandchild\n"
    );
    assert_eq!(
        toggle_task(nested, 2).unwrap(),
        "- [ ] parent\n  - [ ] child\n    1. [ ] grandchild\n"
    );

    let fenced = "```\n- [ ] not a task\n```\n\n~~~md\n- [x] nor this\n~~~\n\n[ ] plain text\n\n- [ ] real\n";
    assert_eq!(count_tasks(fenced), 1);
    assert_eq!(
        toggle_task(fenced, 0).unwrap(),
        "```\n- [ ] not a task\n```\n\n~~~md\n- [x] nor this\n~~~\n\n[ ] plain text\n\n- [x] real\n"
    );
    assert_eq!(toggle_task(fenced, 1), None);
}

#[test]
fn interactive_check_boxes_are_numbered_like_toggle_task_counts_them() {
    let body = "```\n- [ ] code\n```\n- [x] a\n  - [ ] b\n";
    let mut resolve = |_: &str| Ok(None);
    let live = render_with(
        body,
        &mut resolve,
        RenderOptions {
            interactive_tasks: true,
        },
    )
    .unwrap();
    assert!(
        live.html
            .contains(r#"<input type="checkbox" class="task" data-task="0" checked>"#),
        "{}",
        live.html
    );
    assert!(
        live.html
            .contains(r#"<input type="checkbox" class="task" data-task="1">"#),
        "{}",
        live.html
    );
    assert!(!live.html.contains("data-task=\"2\""), "{}", live.html);
    assert!(!live.html.contains("disabled"), "{}", live.html);

    let inert = render(body, &mut resolve).unwrap();
    assert!(inert.html.contains("disabled"), "{}", inert.html);
    assert!(!inert.html.contains("data-task"), "{}", inert.html);
}
