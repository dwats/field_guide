<script lang="ts">
  // The one suggestion list: used under the relation line and the tag input. It renders in the
  // flow of the page, under its input, so it never covers anything. The parent owns the items,
  // the highlighted index, and what picking one does; this component only draws them.
  export interface SuggestionItem {
    key: string;
    text: string;
    detail?: string;
  }

  interface Props {
    /** Id the input names in `aria-controls`; each option is `${listId}-${index}`. */
    listId: string;
    items: SuggestionItem[];
    highlight: number;
    onpick: (index: number) => void;
  }
  let { listId, items, highlight, onpick }: Props = $props();
</script>

{#if items.length > 0}
  <ul class="suggestions" role="listbox" id={listId}>
    {#each items as item, i (item.key)}
      <li role="option" id={`${listId}-${i}`} aria-selected={i === highlight}>
        <button
          type="button"
          class:on={i === highlight}
          onmousedown={(e) => {
            // mousedown, not click, so the input does not blur and close the list first.
            e.preventDefault();
            onpick(i);
          }}
        >
          <span>{item.text}</span>
          {#if item.detail}<span class="muted">{item.detail}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .suggestions {
    margin: 2px 0 0;
    padding: 4px 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    max-height: 260px;
    overflow: auto;
  }
  .suggestions button {
    display: flex;
    gap: 10px;
    width: 100%;
    text-align: left;
    padding: 4px 10px;
    border: none;
    border-radius: 0;
    background: none;
    color: inherit;
  }
  .suggestions button.on,
  .suggestions button:hover {
    background: var(--chip-bg);
  }
</style>
