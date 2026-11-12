<script lang="ts">
  // An empty list or page (component rule, UI review 2026-11-10): a muted icon, one sentence, one
  // muted line, and the action when there is one, centered in its region.
  import type { LucideIcon } from "@lucide/svelte";
  import type { Snippet } from "svelte";

  interface Props {
    icon: LucideIcon;
    title: string;
    line?: string;
    /** The action, such as a button; omitted when there is nothing to do. */
    children?: Snippet;
  }
  let { icon: Icon, title, line, children }: Props = $props();
</script>

<div class="empty-state">
  <span class="icon"><Icon size={32} aria-hidden="true" /></span>
  <p class="title">{title}</p>
  {#if line}<p class="line">{line}</p>{/if}
  {#if children}<div class="action">{@render children()}</div>{/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    max-width: 28rem;
    margin: 0 auto;
    padding-top: var(--space-7);
    text-align: center;
  }
  .icon {
    display: inline-flex;
    color: var(--muted);
  }
  .title {
    margin: 0;
    font-weight: 600;
  }
  .line {
    margin: 0;
    color: var(--muted);
  }
  .action {
    margin-top: var(--space-2);
  }
</style>
