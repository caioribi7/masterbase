<script>
  // Nível "elevado": borda mais acesa e halo maior. Esc ou clique fora fecha.
  import Icon from './Icon.svelte';
  let { title, onClose, children, width = 520 } = $props();

  function key(e) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={key} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={(e) => e.target === e.currentTarget && onClose()}>
  <div class="edge" style:width="min({width}px, 100%)" role="dialog" aria-modal="true" aria-label={title}>
    <div class="box">
      <header>
        <h2 class="brand">{title}</h2>
        <button class="icon-btn" onclick={onClose} aria-label="Fechar"><Icon name="close" /></button>
      </header>
      {@render children()}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 50; padding: 16px; display: grid; place-items: center; background: rgba(6,7,22,.72); backdrop-filter: blur(4px); }
  .edge { padding: 1px; border-radius: 24px; background: linear-gradient(135deg, rgba(154,160,224,.85) 0%, rgba(154,160,224,.14) 30%, rgba(154,160,224,.08) 70%, rgba(154,160,224,.7) 100%); box-shadow: 0 0 48px rgba(154,160,224,.16), 0 30px 60px rgba(4,5,20,.6); max-height: calc(100vh - 32px); display: flex; }
  .box { flex: 1; min-width: 0; border-radius: 23px; padding: 24px 28px 28px; background: linear-gradient(180deg, #1D2052 0%, #13143C 100%); box-shadow: var(--inner); display: flex; flex-direction: column; gap: 18px; overflow-y: auto; }
  header { display: flex; align-items: center; justify-content: space-between; }
  h2 { font-size: 22px; }
</style>
