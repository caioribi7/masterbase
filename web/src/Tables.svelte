<script>
  import { untrack } from 'svelte';
  import { api } from './api.js';
  import { href, n, parseCsv } from './lib.js';
  import { canEdit } from './session.svelte.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { arg, onAuthLost, onOpen } = $props();

  let tables = $state([]);
  let data = $state(null); // { columns, rows, total, page, pages }
  let page = $state(1);
  let sort = $state(null); // { col, desc }
  let filter = $state(null); // { col, op, val }
  let filterOpen = $state(false);
  let draft = $state({ col: '', op: 'é', val: '' });
  let selected = $state(new Set()); // chaves (texto) das linhas marcadas
  let editing = $state(null); // { rowIndex, col, value }
  let adding = $state(null); // { col: value }
  let confirmDelete = $state(false);
  let error = $state('');
  let rowForm = $state(null); // celular: { index (null = linha nova), values, busy, error }
  const phone = () => matchMedia('(max-width: 700px)').matches;

  const current = $derived(arg ?? tables[0]?.name ?? null);
  const pk = $derived(data?.columns.find((c) => c.pk)?.name);
  const allOnPage = $derived(!!data?.rows.length && data.rows.every((r) => selected.has(String(r[pk]))));

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));

  async function loadTables() {
    tables = await api('/tables');
  }

  // Troca de tabela: zera estado da anterior e carrega.
  let loaded = null;
  $effect(() => {
    const name = current;
    if (name && name !== loaded) {
      loaded = name;
      untrack(() => {
        page = 1; sort = null; filter = null; selected = new Set(); adding = null; data = null;
        onOpen(name);
        load();
      });
    }
  });

  async function load() {
    error = '';
    editing = null;
    const q = new URLSearchParams({ page });
    if (filter) { q.set('col', filter.col); q.set('op', filter.op); q.set('val', filter.val); }
    if (sort) { q.set('sort', sort.col); q.set('desc', sort.desc); }
    try {
      data = await api(`/tables/${encodeURIComponent(current)}?${q}`);
      page = data.page;
    } catch (e) {
      fail(e);
    }
  }

  function toggleSort(col) {
    sort = sort?.col !== col ? { col, desc: false } : sort.desc ? null : { col, desc: true };
    page = 1;
    load();
  }

  function openFilter() {
    filterOpen = !filterOpen;
    draft = filter ? { ...filter } : { col: data.columns.find((c) => !c.pk)?.name ?? data.columns[0].name, op: 'é', val: '' };
  }

  function applyFilter(e) {
    e.preventDefault();
    filter = draft.val ? { ...draft } : null;
    filterOpen = false;
    page = 1;
    load();
  }

  function toggleRow(row) {
    const k = String(row[pk]);
    const s = new Set(selected);
    s.has(k) ? s.delete(k) : s.add(k);
    selected = s;
  }

  function toggleAll() {
    const s = new Set(selected);
    for (const r of data.rows) allOnPage ? s.delete(String(r[pk])) : s.add(String(r[pk]));
    selected = s;
  }

  // Coluna que a pessoa não preenche: a chave e, com regra por linha, a do dono (vem do e-mail dela).
  const fixed = (c) => c.pk || c.name === data?.owner;

  function startEdit(i, col) {
    if (fixed(col) || !pk || !canEdit()) return;
    const v = data.rows[i][col.name];
    editing = { rowIndex: i, col: col.name, value: v == null ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v) };
  }

  async function saveEdit() {
    const { rowIndex, col, value } = editing;
    try {
      data.rows[rowIndex] = await api(`/tables/${encodeURIComponent(current)}/rows`, {
        method: 'PATCH',
        body: { pk: data.rows[rowIndex][pk], column: col, value: value === '' ? null : value },
      });
      editing = null;
      error = '';
    } catch (e) {
      fail(e);
    }
  }

  function editKey(e) {
    if (e.key === 'Enter') saveEdit();
    if (e.key === 'Escape') editing = null;
  }

  async function saveNew(e) {
    e.preventDefault();
    const values = Object.fromEntries(Object.entries(adding).filter(([, v]) => v !== ''));
    try {
      await api(`/tables/${encodeURIComponent(current)}/rows`, { method: 'POST', body: { values } });
      adding = null;
      await Promise.all([load(), loadTables()]);
    } catch (e) {
      fail(e);
    }
  }

  const asText = (v) => (v == null ? '' : typeof v === 'object' ? JSON.stringify(v) : String(v));

  // Nome da linha: primeira coluna que não é a chave (ex.: nome do cliente).
  function rowTitle(row) {
    const head = data.columns.find((c) => !c.pk);
    return head && row[head.name] != null ? show(row[head.name], head) : `Linha ${row[pk]}`;
  }

  function openRow(i) {
    const cols = data.columns.filter((c) => !fixed(c));
    const row = i == null ? null : data.rows[i];
    rowForm = { index: i, values: Object.fromEntries(cols.map((c) => [c.name, row ? asText(row[c.name]) : ''])), busy: false, error: '' };
  }

  function newRow() {
    if (phone()) return openRow(null);
    adding = Object.fromEntries(data.columns.filter((c) => !fixed(c)).map((c) => [c.name, '']));
  }

  // Linha nova: um POST. Linha existente: um PATCH por campo que mudou.
  async function saveRow(e) {
    e.preventDefault();
    rowForm.busy = true;
    rowForm.error = '';
    const url = `/tables/${encodeURIComponent(current)}/rows`;
    try {
      if (rowForm.index == null) {
        const values = Object.fromEntries(Object.entries(rowForm.values).filter(([, v]) => v !== ''));
        await api(url, { method: 'POST', body: { values } });
        await loadTables();
      } else {
        const row = data.rows[rowForm.index];
        for (const [col, v] of Object.entries(rowForm.values)) {
          if (v === asText(row[col])) continue;
          data.rows[rowForm.index] = await api(url, { method: 'PATCH', body: { pk: row[pk], column: col, value: v === '' ? null : v } });
        }
      }
      rowForm = null;
      await load();
    } catch (err) {
      if (err.status === 401) return onAuthLost();
      rowForm.busy = false;
      rowForm.error = err.message;
    }
  }

  async function deleteSelected() {
    try {
      await api(`/tables/${encodeURIComponent(current)}/rows`, { method: 'DELETE', body: { pks: [...selected] } });
      selected = new Set();
      confirmDelete = false;
      await Promise.all([load(), loadTables()]);
    } catch (e) {
      confirmDelete = false;
      fail(e);
    }
  }

  let fileInput = $state(null);
  let notice = $state('');

  // Primeira linha do arquivo = nomes das colunas da tabela.
  async function importCsv(e) {
    const file = e.currentTarget.files[0];
    e.currentTarget.value = '';
    if (!file) return;
    error = '';
    notice = '';
    const [header, ...rows] = parseCsv(await file.text());
    if (!header || !rows.length) return (error = 'O arquivo está vazio ou só tem o cabeçalho.');
    try {
      const r = await api(`/tables/${encodeURIComponent(current)}/import`, {
        method: 'POST',
        body: { columns: header.map((h) => h.trim()), rows: rows.map((r) => header.map((_, i) => r[i] ?? '')) },
      });
      notice = `${n(r.imported)} ${r.imported === 1 ? 'linha importada' : 'linhas importadas'}.`;
      await Promise.all([load(), loadTables()]);
    } catch (err) {
      fail(err);
    }
  }

  function exportCsv() {
    const cols = data.columns.map((c) => c.name);
    const cell = (v) => `"${String(v == null ? '' : typeof v === 'object' ? JSON.stringify(v) : v).replaceAll('"', '""')}"`;
    const csv = [cols.map(cell).join(','), ...data.rows.map((r) => cols.map((c) => cell(r[c])).join(','))].join('\n');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['﻿' + csv], { type: 'text/csv' }));
    a.download = `${current}-pagina-${page}.csv`;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  function show(v, col) {
    if (v == null) return '—';
    if (col.label === 'data' && /^\d{4}-\d{2}-\d{2}$/.test(v)) return v.split('-').reverse().join('/');
    if (col.label === 'data e hora') return new Date(v).toLocaleString('pt-BR', { dateStyle: 'short', timeStyle: 'short' });
    if (col.label === 'sim/não') return v ? 'Sim' : 'Não';
    if (col.label === 'decimal' && typeof v === 'number') {
      const scale = Number(/numeric\(\d+,(\d+)\)/.exec(col.type)?.[1] ?? NaN);
      return v.toLocaleString('pt-BR', Number.isNaN(scale) ? {} : { minimumFractionDigits: scale, maximumFractionDigits: scale });
    }
    if (typeof v === 'object') return JSON.stringify(v);
    return String(v);
  }

  const isMono = (c) => c.pk || c.link || ['número', 'decimal', 'id único'].includes(c.label) || c.label.startsWith('data');

  loadTables().catch(fail);
</script>

<div class="side-list">
  <span class="label">Tabelas</span>
  {#each tables as t}
    <a class="list-item" class:active={t.name === current} href={href('tabelas', t.name)}>
      <span>{t.name}</span><span class="mono">{n(t.rows)}</span>
    </a>
  {/each}
</div>

{#if data}
  <section class="content">
    <div class="page-head">
      <div>
        <h1 class="brand">{current}</h1>
        {#if data?.owner}<p>Você vê só as linhas com o seu e-mail em {data.owner}.</p>{/if}
      </div>
      <div class="actions">
        {#if selected.size && canEdit()}
          <button class="glass btn btn-danger" onclick={() => (confirmDelete = true)}><Icon name="trash" size={15} /> Apagar {selected.size}</button>
        {/if}
        <a class="glass btn" href={href('estrutura', current)}><Icon name="Estrutura" size={15} stroke="#C9CCEB" /> Estrutura</a>
        <button class="glass btn" onclick={exportCsv}><Icon name="download" size={15} stroke="#C9CCEB" /> Exportar</button>
        {#if canEdit()}
          <button class="glass btn" onclick={() => fileInput.click()}><Icon name="upload" size={15} stroke="#C9CCEB" /> Importar</button>
          <input type="file" accept=".csv,text/csv" hidden bind:this={fileInput} onchange={importCsv} />
        {/if}
        {#if pk && canEdit()}
          <button class="btn-primary" onclick={newRow}>
            <Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Nova linha
          </button>
        {/if}
      </div>
    </div>

    <div class="tools">
      <div class="pop-wrap">
        <button class="glass chip" onclick={openFilter} aria-expanded={filterOpen}><Icon name="filter" size={14} stroke="#9AA0E0" /> Filtrar</button>
        {#if filterOpen}
          <form class="pop" onsubmit={applyFilter}>
            <select class="field small" bind:value={draft.col} aria-label="Coluna">
              {#each data.columns as c}<option value={c.name}>{c.name}</option>{/each}
            </select>
            <select class="field small op" bind:value={draft.op} aria-label="Condição">
              <option value="é">é</option>
              <option value="contém">contém</option>
            </select>
            <!-- svelte-ignore a11y_autofocus -->
            <input class="field small" bind:value={draft.val} placeholder="valor" aria-label="Valor" autofocus />
            <button class="btn-primary small">Aplicar</button>
          </form>
        {/if}
      </div>
      {#if filter}
        <span class="chip active-filter">
          {filter.col} <em>{filter.op}</em> <strong>{filter.val}</strong>
          <button class="x" aria-label="Remover filtro" onclick={() => { filter = null; page = 1; load(); }}><Icon name="close" size={14} stroke="#9AA0E0" /></button>
        </span>
      {/if}
      {#if sort}
        <span class="chip active-filter">
          ordem: {sort.col} <em>{sort.desc ? 'decrescente' : 'crescente'}</em>
          <button class="x" aria-label="Remover ordem" onclick={() => { sort = null; load(); }}><Icon name="close" size={14} stroke="#9AA0E0" /></button>
        </span>
      {/if}
      <span class="grow"></span>
      {#if error}<span class="error" role="alert">{error}</span>{/if}
      {#if notice}<span class="notice" role="status">{notice}</span>{/if}
      <span class="count">{data.approx ? 'cerca de ' : ''}{n(data.total)} {data.total === 1 ? 'linha' : 'linhas'}</span>
    </div>

    <!-- Celular: uma ficha por linha. Toque para ver e editar tudo. -->
    <div class="cards">
      {#each data.rows as row, i}
        {@const on = pk && selected.has(String(row[pk]))}
        {@const [head, ...rest] = data.columns.filter((c) => !c.pk)}
        <div class="card-edge" class:sel={on}>
          <div class="card">
            {#if pk && canEdit()}<input type="checkbox" class="card-check" checked={on} onchange={() => toggleRow(row)} aria-label="Marcar linha" />{/if}
            <button class="card-body" onclick={() => openRow(i)} disabled={!pk || !canEdit()} aria-label="Abrir linha {pk ? row[pk] : i + 1}">
              <span class="card-title">{head ? show(row[head.name], head) : row[pk]}</span>
              {#each rest.slice(0, 4) as c}
                <span class="card-kv"><span>{c.name}</span><strong class:dim={row[c.name] == null}>{show(row[c.name], c)}</strong></span>
              {/each}
              {#if pk}<span class="card-id mono">#{row[pk]}</span>{/if}
            </button>
          </div>
        </div>
      {:else}
        <p class="none">{filter ? 'Nada bate com esse filtro.' : 'Tabela vazia. Use "Nova linha" para começar.'}</p>
      {/each}
      <div class="cards-foot">
        <span>Página {data.page} de {data.pages}</span>
        <span class="pager">
          <button class="glass sq" disabled={page <= 1} aria-label="Página anterior" onclick={() => { page--; load(); }}><Icon name="left" size={14} stroke="#C9CCEB" /></button>
          <button class="glass sq" disabled={page >= data.pages} aria-label="Próxima página" onclick={() => { page++; load(); }}><Icon name="right" size={14} stroke="#C9CCEB" /></button>
        </span>
      </div>
    </div>

    <div class="edge">
      <div class="grid">
        <div class="scroll">
          <table>
            <thead>
              <tr>
                <th class="check">{#if pk}<input type="checkbox" checked={allOnPage} onchange={toggleAll} aria-label="Marcar a página" />{/if}</th>
                {#each data.columns as c}
                  <th aria-sort={sort?.col === c.name ? (sort.desc ? 'descending' : 'ascending') : undefined}>
                    <button class="sort" onclick={() => toggleSort(c.name)} title="Ordenar">
                      <span>{c.name}</span>
                      {#if sort?.col === c.name}<Icon name={sort.desc ? 'down' : 'up'} size={13} stroke="#E6FF00" />{/if}
                    </button>
                    <small class="mono">{c.link ? `→ ${c.link}` : c.pk ? 'chave' : c.label}</small>
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#if adding}
                <tr class="new">
                  <td class="check"></td>
                  {#each data.columns as c}
                    <td>
                      {#if c.pk}<span class="dim mono">auto</span>
                      {:else if c.name === data.owner}<span class="dim">você</span>
                      {:else}<input class="cell-input" bind:value={adding[c.name]} placeholder={(c.link ? `id em ${c.link}` : c.label) + (c.required ? " *" : "")} aria-label={c.name} />{/if}
                    </td>
                  {/each}
                </tr>
                <tr class="new-actions"><td colspan={data.columns.length + 1}>
                  <form onsubmit={saveNew}>
                    <button class="btn-primary small">Salvar linha</button>
                    <button type="button" class="glass btn small" onclick={() => (adding = null)}>Cancelar</button>
                    <span class="dim">Campo vazio usa o valor padrão. * é obrigatório.</span>
                  </form>
                </td></tr>
              {/if}
              {#each data.rows as row, i}
                {@const on = pk && selected.has(String(row[pk]))}
                <tr class:sel={on}>
                  <td class="check">{#if pk}<input type="checkbox" checked={on} onchange={() => toggleRow(row)} aria-label="Marcar linha" />{/if}</td>
                  {#each data.columns as c}
                    <td ondblclick={() => startEdit(i, c)} class:mono={isMono(c)} class:dimmed={c.pk}>
                      {#if editing?.rowIndex === i && editing.col === c.name}
                        <!-- svelte-ignore a11y_autofocus -->
                        <input class="cell-input editing" bind:value={editing.value} onkeydown={editKey} onblur={() => (editing = null)} autofocus aria-label={c.name} />
                      {:else if c.label === 'opção' && row[c.name] != null}
                        <span class="badge">{row[c.name]}</span>
                      {:else}
                        <span class:dim={row[c.name] == null}>{show(row[c.name], c)}</span>
                      {/if}
                    </td>
                  {/each}
                </tr>
              {:else}
                <tr><td class="none" colspan={data.columns.length + 1}>{filter ? 'Nada bate com esse filtro.' : 'Tabela vazia. Use "Nova linha" para começar.'}</td></tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="foot">
          <span>{selected.size ? `${selected.size} ${selected.size === 1 ? 'linha marcada' : 'linhas marcadas'}` : !canEdit() ? 'Seu acesso é só de leitura.' : pk ? 'Clique duas vezes numa célula para editar.' : 'Tabela sem chave: só leitura.'}</span>
          <span class="pager">
            Página {data.page} de {data.pages}
            <button class="glass sq" disabled={page <= 1} aria-label="Página anterior" onclick={() => { page--; load(); }}><Icon name="left" size={14} stroke="#C9CCEB" /></button>
            <button class="glass sq" disabled={page >= data.pages} aria-label="Próxima página" onclick={() => { page++; load(); }}><Icon name="right" size={14} stroke="#C9CCEB" /></button>
          </span>
        </div>
      </div>
    </div>
  </section>
{:else if tables.length === 0}
  <section class="content"><div class="empty-state"><Glyph size={56} /><p>Nenhuma tabela ainda.</p><a class="glass btn" href={href('estrutura')}>Abrir Estrutura</a></div></section>
{:else if error}
  <section class="content"><p class="error" role="alert">{error}</p></section>
{/if}

{#if rowForm}
  <Modal title={rowForm.index == null ? `Nova linha em ${current}` : rowTitle(data.rows[rowForm.index])} onClose={() => (rowForm = null)} width={520}>
    <form class="form" onsubmit={saveRow}>
      {#each data.columns.filter((c) => !fixed(c)) as c}
        <label class="stack">
          <span class="label">{c.name}{c.required ? ' *' : ''} <small class="mono">{c.link ? `id em ${c.link}` : c.label}</small></span>
          {#if c.label === 'sim/não'}
            <select class="field" bind:value={rowForm.values[c.name]}><option value="">vazio</option><option value="true">Sim</option><option value="false">Não</option></select>
          {:else}
            <input class="field" bind:value={rowForm.values[c.name]}
              type={c.label === 'data' ? 'date' : 'text'}
              inputmode={['número', 'decimal'].includes(c.label) || c.link ? 'decimal' : c.label === 'e-mail' ? 'email' : undefined} />
          {/if}
        </label>
      {/each}
      <p class="hint">Campo vazio usa o valor padrão. * é obrigatório.</p>
      {#if rowForm.error}<p class="error" role="alert">{rowForm.error}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (rowForm = null)}>Cancelar</button>
        <button class="btn-primary" disabled={rowForm.busy}>Salvar</button>
      </div>
    </form>
  </Modal>
{/if}

{#if confirmDelete}
  <Modal title="Apagar linhas" onClose={() => (confirmDelete = false)} width={440}>
    <p class="msg">Apagar {selected.size} {selected.size === 1 ? 'linha' : 'linhas'} de <strong>{current}</strong>? Não dá para desfazer.</p>
    <div class="modal-actions">
      <button class="glass btn" onclick={() => (confirmDelete = false)}>Cancelar</button>
      <button class="glass btn btn-danger solid" onclick={deleteSelected}><Icon name="trash" size={15} /> Apagar</button>
    </div>
  </Modal>
{/if}

<style>
  .tools { display: flex; align-items: center; gap: 8px; font-size: 13px; flex-wrap: wrap; }
  .chip { height: 34px; padding: 0 12px; border-radius: 9px; display: inline-flex; align-items: center; gap: 8px; font-size: 13px; color: var(--soft); }
  .active-filter { background: linear-gradient(180deg, #2A2E70 0%, #1E2158 100%); border: 1px solid rgba(154,160,224,.5); box-shadow: inset 0 1px 0 rgba(255,255,255,.14), 0 0 16px rgba(154,160,224,.14); padding-right: 6px; }
  .active-filter em { font-style: normal; color: var(--lavender-2); }
  .x { background: none; border: 0; display: grid; place-items: center; padding: 2px; }
  .count { color: var(--lavender-2); }
  .notice { color: var(--lime); }
  .pop-wrap { position: relative; }
  .pop { position: absolute; top: 42px; left: 0; z-index: 10; padding: 12px; border-radius: 14px; display: flex; align-items: center; gap: 8px; background: linear-gradient(180deg, #1F2259 0%, #15173F 100%); border: 1px solid rgba(154,160,224,.4); box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 20px 40px rgba(4,5,20,.6); }
  .pop .field { width: 150px; }
  .pop .op { width: 96px; }

  .edge { flex: 1; min-height: 240px; padding: 1px; border-radius: 18px; background: var(--edge); display: flex; }
  .grid { flex: 1; min-width: 0; border-radius: 17px; background: linear-gradient(180deg, #15173F 0%, #10112F 100%); box-shadow: inset 0 1px 0 rgba(255,255,255,.1); display: flex; flex-direction: column; overflow: hidden; }
  .scroll { flex: 1; overflow: auto; }
  table { width: 100%; border-collapse: separate; border-spacing: 0; font-size: 14px; }
  th { position: sticky; top: 0; z-index: 1; height: 54px; padding: 0 12px; text-align: left; font-weight: 500; font-size: 13px; color: var(--soft); white-space: nowrap; background: linear-gradient(180deg, #1B1D4E 0%, #16183F 100%); border-bottom: 1px solid var(--line-2); }
  th small { display: block; font-size: 10px; color: var(--lavender-3); margin-top: 3px; }
  .sort { padding: 0; border: 0; background: none; display: inline-flex; align-items: center; gap: 4px; font-weight: 500; color: inherit; }
  .sort:hover span { color: var(--text); }
  td { height: 44px; padding: 0 12px; border-bottom: 1px solid #1A1C46; white-space: nowrap; max-width: 280px; overflow: hidden; text-overflow: ellipsis; }
  td.mono { font-size: 12px; color: var(--lavender); }
  td.dimmed { color: var(--lavender-2); }
  td.none { height: 120px; text-align: center; color: var(--lavender-2); }
  .check { width: 44px; padding-left: 18px; }
  tr.sel td { background: rgba(154,160,224,.1); border-bottom-color: rgba(154,160,224,.3); }
  tbody tr:hover td { background: rgba(154,160,224,.05); }
  .cell-input { width: 100%; min-width: 80px; height: 30px; padding: 0 8px; border-radius: 7px; background: #0C0D28; border: 1px solid #2A2E6A; font-size: 14px; outline: none; }
  .cell-input:focus, .cell-input.editing { border-color: var(--lime); box-shadow: 0 0 0 3px rgba(230,255,0,.12); }
  tr.new td { background: rgba(230,255,0,.03); }
  .new-actions td { height: 52px; }
  .new-actions form { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .foot { height: 52px; flex-shrink: 0; padding: 0 16px; border-top: 1px solid var(--line-2); display: flex; align-items: center; justify-content: space-between; gap: 12px; font-size: 13px; color: var(--soft); }
  .pager { display: flex; align-items: center; gap: 10px; color: var(--lavender); white-space: nowrap; }
  .sq { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; }
  .sq:disabled { opacity: .4; cursor: default; }

  .form { display: flex; flex-direction: column; gap: 14px; }
  .stack { display: flex; flex-direction: column; gap: 6px; }
  .stack small { font-size: 10px; color: var(--lavender-3); margin-left: 6px; text-transform: none; letter-spacing: 0; }
  .hint { font-size: 13px; color: var(--lavender-2); }

  /* Fichas: só no celular. */
  .cards { display: none; flex-direction: column; gap: 10px; }
  .card-edge { padding: 1px; border-radius: 16px; background: var(--edge); }
  .card-edge.sel { background: linear-gradient(135deg, rgba(230,255,0,.6), rgba(154,160,224,.2)); }
  .card { position: relative; border-radius: 15px; background: linear-gradient(180deg, #17194A 0%, #11123A 100%); box-shadow: var(--inner); }
  .card-check { position: absolute; top: 16px; right: 16px; width: 20px; height: 20px; }
  .card-body { width: 100%; padding: 14px 52px 14px 16px; border: 0; background: none; color: inherit; text-align: left; display: flex; flex-direction: column; gap: 6px; font: inherit; }
  .card-title { font-weight: 600; font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-kv { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
  .card-kv span { color: var(--lavender-2); flex-shrink: 0; }
  .card-kv strong { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-id { font-size: 11px; color: var(--lavender-3); }
  .cards .none { padding: 32px 0; text-align: center; color: var(--lavender-2); }
  .cards-foot { display: flex; align-items: center; justify-content: space-between; padding: 4px 2px 8px; font-size: 13px; color: var(--lavender); }

  @media (max-width: 700px) {
    .cards { display: flex; }
    .edge { display: none; }
    .page-head .actions :is(a, button):not(.btn-primary):not(.btn-danger) { display: none; }
  }

  .msg { color: var(--soft); font-size: 15px; line-height: 22px; }
  .msg strong { color: var(--text); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; }

</style>
