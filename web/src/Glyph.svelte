<script>
  // Glifo âncora-chave, uma cor só. viewBox 34..222 para o arco não cortar.
  let { size = 28, glow = false } = $props();
</script>

<svg width={size} height={size} viewBox="34 34 188 188" fill="none" aria-hidden="true" class:glow>
  <path fill-rule="evenodd" fill="#E6FF00" d="M98 70a30 30 0 1 0 60 0a30 30 0 1 0 -60 0Z M120 86L123 71.48A9 9 0 1 1 133 71.48L136 86Z" />
  <rect x="120" y="98" width="16" height="114" fill="#E6FF00" />
  <rect x="130" y="116" width="30" height="15" rx="4" fill="#E6FF00" />
  <rect x="130" y="141" width="22" height="15" rx="4" fill="#E6FF00" />
  <path d="M66 150A62 62 0 0 0 190 150" stroke="#E6FF00" stroke-width="16" />
  <path d="M52 154L66 129.75L80 154Z" fill="#E6FF00" stroke="#E6FF00" stroke-width="8" stroke-linejoin="round" />
  <path d="M176 154L190 129.75L204 154Z" fill="#E6FF00" stroke="#E6FF00" stroke-width="8" stroke-linejoin="round" />
</svg>

<style>
  svg { flex-shrink: 0; }
  .glow { filter: drop-shadow(0 0 14px rgba(230, 255, 0, 0.35)); }
</style>
