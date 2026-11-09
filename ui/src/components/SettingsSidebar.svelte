<script lang="ts">
  // The sidebar on Settings (review 3.7): the sections, with the one in view marked. A click
  // scrolls the page to the section. The list is lib/settingsSections.ts, shared with the page.
  import { SECTIONS, sectionElementId, type SectionId } from "../lib/settingsSections";
  import SidebarFrame from "./SidebarFrame.svelte";
  import TreeRow from "./TreeRow.svelte";

  let current = $state<SectionId>(SECTIONS[0].id);

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
  <nav class="region" aria-label="Settings sections">
    <h2 class="region-head">Settings</h2>
    <ul class="side-list">
      {#each SECTIONS as section (section.id)}
        <li>
          <TreeRow name={section.label} selected={current === section.id} onpick={() => go(section.id)} />
        </li>
      {/each}
    </ul>
  </nav>
</SidebarFrame>
