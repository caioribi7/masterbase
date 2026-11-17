<script>
  import { api } from './api.js';
  import { ago, go, href, phrase } from './lib.js';
  import Icon from './Icon.svelte';

  let { arg, onAuthLost } = $props();

  let items = $state([]);
  let more = $state(false);
  let tables = $state([]);
  let error = $state('');
  let busy = $state(false);

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));

  async function load(before) {
    busy = true;
    const q = new URLSearchParams();
    if (arg) q.set('table', arg);
    if (before) q.set('before', before);
    try {
      const r = await api(`/activity?${q}`);
      items = before ? [...items, ...r.items] : r.items;
      more = r.more;
    } catch (e) {
      fail(e);
    }
    busy = false;
  }

  $effect(() => { arg; load(); });
  api('/tables').then((t) => (tables = t.map((x) => x.name))).catch(fail);

  const day = (iso) => new Date(iso).toLocaleDateString('pt-BR', { weekday: 'long', day: 'numeric', month: 'long' });
  const time = (iso) => new Date(iso).toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' });
  const icon = (a) => ({ login: 'logout', signup: 'Acesso', update: 'pencil', insert: 'plus', delete: 'trash', drop_table: 'trash', drop_column: 'trash' })[a] ?? 'Estrutura';
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Atividade</h1>
      <p>Quem mudou o quê e quando. O registro não pode ser editado pela tela.</p>
    </div>
    <div class="actions">
      <select class="field small pick" value={arg ?? ''} onchange={(e) => go('atividade', e.currentTarget.value || null)} aria-label="Filtrar por tabela">
        <option value="">Todas as tabelas</option>
        {#each tables as t}<option value={t}>{t}</option>{/each}
        {#if arg && !tables.includes(arg)}<option value={arg}>{arg}</option>{/if}
      </select>
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="panel-edge"><div class="panel list">
    {#each items as a, i (a.id)}
      {#if i === 0 || day(a.at) !== day(items[i - 1].at)}<div class="day label">{day(a.at)}</div>{/if}
      <div class="item">
        <span class="ico" class:warn={a.action.startsWith('drop') || a.action === 'delete'}><Icon name={icon(a.action)} size={14} /></span>
        <p>
          <span class="who">{a.user ?? 'Masterbase'}</span>
          {#each phrase(a) as p}{#if p.b}<strong>{p.b}</strong>{:else}{p.t}{/if}{/each}
        </p>
        <time class="mono" datetime={a.at} title={ago(a.at)}>{time(a.at)}</time>
      </div>
    {:else}
      {#if !busy}<p class="none">{arg ? `Nada registrado em ${arg}.` : 'Nada registrado ainda.'}</p>{/if}
    {/each}
    {#if more}
      <div class="foot"><button class="glass btn small" disabled={busy} onclick={() => load(items.at(-1).id)}>Carregar mais</button></div>
    {/if}
  </div></div>

  {#if arg}<a class="back" href={href('atividade')}><Icon name="left" size={14} /> Ver tudo</a>{/if}
</section>

<style>
  .pick { width: 200px; }
  .list { padding: 8px 0; }
  .day { padding: 16px 20px 8px; color: var(--lavender-3); }
  .item { min-height: 52px; padding: 8px 20px; display: flex; align-items: center; gap: 14px; font-size: 14px; color: var(--soft); }
  .item:hover { background: rgba(154,160,224,.04); }
  .item p { flex: 1; min-width: 0; line-height: 22px; }
  .item strong { color: var(--text); font-weight: 600; }
  .who { color: var(--text); font-weight: 600; margin-right: 4px; }
  .ico { width: 30px; height: 30px; flex-shrink: 0; border-radius: 9px; display: grid; place-items: center; color: var(--lavender); background: linear-gradient(180deg, #20235A 0%, #15173F 100%); border: 1px solid rgba(154,160,224,.24); box-shadow: inset 0 1px 0 rgba(255,255,255,.1); }
  .ico.warn { color: var(--err); border-color: rgba(255,122,138,.3); }
  time { font-size: 11px; color: var(--lavender-3); }
  .none { padding: 40px 20px; text-align: center; color: var(--lavender-2); }
  .foot { padding: 12px 20px 8px; display: flex; justify-content: center; }
  .back { display: inline-flex; align-items: center; gap: 6px; font-size: 13px; color: var(--soft); }
</style>
