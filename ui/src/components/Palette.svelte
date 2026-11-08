<script lang="ts">
  // The command palette (review 4.7): one input over the entries of this guide, the commands
  // with their shortcuts, and the guides, plus a row that creates the typed entry. Mod+K or the
  // Search button opens it; Up and Down move, Enter runs, Escape closes.
  import { untrack } from "svelte";

  import { api, errorMessage, type EntrySummary, type Guide } from "../lib/api";
  import { moveHighlight } from "../lib/combobox";
  import { IS_MAC, type Action } from "../lib/keys";
  import { buildGroups, flatten, type Command, type Row } from "../lib/palette";
  import { navigate, type Route } from "../lib/router";
  import { session, touch } from "../lib/session.svelte";
  import { flattenTags, type TagOption } from "../lib/tags";

  interface Props {
    open: boolean;
    route: Route;
    onclose: () => void;
    onaction: (action: Action) => void;
  }
  let { open, route, onclose, onaction }: Props = $props();

  let text = $state("");
  let entries = $state<EntrySummary[]>([]);
  let guides = $state<Guide[]>([]);
  let tags = $state<TagOption[]>([]);
  let highlight = $state(0);
  let error = $state("");
  let input = $state<HTMLInputElement | null>(null);
  let opener: HTMLElement | null = null;
  let request = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const uid = $props.id();

  const guide = $derived(session.current.guide);
  const inGuide = $derived(
    guide !== null &&
      (route.view === "guide" || route.view === "entry" || route.view === "tags" || route.view === "trash"),
  );
  const commands = $derived(commandsFor(route, inGuide, session.notices.length > 0));
  const groups = $derived(
    buildGroups(text, {
      entries,
      tagPathOf: (ids) => ids.map((id) => tags.find((t) => t.id === id)?.path ?? "").filter(Boolean).join(", "),
      commands,
      guides,
      inGuide,
      mac: IS_MAC,
    }),
  );
  const rows = $derived(flatten(groups));

  // Opening resets the text and loads what the lists need; focus goes to the input and back.
  $effect(() => {
    if (open) {
      opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      text = "";
      highlight = 0;
      untrack(() => void loadStatic());
      setTimeout(() => input?.focus(), 0);
    } else {
      opener?.focus();
      opener = null;
    }
  });

  // The entry list follows the text with a short debounce; guides and commands filter locally.
  $effect(() => {
    if (!open) return;
    const query = text;
    const g = inGuide ? guide?.id : undefined;
    clearTimeout(timer);
    timer = setTimeout(() => void loadEntries(g, query), 60);
  });

  $effect(() => {
    void rows;
    highlight = 0;
  });

  function commandsFor(r: Route, guideLevel: boolean, hasNotice: boolean): Command[] {
    const list: Command[] = [];
    if (guideLevel) list.push({ action: "new", label: "New entry" });
    if (r.view === "guides") list.push({ action: "new-guide", label: "New guide" });
    if (r.view === "entry") {
      list.push(
        { action: "toggle-mode", label: "Toggle Read and Edit" },
        { action: "focus-relation", label: "Add a relation" },
        { action: "add-tag", label: "Add a tag" },
        { action: "rename", label: "Rename entry" },
        { action: "toggle-panel", label: "Show or hide the right panel" },
      );
    }
    if (guideLevel) {
      list.push(
        { action: "focus-search", label: "Search this guide" },
        { action: "tag-editor", label: "Tag editor" },
        { action: "trash", label: "Trash" },
      );
    }
    list.push(
      { action: "toggle-sidebar", label: "Show or hide the sidebar" },
      { action: "settings", label: "Settings" },
      { action: "back", label: "Back" },
      { action: "forward", label: "Forward" },
    );
    if (hasNotice) list.push({ action: "dismiss-notice", label: "Dismiss notice" });
    list.push(
      { action: "density-compact", label: "Density: Compact" },
      { action: "density-roomy", label: "Density: Roomy" },
    );
    return list;
  }

  async function loadStatic() {
    try {
      guides = await api.listGuides();
      tags = inGuide && guide ? flattenTags(await api.tagTree(guide.id)) : [];
      error = "";
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function loadEntries(guideId: string | undefined, query: string) {
    const current = ++request;
    if (!guideId || query.trim().startsWith(">")) {
      entries = [];
      return;
    }
    try {
      const result = await api.listEntries(guideId, { text: query.trim() || null });
      if (current !== request) return;
      entries = result;
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function run(row: Row) {
    onclose();
    if (row.kind === "entry") navigate({ view: "entry", entryId: row.entry.id });
    else if (row.kind === "guide") navigate({ view: "guide", guideId: row.guide.id });
    else if (row.kind === "command") onaction(row.command.action);
    else if (row.kind === "create" && guide) {
      try {
        const entry = await api.createEntry(guide.id, row.title);
        touch();
        navigate({ view: "entry", entryId: entry.id });
      } catch (e) {
        error = errorMessage(e);
      }
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && rows.length > 0) {
      event.preventDefault();
      highlight = moveHighlight(highlight, event.key === "ArrowDown" ? 1 : -1, rows.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const row = rows[highlight];
      if (row) void run(row);
    }
  }

  function rowText(row: Row): string {
    if (row.kind === "entry") return row.entry.title;
    if (row.kind === "guide") return row.guide.name;
    if (row.kind === "command") return row.command.label;
    return `Create entry "${row.title}"`;
  }

  function rowKey(row: Row): string {
    if (row.kind === "entry") return `e-${row.entry.id}`;
    if (row.kind === "guide") return `g-${row.guide.id}`;
    if (row.kind === "command") return `c-${row.command.action}`;
    return "create";
  }
</script>

{#if open}
  <div class="backdrop" role="presentation" onclick={onclose}></div>
  <div class="palette" role="dialog" aria-label="Search and commands">
    <input
      bind:this={input}
      bind:value={text}
      type="text"
      placeholder={inGuide ? "Search entries, commands, and guides" : "Search guides and commands"}
      aria-label="Search and commands"
      role="combobox"
      aria-expanded={rows.length > 0}
      aria-controls={`${uid}-rows`}
      aria-activedescendant={rows.length > 0 ? `${uid}-row-${highlight}` : undefined}
      autocomplete="off"
      {onkeydown}
    />
    {#if error}<p class="error note">{error}</p>{/if}
    <ul id={`${uid}-rows`} role="listbox" aria-label="Results">
      {#if rows.length === 0}
        <li class="note muted" role="presentation">No entries or commands match.</li>
      {/if}
      {#each groups as group (group.heading)}
        <li class="heading muted" role="presentation">{group.heading}</li>
        {#each group.rows as row (rowKey(row))}
          {@const index = rows.indexOf(row)}
          <li
            id={`${uid}-row-${index}`}
            role="option"
            aria-selected={index === highlight}
            class:on={index === highlight}
          >
            <button
              type="button"
              tabindex="-1"
              onmousedown={(e) => {
                e.preventDefault();
                void run(row);
              }}
            >
              <span class="text">
                {rowText(row)}
                {#if row.kind === "entry" && row.entry.is_stub}<span class="chip">stub</span>{/if}
              </span>
              {#if row.kind === "entry" && row.detail}<span class="detail muted">{row.detail}</span>{/if}
              {#if row.kind === "command" && row.keys}<kbd>{row.keys}</kbd>{/if}
            </button>
          </li>
        {/each}
      {/each}
    </ul>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    background: rgb(0 0 0 / 0.2);
  }
  .palette {
    position: fixed;
    top: 15vh;
    left: 50%;
    z-index: 31;
    width: min(560px, calc(100vw - 32px));
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.2);
  }
  .palette input {
    width: 100%;
    box-sizing: border-box;
  }
  ul {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    max-height: min(420px, 60vh);
    overflow: auto;
  }
  .heading {
    padding: 6px 10px 2px;
    font-size: 0.75em;
    font-weight: 600;
    font-variant-caps: all-small-caps;
    letter-spacing: 0.04em;
  }
  .note {
    padding: 8px 10px;
  }
  li[role="option"] button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: var(--row-h);
    text-align: left;
    padding: 4px 10px;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: inherit;
  }
  li.on button,
  li[role="option"] button:hover {
    background: var(--chip-bg);
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .detail {
    font-size: var(--small);
  }
  kbd {
    font: inherit;
    font-size: var(--small);
    padding: 0 6px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--muted);
  }
</style>
