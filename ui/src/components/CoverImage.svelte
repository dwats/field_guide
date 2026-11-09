<script lang="ts">
  // A cover inside a frame the parent sizes (posters are 2:3): the stored crop decides what
  // shows, or the centre when there is none. The same crop fits any frame size.
  import type { CoverCrop } from "../lib/api";
  import { cropStyle } from "../lib/cover";

  interface Props {
    src: string;
    crop: CoverCrop | null;
    alt?: string;
  }
  let { src, crop, alt = "" }: Props = $props();
</script>

<div class="cover-frame">
  <img {src} {alt} style={cropStyle(crop)} draggable="false" />
</div>

<style>
  .cover-frame {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }
  img {
    position: absolute;
    left: 0;
    top: 0;
    display: block;
    max-width: none;
  }
</style>
