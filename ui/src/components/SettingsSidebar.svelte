<script lang="ts">
  // The sidebar on Settings (review 3.7): the sections, with the one in view marked. A click
  // scrolls the page to the section.
  import SidebarFrame from "./SidebarFrame.svelte";

  const SECTIONS = [
    { id: "appearance", label: "Appearance" },
    { id: "data", label: "Data" },
    { id: "backups", label: "Backups" },
    { id: "updates", label: "Updates" },
  ];

  let current = $state("appearance");

  // The section whose top is nearest above the page's top edge is the current one.
  $effect(() => {
    const main = document.querySelector("main");
    if (!main) return;
    const update = () => {
      const top = main.getBoundingClientRect().top + 8;
      let found = SECTIONS[0].id;
      for (const section of SECTIONS) {
        const el = document.getElementById(`settings-${section.id}`);
        if (el && el.getBoundingClientRect().top <= top) found = section.id;
      }
      current = found;
    };
    update();
    main.addEventListener("scroll", update);
    return () => main.removeEventListener("scroll", update);
  });

  function go(id: string) {
    document.getElementById(`settings-${id}`)?.scrollIntoView({ block: "start" });
    current = id;
  }
</script>

<SidebarFrame label="Settings sections">
  <nav class="region" aria-label="Settings sections">
    <h2 class="region-head">Settings</h2>
    <ul>
      {#each SECTIONS as section (section.id)}
        <li>
          <div class="tree-line" class:selected={current === section.id}>
            <span class="tree-chev"></span>
            <button
              class="tree-name"
              aria-current={current === section.id ? "true" : undefined}
              onclick={() => go(section.id)}
            >
              {section.label}
            </button>
          </div>
        </li>
      {/each}
    </ul>
  </nav>
</SidebarFrame>

<style>
  .region {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
