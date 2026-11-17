<script>
  import { api } from './api.js';
  import Glyph from './Glyph.svelte';

  let { user, onLogout } = $props();

  let tables = $state([]);
  let current = $state(null);
  let data = $state(null); // { columns, rows, total, page, pages }
  let page = $state(1);
  let filter = $state(null); // { col, val }
  let filterOpen = $state(false);
  let draftCol = $state('');
  let draftVal = $state('');
  let selected = $state(null);
  let editing = $state(null); // { rowIndex, col, value }
  let adding = $state(null); // { col: value }
  let error = $state('');

  const nav = ['Início', 'Tabelas', 'Estrutura', 'Relações', 'Consultas', 'Acesso', 'API e chaves', 'Backups', 'Atividade'];
  const icons = {
    'Início': 'M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z',
    'Tabelas': 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM3 10h18M9 4v16',
    'Estrutura': 'M4 6h16M4 12h16M4 18h10',
    'Relações': 'M6 3a3 3 0 1 0 0 6a3 3 0 1 0 0-6M18 15a3 3 0 1 0 0 6a3 3 0 1 0 0-6M9 6h3a3 3 0 0 1 3 3v6',
    'Consultas': 'M4 5h16l-6 7v6l-4 2v-8z',
    'Acesso': 'M12 4a4 4 0 1 0 0 8a4 4 0 1 0 0-8M4 21a8 8 0 0 1 16 0',
    'API e chaves': 'M8 11a4 4 0 1 0 0 8a4 4 0 1 0 0-8M11 12l9-9M17 6l3 3',
    'Backups': 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 7v5l3 2',
    'Atividade': 'M3 12h4l3-8 4 16 3-8h4',
  };

  const pk = $derived(data?.columns.find((c) => c.pk)?.name);
  const initials = $derived(user.name.split(' ').map((w) => w[0]).slice(0, 2).join('').toUpperCase());

  async function loadTables() {
    tables = await api('/tables');
    if (!current && tables.length) open(tables[0].name);
  }

  async function open(name) {
    if (current !== name) { current = name; page = 1; filter = null; selected = null; adding = null; }
    await load();
  }

  async function load() {
    error = '';
    editing = null;
    const q = new URLSearchParams({ page });
    if (filter) { q.set('col', filter.col); q.set('val', filter.val); }
    try {
      data = await api(`/tables/${encodeURIComponent(current)}?${q}`);
    } catch (e) {
      if (e.status === 401) onLogout();
      error = e.message;
    }
  }

  function applyFilter(e) {
    e.preventDefault();
    filter = draftVal ? { col: draftCol, val: draftVal } : null;
    filterOpen = false;
    page = 1;
    load();
  }

  function startEdit(i, col) {
    if (col.pk || !pk) return;
    const v = data.rows[i][col.name];
    editing = { rowIndex: i, col: col.name, value: v == null ? '' : String(v) };
  }

  async function saveEdit() {
    const { rowIndex, col, value } = editing;
    const row = data.rows[rowIndex];
    try {
      data.rows[rowIndex] = await api(`/tables/${encodeURIComponent(current)}/rows`, {
        method: 'PATCH',
        body: { pk: row[pk], column: col, value: value === '' ? null : value },
      });
      editing = null;
      error = '';
    } catch (e) {
      error = e.message;
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
      error = e.message;
    }
  }

  function exportCsv() {
    const cols = data.columns.map((c) => c.name);
    const cell = (v) => `"${String(v ?? '').replaceAll('"', '""')}"`;
    const csv = [cols.map(cell).join(','), ...data.rows.map((r) => cols.map((c) => cell(r[c])).join(','))].join('\n');
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob(['﻿' + csv], { type: 'text/csv' }));
    a.download = `${current}-pagina-${page}.csv`;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  async function logout() {
    await api('/logout', { method: 'POST' }).catch(() => {});
    onLogout();
  }

  function show(v, col) {
    if (v == null) return '—';
    if (col.label === 'data' && /^\d{4}-\d{2}-\d{2}$/.test(v)) return v.split('-').reverse().join('/');
    if (col.label === 'data e hora') return new Date(v).toLocaleString('pt-BR', { dateStyle: 'short', timeStyle: 'short' });
    if (col.label === 'sim/não') return v ? 'Sim' : 'Não';
    if (typeof v === 'object') return JSON.stringify(v);
    return String(v);
  }

  const n = (x) => x.toLocaleString('pt-BR');
  loadTables().catch((e) => (e.status === 401 ? onLogout() : (error = e.message)));
</script>

<div class="shell">
  <aside class="side">
    <div class="logo"><Glyph size={26} /><span class="brand">Masterbase</span></div>

    <div class="glass project">
      <span class="avatar-sq brand">PA</span>
      <span class="grow"><strong>Padaria Aurora</strong><small>Hospedado</small></span>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 9l4-4 4 4M8 15l4 4 4-4" /></svg>
    </div>

    <nav>
      {#each nav as item}
        <span class="nav-item" class:active={item === 'Tabelas'} aria-current={item === 'Tabelas' ? 'page' : undefined} title={item === 'Tabelas' ? undefined : 'Em breve'}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={icons[item]} /></svg>
          {item}
        </span>
      {/each}
    </nav>

    <div class="grow"></div>
    <div class="me">
      <span class="avatar">{initials}</span>
      <span class="grow"><strong>{user.name}</strong><small>{user.email}</small></span>
      <button class="icon-btn" onclick={logout} title="Sair" aria-label="Sair">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 17l5-5-5-5M15 12H4" /></svg>
      </button>
    </div>
  </aside>

  <div class="main">
    <div class="top">
      <span class="crumb">Tabelas <i>/</i> <strong>{current ?? ''}</strong></span>
    </div>

    <div class="body">
      <div class="list">
        <span class="label list-title">Tabelas</span>
        {#each tables as t}
          <button class="table-item" class:active={t.name === current} onclick={() => open(t.name)}>
            <span>{t.name}</span><span class="mono">{n(t.rows)}</span>
          </button>
        {/each}
      </div>

      {#if data}
        <section class="content">
          <div class="head">
            <h1 class="brand">{current}</h1>
            <div class="actions">
              <button class="glass btn" onclick={exportCsv}>
                <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#C9CCEB" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12M7 10l5 5 5-5M4 15v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4" /></svg>
                Exportar
              </button>
              <button class="btn-primary" onclick={() => (adding = Object.fromEntries(data.columns.filter((c) => !c.pk).map((c) => [c.name, ''])))}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#12133A" stroke-width="2.6" stroke-linecap="round"><path d="M12 5v14M5 12h14" /></svg>
                Nova linha
              </button>
            </div>
          </div>

          <div class="tools">
            <div class="pop-wrap">
              <button class="glass chip" onclick={() => { filterOpen = !filterOpen; draftCol = filter?.col ?? data.columns[1]?.name ?? data.columns[0].name; draftVal = filter?.val ?? ''; }}>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 5h16l-6 7v6l-4 2v-8z" /></svg>
                Filtrar
              </button>
              {#if filterOpen}
                <form class="pop" onsubmit={applyFilter}>
                  <select class="field small" bind:value={draftCol}>
                    {#each data.columns as c}<option value={c.name}>{c.name}</option>{/each}
                  </select>
                  <span class="is">é</span>
                  <!-- svelte-ignore a11y_autofocus -->
                  <input class="field small" bind:value={draftVal} placeholder="valor" autofocus />
                  <button class="btn-primary small">Aplicar</button>
                </form>
              {/if}
            </div>
            {#if filter}
              <span class="chip active-filter">
                {filter.col} <em>é</em> <strong>{filter.val}</strong>
                <button class="x" aria-label="Remover filtro" onclick={() => { filter = null; page = 1; load(); }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18" /></svg>
                </button>
              </span>
            {/if}
            <span class="grow"></span>
            {#if error}<span class="error" role="alert">{error}</span>{/if}
            <span class="count">{n(data.total)} {data.total === 1 ? 'linha' : 'linhas'}</span>
          </div>

          <div class="edge">
            <div class="grid">
              <div class="scroll">
                <table>
                  <thead>
                    <tr>
                      <th class="check"></th>
                      {#each data.columns as c}<th><span>{c.name}</span><small class="mono">{c.label}</small></th>{/each}
                    </tr>
                  </thead>
                  <tbody>
                    {#if adding}
                      <tr class="new">
                        <td class="check"></td>
                        {#each data.columns as c}
                          <td>
                            {#if c.pk}<span class="dim mono">auto</span>
                            {:else}<input class="cell-input" bind:value={adding[c.name]} placeholder={c.label} />{/if}
                          </td>
                        {/each}
                      </tr>
                      <tr class="new-actions"><td colspan={data.columns.length + 1}>
                        <form onsubmit={saveNew}><button class="btn-primary small">Salvar linha</button> <button type="button" class="glass btn small" onclick={() => (adding = null)}>Cancelar</button> <span class="dim">Campos vazios usam o valor padrão.</span></form>
                      </td></tr>
                    {/if}
                    {#each data.rows as row, i}
                      <tr class:sel={selected === i}>
                        <td class="check"><input type="checkbox" checked={selected === i} onchange={() => (selected = selected === i ? null : i)} aria-label="Selecionar linha" /></td>
                        {#each data.columns as c}
                          <td ondblclick={() => startEdit(i, c)} class:mono={c.pk || c.label === 'número' || c.label.startsWith('data')} class:dimmed={c.pk}>
                            {#if editing?.rowIndex === i && editing.col === c.name}
                              <!-- svelte-ignore a11y_autofocus -->
                              <input class="cell-input editing" bind:value={editing.value} onkeydown={editKey} onblur={() => (editing = null)} autofocus />
                            {:else if c.label === 'opção' && row[c.name] != null}
                              <span class="badge">{row[c.name]}</span>
                            {:else}
                              <span class:dim={row[c.name] == null}>{show(row[c.name], c)}</span>
                            {/if}
                          </td>
                        {/each}
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
              <div class="foot">
                <span>{selected != null ? '1 linha selecionada' : pk ? 'Clique duas vezes numa célula para editar.' : 'Tabela sem chave primária: só leitura.'}</span>
                <span class="pager">
                  Página {data.page} de {data.pages}
                  <button class="glass sq" disabled={page <= 1} aria-label="Página anterior" onclick={() => { page--; load(); }}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#C9CCEB" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6l-6 6 6 6" /></svg>
                  </button>
                  <button class="glass sq" disabled={page >= data.pages} aria-label="Próxima página" onclick={() => { page++; load(); }}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#C9CCEB" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 6l6 6-6 6" /></svg>
                  </button>
                </span>
              </div>
            </div>
          </div>
        </section>
      {:else if tables.length === 0}
        <section class="content empty"><Glyph size={56} /><p>Nenhuma tabela ainda.</p></section>
      {/if}
    </div>
  </div>
</div>

<style>
  .shell { display: flex; height: 100vh; overflow: hidden; }
  .grow { flex: 1; min-width: 0; }
  small { display: block; font-size: 12px; color: var(--lavender); }
  strong { font-weight: 600; }

  .side { width: 248px; flex-shrink: 0; padding: 20px 16px; background: linear-gradient(180deg, #0E0F2C 0%, #0B0C24 100%); border-right: 1px solid var(--line); display: flex; flex-direction: column; gap: 20px; }
  .logo { height: 36px; padding: 0 6px; display: flex; align-items: center; gap: 10px; font-size: 19px; }
  .project { height: 52px; padding: 0 12px; border-radius: 12px; display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .avatar-sq { width: 30px; height: 30px; border-radius: 8px; background: #2A2E6A; display: grid; place-items: center; font-size: 13px; font-weight: 700; }
  nav { display: flex; flex-direction: column; gap: 2px; }
  .nav-item { position: relative; height: 38px; padding: 0 12px; border-radius: 10px; border: 1px solid transparent; display: flex; align-items: center; gap: 12px; font-size: 14px; color: var(--lavender); stroke: var(--lavender-2); }
  .nav-item:not(.active) { opacity: .8; cursor: default; }
  .nav-item.active { background: linear-gradient(180deg, #23266A 0%, #191B4E 100%); border-color: rgba(154,160,224,.32); box-shadow: inset 0 1px 0 rgba(255,255,255,.12); color: var(--text); font-weight: 600; stroke: var(--lime); }
  .nav-item.active::before { content: ''; position: absolute; left: -9px; top: 10px; width: 3px; height: 16px; border-radius: 2px; background: var(--lime); }
  .me { padding: 12px 8px 0; border-top: 1px solid var(--line); display: flex; align-items: center; gap: 10px; font-size: 13px; }
  .me small { color: var(--lavender-2); overflow: hidden; text-overflow: ellipsis; }
  .avatar { width: 32px; height: 32px; flex-shrink: 0; border-radius: 50%; background: linear-gradient(180deg, #B3B8EE, #9AA0E0); color: var(--base); display: grid; place-items: center; font-size: 12px; font-weight: 600; }
  .icon-btn { width: 30px; height: 30px; border-radius: 8px; border: 1px solid transparent; background: none; display: grid; place-items: center; }
  .icon-btn:hover { border-color: var(--line-2); }

  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .top { height: 64px; flex-shrink: 0; padding: 0 28px; border-bottom: 1px solid var(--line); display: flex; align-items: center; }
  .crumb { font-size: 14px; color: var(--lavender-2); }
  .crumb i { font-style: normal; color: #3E4386; margin: 0 6px; }
  .crumb strong { color: var(--text); }

  .body { flex: 1; min-height: 0; display: flex; }
  .list { width: 220px; flex-shrink: 0; padding: 20px 12px; border-right: 1px solid var(--line); display: flex; flex-direction: column; gap: 4px; overflow-y: auto; }
  .list-title { padding: 0 8px 10px; color: var(--lavender-2); }
  .table-item { height: 36px; padding: 0 10px; border-radius: 9px; border: 1px solid transparent; background: none; display: flex; align-items: center; justify-content: space-between; font-size: 14px; color: var(--lavender); text-align: left; }
  .table-item .mono { font-size: 11px; color: var(--lavender-3); }
  .table-item:hover { background: rgba(154,160,224,.06); }
  .table-item.active { background: linear-gradient(180deg, #20235A 0%, #17194A 100%); border-color: rgba(154,160,224,.3); box-shadow: inset 0 1px 0 rgba(255,255,255,.1); color: var(--text); font-weight: 600; }
  .table-item.active .mono { color: var(--lavender); }

  .content { flex: 1; min-width: 0; padding: 24px 28px; display: flex; flex-direction: column; gap: 18px; }
  .empty { align-items: center; justify-content: center; color: var(--lavender); }
  .head { display: flex; align-items: flex-end; justify-content: space-between; gap: 16px; }
  h1 { font-size: 30px; line-height: 34px; }
  .actions { display: flex; gap: 10px; }

  .tools { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .chip { height: 34px; padding: 0 12px; border-radius: 9px; display: inline-flex; align-items: center; gap: 8px; font-size: 13px; color: var(--soft); }
  .active-filter { background: linear-gradient(180deg, #2A2E70 0%, #1E2158 100%); border: 1px solid rgba(154,160,224,.5); box-shadow: inset 0 1px 0 rgba(255,255,255,.14), 0 0 16px rgba(154,160,224,.14); padding-right: 6px; }
  .active-filter em { font-style: normal; color: var(--lavender-2); }
  .x { background: none; border: 0; display: grid; place-items: center; padding: 2px; }
  .count { color: var(--lavender-2); }
  .error { color: var(--err); }
  .pop-wrap { position: relative; }
  .pop { position: absolute; top: 42px; left: 0; z-index: 10; padding: 12px; border-radius: 14px; display: flex; align-items: center; gap: 8px; background: linear-gradient(180deg, #1F2259 0%, #15173F 100%); border: 1px solid rgba(154,160,224,.4); box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 20px 40px rgba(4,5,20,.6); }
  .is { color: var(--lavender-2); }
  .field.small { height: 36px; width: 150px; font-size: 13px; border-radius: 9px; }
  .btn-primary.small, .btn.small { height: 34px; font-size: 13px; padding: 0 14px; }

  .edge { flex: 1; min-height: 0; padding: 1px; border-radius: 18px; background: var(--edge); display: flex; }
  .grid { flex: 1; min-width: 0; border-radius: 17px; background: linear-gradient(180deg, #15173F 0%, #10112F 100%); box-shadow: inset 0 1px 0 rgba(255,255,255,.1); display: flex; flex-direction: column; overflow: hidden; }
  .scroll { flex: 1; overflow: auto; }
  table { width: 100%; border-collapse: separate; border-spacing: 0; font-size: 14px; }
  th { position: sticky; top: 0; z-index: 1; height: 54px; padding: 0 12px; text-align: left; font-weight: 500; font-size: 13px; color: var(--soft); white-space: nowrap; background: linear-gradient(180deg, #1B1D4E 0%, #16183F 100%); border-bottom: 1px solid var(--line-2); }
  th small { font-family: 'Martian Mono', monospace; font-size: 10px; color: var(--lavender-3); margin-top: 3px; }
  td { height: 44px; padding: 0 12px; border-bottom: 1px solid #1A1C46; white-space: nowrap; max-width: 280px; overflow: hidden; text-overflow: ellipsis; }
  td.mono { font-size: 12px; color: var(--lavender); }
  td.dimmed { color: var(--lavender-2); }
  .check { width: 44px; padding-left: 18px; }
  tr.sel td { background: rgba(154,160,224,.1); border-bottom-color: rgba(154,160,224,.3); }
  tbody tr:hover td { background: rgba(154,160,224,.05); }
  input[type='checkbox'] { accent-color: var(--lime); width: 16px; height: 16px; }
  .badge { height: 22px; padding: 0 9px; border-radius: 6px; display: inline-flex; align-items: center; background: rgba(154,160,224,.16); border: 1px solid rgba(154,160,224,.35); font-size: 12px; font-weight: 600; color: #DDE0FF; }
  .dim { color: var(--lavender-3); }
  .cell-input { width: 100%; min-width: 80px; height: 30px; padding: 0 8px; border-radius: 7px; background: #0C0D28; border: 1px solid #2A2E6A; font-size: 14px; outline: none; }
  .cell-input:focus, .cell-input.editing { border-color: var(--lime); box-shadow: 0 0 0 3px rgba(230,255,0,.12); }
  tr.new td { background: rgba(230,255,0,.03); }
  .new-actions td { height: 52px; }
  .new-actions form { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .foot { height: 52px; flex-shrink: 0; padding: 0 16px; border-top: 1px solid var(--line-2); display: flex; align-items: center; justify-content: space-between; font-size: 13px; color: var(--soft); }
  .pager { display: flex; align-items: center; gap: 10px; color: var(--lavender); }
  .sq { width: 30px; height: 30px; border-radius: 8px; display: grid; place-items: center; }
  .sq:disabled { opacity: .4; cursor: default; }
</style>
