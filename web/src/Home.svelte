<script>
  import { api } from './api.js';
  import { ago, go, href, intent, n, phrase } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';
  import { session, isAdmin } from './session.svelte.js';

  let { onAuthLost } = $props();
  let o = $state(null);
  let error = $state('');

  api('/overview')
    .then((r) => (o = r))
    .catch((e) => (e.status === 401 ? onAuthLost() : (error = e.message)));

  const max = $derived(Math.max(1, ...(o?.tables.map((t) => t.rows) ?? [])));

  function createTable() {
    intent.createTable = true;
    go('estrutura');
  }
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">{session.user.project}</h1>
      <p>Como o banco está agora e o que mudou por último.</p>
    </div>
    <div class="actions">
      <a class="glass btn" href={href('tabelas')}><Icon name="grid" size={15} stroke="#C9CCEB" /> Abrir tabelas</a>
      {#if isAdmin()}<button class="btn-primary" onclick={createTable}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Criar tabela</button>{/if}
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if o}
    <div class="stats">
      {#each [['Tabelas', n(o.tables.length)], ['Linhas', n(o.rows)], ['Tamanho', o.size], ['Postgres', o.version.split(' ')[0]]] as [k, v]}
        <div class="panel-edge"><div class="panel stat"><span class="label">{k}</span><strong class="brand">{v}</strong></div></div>
      {/each}
    </div>

    <div class="cols">
      <div class="panel-edge"><div class="panel card">
        <header><span class="label">Tabelas</span></header>
        {#if o.tables.length}
          <ul>
            {#each o.tables as t}
              <li>
                <a class="row" href={href('tabelas', t.name)}>
                  <span class="name">{t.name}</span>
                  <span class="bar"><i style:width="{Math.max(2, (t.rows / max) * 100)}%"></i></span>
                  <span class="mono num">{n(t.rows)}</span>
                </a>
              </li>
            {/each}
          </ul>
        {:else}
          <div class="empty-state"><Glyph size={48} /><p>Nenhuma tabela ainda.</p></div>
        {/if}
      </div></div>

      <div class="panel-edge"><div class="panel card">
        <header><span class="label">Últimas alterações</span><a class="more" href={href('atividade')}>Ver tudo</a></header>
        {#if o.recent.length}
          <ul>
            {#each o.recent as a}
              <li class="act">
                <span class="who">{a.user ?? 'Alguém'}</span>
                {#each phrase(a) as p}{#if p.b}<strong>{p.b}</strong>{:else}{p.t}{/if}{/each}
                <time class="mono" datetime={a.at}>{ago(a.at)}</time>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="dim">Nada mudou ainda.</p>
        {/if}
      </div></div>
    </div>
  {/if}
</section>

<style>
  .stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 16px; }
  .stat { padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; }
  .stat .label { color: var(--lavender-2); }
  .stat strong { font-size: 28px; line-height: 32px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .cols { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 16px; }
  .card { padding: 18px 20px; display: flex; flex-direction: column; gap: 12px; }
  .card header { display: flex; justify-content: space-between; align-items: center; }
  .card .label { color: var(--lavender-2); }
  .more { font-size: 13px; color: var(--soft); }
  ul { list-style: none; padding: 0; display: flex; flex-direction: column; }

  .row { height: 44px; padding: 0 10px; margin: 0 -10px; border-radius: 10px; display: grid; grid-template-columns: minmax(80px, 160px) 1fr 64px; align-items: center; gap: 14px; color: var(--text); text-decoration: none; font-size: 14px; }
  .row:hover { background: rgba(154,160,224,.06); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar { height: 6px; border-radius: 3px; background: rgba(154,160,224,.1); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 3px; background: linear-gradient(90deg, #5E64A8, #9AA0E0); }
  .num { font-size: 12px; color: var(--lavender); text-align: right; }

  .act { min-height: 44px; padding: 10px 0; border-bottom: 1px solid #1A1C46; font-size: 14px; color: var(--soft); line-height: 22px; }
  .act:last-child { border-bottom: 0; }
  .act strong { color: var(--text); font-weight: 600; }
  .who { color: var(--text); font-weight: 600; margin-right: 4px; }
  time { float: right; margin-left: 12px; font-size: 11px; color: var(--lavender-3); }

  @media (max-width: 1100px) {
    .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .cols { grid-template-columns: minmax(0, 1fr); }
  }
</style>
