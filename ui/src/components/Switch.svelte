<script lang="ts">
  // A switch in one row: the label at the left, the track at the right, on or off. The track is
  // drawn as `PrefCard` draws its own: 36 by 20, the accent when on and a field-strength gray when
  // off, with a 16-pixel knob in the page color at the side the state names.
  interface Props {
    label: string;
    checked: boolean;
    onchange: (next: boolean) => void;
  }
  let { label, checked, onchange }: Props = $props();
</script>

<button class="switch" role="switch" aria-checked={checked} onclick={() => onchange(!checked)}>
  <span class="label">{label}</span>
  <span class="track" aria-hidden="true"><span class="knob"></span></span>
</button>

<style>
  .switch {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    width: 100%;
    padding: 2px 0;
    border: none;
    background: none;
    text-align: left;
  }
  .label {
    min-width: 0;
  }
  .track {
    flex: none;
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: var(--radius-pill);
    background: var(--field-border);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--bg);
  }
  .switch[aria-checked="true"] .track {
    background: var(--accent);
  }
  .switch[aria-checked="true"] .knob {
    left: 18px;
  }
</style>
