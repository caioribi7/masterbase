<script>
  // Construtor visual: "mostrar X de TABELA onde ... ordenado por ...". O servidor monta o SELECT.
  import { api } from './api.js';
  import { n } from './lib.js';
  import { canEdit } from './session.svelte.js';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { onAuthLost } = $props();

  const ops = ['é', 'não é', 'contém', 'maior que', 'menor que', 'vazio', 'não vazio'];
  const blank = (table = '') => ({ table, columns: [], filters: [], sort: null, desc: false, limit: 100 });

  let schema = $state([]);
  let saved = $state([]);
  let spec = $state(blank());
  let activeId = $state(null);
  let result = $state(null);
  let error = $state('');
  let busy = $state(false);
  let saving = $state(false);
  let saveName = $state('');
  let showSql = $state(false);
  let showChart = $state(false);
  let groupBy = $state(null); // coluna do gráfico
  let measure = $state(''); // '' = quantidade, ou nome de coluna numérica para somar

  // Título de cada ficha no celular: a primeira coluna de texto (nome), senão a primeira.
  const titleCol = $derived(result?.columns.find((c) => c.label === 'texto') ?? result?.columns[0]);
  const cols = $derived(schema.find((t) => t.name === spec.table)?.columns ?? []);
  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));

  async function init() {
    try {
      [schema, saved] = await Promise.all([api('/schema'), api('/queries')]);
      if (!spec.table && schema.length) spec = blank(schema[0].name);
    } catch (e) {
      fail(e);
    }
  }
  init();

  function pickTable(name) {
    spec = blank(name);
    activeId = null;
    result = null;
  }

  function toggleCol(name) {
    // Lista vazia = todas. Ao desmarcar a primeira, a lista vira "todas menos essa".
    const all = cols.map((c) => c.name);
    const now = spec.columns.length ? spec.columns : all;
    const next = now.includes(name) ? now.filter((c) => c !== name) : all.filter((c) => now.includes(c) || c === name);
    spec.columns = next.length === all.length || next.length === 0 ? [] : next;
  }

  const body = () => ({
    ...spec,
    filters: spec.filters.map((f) => ({ col: f.col, op: f.op, val: f.op.endsWith('vazio') ? null : f.val })),
    limit: Number(spec.limit) || 100,
  });

  async function run(e) {
    e?.preventDefault();
    busy = true;
    error = '';
    try {
      result = await api('/query', { method: 'POST', body: body() });
      // No celular o resultado fica abaixo do formulário: rola até ele.
      if (matchMedia('(max-width: 700px)').matches) setTimeout(() => document.querySelector('.tools')?.scrollIntoView({ behavior: 'smooth' }), 50);
    } catch (err) {
      fail(err);
    }
    busy = false;
  }

  function open(q) {
    spec = { ...blank(), ...structuredClone($state.snapshot(q.spec)) };
    spec.filters = spec.filters.map((f) => ({ ...f, val: f.val ?? '' }));
    activeId = q.id;
    run();
  }

  async function save(e) {
    e.preventDefault();
    try {
      const r = await api('/queries', { method: 'POST', body: { name: saveName, spec: body() } });
      saved = await api('/queries');
      activeId = r.id;
      saving = false;
    } catch (err) {
      saving = false;
      fail(err);
    }
  }

  async function remove(q) {
    try {
      await api(`/queries/${q.id}`, { method: 'DELETE' });
      saved = saved.filter((x) => x.id !== q.id);
      if (activeId === q.id) activeId = null;
    } catch (err) {
      fail(err);
    }
  }

  // Gráfico rápido: agrupa as linhas do resultado por uma coluna e conta (ou soma uma coluna numérica).
  const numeric = $derived(result?.columns.filter((c) => !c.pk && (c.label === 'número' || c.label === 'decimal')) ?? []);
  // Coluna padrão: a que repete mais (menos valores distintos, mas mais de um).
  const bestGroup = $derived.by(() => {
    if (!result?.rows.length) return null;
    const scored = result.columns
      .filter((c) => !c.pk)
      .map((c) => [c.name, new Set(result.rows.map((r) => show(r[c.name]))).size])
      .filter(([, k]) => k > 1 && k < result.rows.length);
    scored.sort((a, b) => a[1] - b[1]);
    return scored[0]?.[0] ?? result.columns[0]?.name;
  });
  const bars = $derived.by(() => {
    const col = groupBy ?? bestGroup;
    if (!result || !col) return [];
    const sums = new Map();
    for (const r of result.rows) {
      const k = show(r[col]);
      const v = measure ? Number(r[measure]) || 0 : 1;
      sums.set(k, (sums.get(k) ?? 0) + v);
    }
    const all = [...sums].sort((a, b) => b[1] - a[1]);
    const top = all.slice(0, 12);
    if (all.length > 12) top.push(['outros', all.slice(12).reduce((t, [, v]) => t + v, 0)]);
    const max = Math.max(...top.map(([, v]) => v), 1);
    return top.map(([label, value]) => ({ label, value, pct: (value / max) * 100 }));
  });

  function show(v) {
    if (v == null) return '—';
    if (typeof v === 'boolean') return v ? 'Sim' : 'Não';
    if (typeof v === 'object') return JSON.stringify(v);
    return String(v);
  }
</script>

<div class="side-list">
  <span class="label">Salvas</span>
  {#each saved as q (q.id)}
    <div class="saved" class:active={q.id === activeId}>
      <button class="list-item" class:active={q.id === activeId} onclick={() => open(q)}><span>{q.name}</span></button>
      {#if canEdit()}<button class="icon-btn del" aria-label="Apagar {q.name}" title="Apagar" onclick={() => remove(q)}><Icon name="close" size={13} /></button>{/if}
    </div>
  {:else}
    <p class="hint pad">Nenhuma ainda. Monte uma consulta e salve para usar de novo.</p>
  {/each}
  <button class="list-item add" onclick={() => pickTable(spec.table || schema[0]?.name || '')}><Icon name="plus" size={14} /> Nova consulta</button>
</div>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Consultas</h1>
      <p>Escolha a tabela, o que mostrar e as condições. Sem escrever SQL.</p>
    </div>
  </div>

  <div class="panel-edge"><form class="panel builder" onsubmit={run}>
    <div class="line">
      <span class="grp">
        <span class="word">Mostrar</span>
        <select class="field small" value={spec.table} onchange={(e) => pickTable(e.currentTarget.value)} aria-label="Tabela">
          {#each schema as t}<option value={t.name}>{t.name}</option>{/each}
        </select>
      </span>
      <span class="word">com as colunas</span>
      <div class="chips">
        {#each cols as c}
          {@const on = !spec.columns.length || spec.columns.includes(c.name)}
          <button type="button" class="pill" class:on aria-pressed={on} onclick={() => toggleCol(c.name)}>{c.name}</button>
        {/each}
      </div>
    </div>

    {#each spec.filters as f, i}
      <div class="line filter">
        <span class="word">{i ? 'e' : 'onde'}</span>
        <select class="field small" bind:value={f.col} aria-label="Coluna">{#each cols as c}<option value={c.name}>{c.name}</option>{/each}</select>
        <select class="field small op" bind:value={f.op} aria-label="Condição">{#each ops as o}<option value={o}>{o}</option>{/each}</select>
        {#if !f.op.endsWith('vazio')}<input class="field small val" bind:value={f.val} placeholder="valor" aria-label="Valor" required />{/if}
        <button type="button" class="icon-btn x" aria-label="Tirar condição" onclick={() => spec.filters.splice(i, 1)}><Icon name="close" size={14} /></button>
      </div>
    {/each}

    <div class="line">
      <button type="button" class="glass btn small add-cond" onclick={() => spec.filters.push({ col: cols[0]?.name ?? '', op: 'é', val: '' })}><Icon name="plus" size={14} /> Condição</button>
      <span class="grp sort">
        <span class="word gap">ordenar por</span>
        <select class="field small" bind:value={spec.sort} aria-label="Ordenar por">
          <option value={null}>sem ordem</option>
          {#each cols as c}<option value={c.name}>{c.name}</option>{/each}
        </select>
        {#if spec.sort}
          <select class="field small op" bind:value={spec.desc} aria-label="Direção"><option value={false}>crescente</option><option value={true}>decrescente</option></select>
        {/if}
      </span>
      <span class="grp">
        <span class="word gap">até</span>
        <input class="field small num" type="number" min="1" max="500" bind:value={spec.limit} aria-label="Máximo de linhas" />
        <span class="word">linhas</span>
      </span>
      <span class="grow"></span>
      <span class="acts">
        {#if canEdit()}<button type="button" class="glass btn small" onclick={() => { saveName = saved.find((q) => q.id === activeId)?.name ?? ''; saving = true; }}><Icon name="save" size={14} /> Salvar</button>{/if}
        <button class="btn-primary small" disabled={busy || !spec.table}><Icon name="play" size={12} stroke="#12133A" weight={2.6} /> Rodar</button>
      </span>
    </div>
  </form></div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if result}
    <div class="tools">
      <span class="count">{n(result.rows.length)} {result.rows.length === 1 ? 'linha' : 'linhas'}{result.rows.length >= (Number(spec.limit) || 100) ? ' (limite atingido)' : ''}</span>
      <span class="grow"></span>
      {#if result.rows.length > 1}<button class="glass btn small" aria-expanded={showChart} onclick={() => (showChart = !showChart)}>{showChart ? 'Esconder gráfico' : 'Gráfico'}</button>{/if}
      <button class="glass btn small" aria-expanded={showSql} onclick={() => (showSql = !showSql)}>{showSql ? 'Esconder SQL' : 'Ver SQL'}</button>
    </div>
    {#if showChart && result.rows.length > 1}
      <div class="panel-edge"><div class="panel chart">
        <div class="chart-opts">
          <label><span class="word">Agrupar por</span>
            <select class="field small" value={groupBy ?? bestGroup} onchange={(e) => (groupBy = e.currentTarget.value)}>
              {#each result.columns as c}<option value={c.name}>{c.name}</option>{/each}
            </select>
          </label>
          <label><span class="word">e mostrar</span>
            <select class="field small" bind:value={measure}>
              <option value="">quantidade de linhas</option>
              {#each numeric as c}<option value={c.name}>soma de {c.name}</option>{/each}
            </select>
          </label>
        </div>
        <ul class="bars" aria-label="Gráfico">
          {#each bars as b (b.label)}
            <li>
              <span class="bar-label" title={b.label}>{b.label}</span>
              <span class="bar-track"><span class="bar-fill" style:width="{b.pct}%"></span></span>
              <span class="bar-value mono">{n(Math.round(b.value * 100) / 100)}</span>
            </li>
          {/each}
        </ul>
        <p class="hint">Conta as {n(result.rows.length)} linhas deste resultado. Para incluir mais, aumente o "até" e rode de novo.</p>
      </div></div>
    {/if}
    {#if showSql}<pre class="code">{result.sql}</pre>{/if}
    <div class="cards">
      {#each result.rows as row, i (i)}
        <div class="card-edge"><div class="card">
          <strong class="card-title">{show(row[titleCol.name])}</strong>
          {#each result.columns as c}
            {#if c !== titleCol}<div class="card-kv"><span>{c.name}</span><strong class:mono={c.label !== 'texto' && c.label !== 'e-mail' && c.label !== 'opção'}>{show(row[c.name])}</strong></div>{/if}
          {/each}
        </div></div>
      {:else}
        <p class="none">Nada bate com essas condições.</p>
      {/each}
    </div>
    <div class="panel-edge grid-view"><div class="panel scroll-x">
      <table class="simple">
        <thead><tr>{#each result.columns as c}<th>{c.name}</th>{/each}</tr></thead>
        <tbody>
          {#each result.rows as row}
            <tr>{#each result.columns as c}<td class:mono={c.label !== 'texto' && c.label !== 'e-mail' && c.label !== 'opção'}>{show(row[c.name])}</td>{/each}</tr>
          {:else}
            <tr><td class="none" colspan={result.columns.length}>Nada bate com essas condições.</td></tr>
          {/each}
        </tbody>
      </table>
    </div></div>
  {/if}
</section>

{#if saving}
  <Modal title="Salvar consulta" onClose={() => (saving = false)} width={440}>
    <form class="form" onsubmit={save}>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field" bind:value={saveName} placeholder="ex.: clientes pro de Curitiba" aria-label="Nome da consulta" required autofocus />
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (saving = false)}>Cancelar</button>
        <button class="btn-primary">Salvar</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .saved { position: relative; display: flex; }
  .saved .list-item { flex: 1; min-width: 0; padding-right: 30px; }
  .del { position: absolute; right: 3px; top: 3px; opacity: 0; }
  .saved:hover .del, .del:focus-visible { opacity: 1; }
  .pad { padding: 0 8px 8px; }

  .builder { padding: 18px 20px; display: flex; flex-direction: column; gap: 12px; }
  .line { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; min-height: 36px; }
  .word { font-size: 14px; color: var(--lavender); }
  .word.gap { margin-left: 10px; }
  .line .field { width: 150px; }
  .line .op { width: 124px; }
  .line .num { width: 84px; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .pill { height: 30px; padding: 0 12px; border-radius: 8px; border: 1px solid #2A2E6A; background: #0C0D28; font-size: 13px; color: var(--lavender-3); transition: border-color .18s ease-out, color .18s ease-out; }
  .pill.on { background: linear-gradient(180deg, #2A2E70 0%, #1E2158 100%); border-color: rgba(154,160,224,.5); box-shadow: inset 0 1px 0 rgba(255,255,255,.14); color: var(--text); }
  .tools { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .count { color: var(--lavender-2); }
  button:disabled { opacity: .6; }
  .grp, .acts { display: inline-flex; align-items: center; gap: 10px; }

  .chart { padding: 18px 20px; display: flex; flex-direction: column; gap: 16px; }
  .chart-opts { display: flex; flex-wrap: wrap; gap: 10px 20px; }
  .chart-opts label { display: inline-flex; align-items: center; gap: 10px; }
  .chart-opts .field { width: 190px; }
  .bars { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .bars li { display: grid; grid-template-columns: minmax(80px, 180px) 1fr auto; align-items: center; gap: 12px; font-size: 13px; }
  .bar-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--soft); }
  .bar-track { height: 14px; border-radius: 7px; background: rgba(154,160,224,.08); overflow: hidden; }
  .bar-fill { display: block; height: 100%; min-width: 3px; border-radius: 7px; background: linear-gradient(90deg, #5A60C8, #9AA0E0); box-shadow: 0 0 12px rgba(154,160,224,.3); transition: width .3s ease-out; }
  .bar-value { min-width: 40px; text-align: right; font-size: 12px; color: var(--lavender-2); }

  .cards { display: none; flex-direction: column; gap: 10px; }
  .card-edge { padding: 1px; border-radius: 16px; background: var(--edge); }
  .card { border-radius: 15px; padding: 14px 16px; display: flex; flex-direction: column; gap: 6px; background: linear-gradient(180deg, #17194A 0%, #11123A 100%); box-shadow: var(--inner); }
  .card-title { font-weight: 600; font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-kv { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
  .card-kv span { color: var(--lavender-2); flex-shrink: 0; }
  .card-kv strong { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cards .none { padding: 32px 0; text-align: center; color: var(--lavender-2); }

  /* Celular: cada parte da frase numa linha, campos na largura toda, resultado em fichas. */
  @media (max-width: 700px) {
    .builder { padding: 16px; gap: 14px; }
    .line { gap: 8px; }
    .word.gap { margin-left: 0; }
    .grp { width: 100%; }
    .grp .field, .grp .op { flex: 1; min-width: 0; width: auto; }
    .grp .num { flex: 0 0 84px; }
    .filter { display: grid; grid-template-columns: auto 1fr 1fr auto; padding: 10px; border-radius: 12px; background: rgba(12,13,40,.6); border: 1px solid rgba(154,160,224,.14); }
    .filter .field { width: auto; min-width: 0; }
    .filter .val { grid-column: 2 / 4; }
    .filter .x { grid-column: 4; grid-row: 1; }
    .add-cond { width: 100%; justify-content: center; }
    .grow { display: none; }
    .acts { width: 100%; }
    .acts > * { flex: 1; justify-content: center; height: 44px; }
    .del { opacity: 1; }
    .cards { display: flex; }
    .grid-view { display: none; }
    .chart { padding: 16px; }
    .chart-opts label { width: 100%; }
    .chart-opts .field { flex: 1; width: auto; min-width: 0; }
    .bars li { grid-template-columns: 96px 1fr auto; gap: 8px; }
  }
</style>
