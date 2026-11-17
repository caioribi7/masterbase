<script>
  import { icons } from './lib.js';
  let { name, size = 16, stroke = 'currentColor', weight = 2 } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" {stroke} stroke-width={weight} stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={icons[name]} /></svg>

<style>
  svg { flex-shrink: 0; }
</style>
