<script lang="ts">
  // The sidebar on Settings (review 3.7): the sections, with the one in view marked. A click
  // scrolls the page to the section. The list is lib/settingsSections.ts, shared with the page.
  // The list is one Tab stop with the arrow keys inside; its title shares the fold button's row.
  import { listKeydown, rememberRow } from "../lib/rovingList";
  import { SECTIONS, sectionElementId, type SectionId } from "../lib/settingsSections";
  import { rovingStop } from "../lib/sidebar";
  import SidebarFrame from "./SidebarFrame.svelte";
  import TreeRow from "./TreeRow.svelte";

  let current = $state<SectionId>(SECTIONS[0].id);
  let listEl = $state<HTMLElement | null>(null);
  let focused = $state<string | null>(null);
  const stop = $derived(
    rovingStop(
      SECTIONS.map((s) => s.id),
      focused,
      current,
    ),
  );
  const onListKey = listKeydown(() => listEl);

  // The section whose top is nearest above the page's top edge is the current one.
  $effect(() => {
    const main = document.querySelector("main");
    if (!main) return;
    const update = () => {
      const top = main.getBoundingClientRect().top + 8;
      let found: SectionId = SECTIONS[0].id;
      for (const section of SECTIONS) {
        const el = document.getElementById(sectionElementId(section.id));
        if (el && el.getBoundingClientRect().top <= top) found = section.id;
      }
      current = found;
    };
    update();
    main.addEventListener("scroll", update);
    return () => main.removeEventListener("scroll", update);
  });

  function go(id: SectionId) {
    document.getElementById(sectionElementId(id))?.scrollIntoView({ block: "start" });
    current = id;
  }
</script>

<SidebarFrame label="Settings sections">
  {#snippet head()}<h2 class="region-head title">Settings</h2>{/snippet}
  <nav class="region" aria-label="Settings sections">
    <ul class="side-list" bind:this={listEl} onfocusin={rememberRow((key) => (focused = key))}>
      {#each SECTIONS as section (section.id)}
        <li>
          <TreeRow
            id={section.id}
            name={section.label}
            selected={current === section.id}
            tabindex={stop === section.id ? 0 : -1}
            onpick={() => go(section.id)}
            onkeydown={onListKey}
          />
        </li>
      {/each}
    </ul>
  </nav>
</SidebarFrame>

<style>
  /* The title in the head row, beside the fold button; the frame gives the row its padding. */
  .title {
    position: static;
    flex: 1;
    padding: 0;
    background: none;
  }
</style>
