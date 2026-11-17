<script>
  import { api } from './api.js';
  import { go, href, intent, kinds } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { arg, onAuthLost, onOpen } = $props();

  let schema = $state(null); // [{ name, columns: [...] }]
  let error = $state('');
  let modal = $state(null); // 'create' | 'rename' | 'drop' | 'column'
  let busy = $state(false);
  let formError = $state('');

  // Formulários
  const blankCol = () => ({ name: '', kind: 'texto', link: '', required: false, unique: false });
  let newTable = $state({ name: '', columns: [blankCol()] });
  let newCol = $state(blankCol());
  let renameTo = $state('');
  let dropConfirm = $state('');
  let editingCol = $state(null); // { from, name }

  const current = $derived(schema?.find((t) => t.name === arg) ?? schema?.[0] ?? null);
  const others = $derived(schema?.map((t) => t.name) ?? []);

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));

  async function load() {
    try {
      schema = await api('/schema');
      error = '';
    } catch (e) {
      fail(e);
    }
  }
  load().then(() => {
    if (intent.createTable) { intent.createTable = false; openCreate(); }
  });

  $effect(() => { if (current) onOpen(current.name); });

  function close() { modal = null; formError = ''; busy = false; }

  function openCreate() {
    newTable = { name: '', columns: [blankCol()] };
    modal = 'create';
  }

  const clean = (c) => ({ name: c.name, kind: c.kind, required: c.required, unique: c.unique, link: c.kind === 'ligação' ? c.link || null : null });

  // Uma ação de formulário: chama a API, recarrega e fecha.
  async function act(fn) {
    busy = true;
    formError = '';
    try {
      await fn();
      await load();
      close();
    } catch (e) {
      busy = false;
      e.status === 401 ? onAuthLost() : (formError = e.message);
    }
  }

  const t = () => encodeURIComponent(current.name);

  function createTable(e) {
    e.preventDefault();
    act(async () => {
      const r = await api('/schema', { method: 'POST', body: { name: newTable.name, columns: newTable.columns.filter((c) => c.name.trim()).map(clean) } });
      go('estrutura', r.name);
    });
  }

  function renameTable(e) {
    e.preventDefault();
    act(async () => {
      const r = await api(`/schema/${t()}`, { method: 'PATCH', body: { name: renameTo } });
      go('estrutura', r.name);
    });
  }

  function dropTable(e) {
    e.preventDefault();
    act(async () => {
      await api(`/schema/${t()}`, { method: 'DELETE', body: { confirm: dropConfirm } });
      go('estrutura');
    });
  }

  function addColumn(e) {
    e.preventDefault();
    act(() => api(`/schema/${t()}/columns`, { method: 'POST', body: clean(newCol) }));
  }

  async function quick(fn) {
    try {
      await fn();
      await load();
    } catch (e) {
      fail(e);
    }
  }

  const colUrl = (name) => `/schema/${t()}/columns/${encodeURIComponent(name)}`;
  const setRequired = (c) => quick(() => api(colUrl(c.name), { method: 'PATCH', body: { required: !c.required } }));
  let dropCol = $state(null);
  function dropColumn(e) {
    e.preventDefault();
    act(() => api(colUrl(dropCol.name), { method: 'DELETE' }));
  }

  function saveColName(e) {
    e?.preventDefault();
    const { from, name } = editingCol;
    editingCol = null;
    if (name.trim() && name !== from) quick(() => api(colUrl(from), { method: 'PATCH', body: { name } }));
  }

  function colKey(e) {
    if (e.key === 'Escape') editingCol = null;
  }
</script>

<div class="side-list">
  <span class="label">Tabelas</span>
  {#each others as name}
    <a class="list-item" class:active={name === current?.name} href={href('estrutura', name)}><span>{name}</span></a>
  {/each}
  <button class="list-item add" onclick={openCreate}><Icon name="plus" size={14} /> Nova tabela</button>
</div>

<section class="content">
  {#if current}
    <div class="page-head">
      <div>
        <h1 class="brand">{current.name}</h1>
        <p>{current.columns.length} {current.columns.length === 1 ? 'coluna' : 'colunas'}. Mudanças valem na hora para todas as linhas.</p>
      </div>
      <div class="actions">
        <a class="glass btn" href={href('tabelas', current.name)}><Icon name="grid" size={15} stroke="#C9CCEB" /> Ver dados</a>
        <button class="glass btn" onclick={() => { renameTo = current.name; modal = 'rename'; }}><Icon name="pencil" size={15} stroke="#C9CCEB" /> Renomear</button>
        <button class="glass btn btn-danger" onclick={() => { dropConfirm = ''; modal = 'drop'; }}><Icon name="trash" size={15} /> Apagar tabela</button>
        <button class="btn-primary" onclick={() => { newCol = blankCol(); modal = 'column'; }}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Nova coluna</button>
      </div>
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="panel-edge"><div class="panel">
      <div class="cols-head mono"><span>Coluna</span><span>Tipo</span><span>Regras</span><span></span></div>
      {#each current.columns as c (c.name)}
        <div class="col-row">
          <span class="col-name">
            {#if editingCol?.from === c.name}
              <form onsubmit={saveColName}>
                <!-- svelte-ignore a11y_autofocus -->
                <input class="field small" bind:value={editingCol.name} onkeydown={colKey} onblur={saveColName} autofocus aria-label="Novo nome" />
              </form>
            {:else}
              <strong>{c.name}</strong>
              {#if !c.pk}<button class="icon-btn" title="Renomear" aria-label="Renomear {c.name}" onclick={() => (editingCol = { from: c.name, name: c.name })}><Icon name="pencil" size={14} /></button>{/if}
            {/if}
          </span>
          <span>
            {#if c.link}<a class="badge" href={href('estrutura', c.link)}><Icon name="link" size={12} /> {c.link}</a>
            {:else}<span class="badge">{c.label}</span>{/if}
          </span>
          <span class="rules">
            {#if c.pk}
              <span class="badge lime"><Icon name="key" size={12} /> chave automática</span>
            {:else}
              <label class="check-label"><input type="checkbox" checked={c.required} onchange={() => setRequired(c)} /> obrigatório</label>
              {#if c.unique}<span class="badge">sem repetidos</span>{/if}
            {/if}
          </span>
          <span class="end">
            {#if !c.pk}<button class="icon-btn danger" title="Apagar coluna" aria-label="Apagar {c.name}" onclick={() => { dropCol = c; modal = 'dropcol'; }}><Icon name="trash" size={15} /></button>{/if}
          </span>
        </div>
      {/each}
    </div></div>
  {:else if schema}
    <div class="empty-state">
      <Glyph size={56} />
      <p>Nenhuma tabela ainda. Crie a primeira e as colunas que ela precisa.</p>
      <button class="btn-primary" onclick={openCreate}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Criar tabela</button>
    </div>
  {:else if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

{#snippet colFields(c, i)}
  <div class="col-form">
    <input class="field small" bind:value={c.name} placeholder="nome da coluna" aria-label="Nome da coluna" />
    <select class="field small" bind:value={c.kind} aria-label="Tipo">
      {#each kinds as k}<option value={k}>{k === 'ligação' ? 'ligação com…' : k}</option>{/each}
    </select>
    {#if c.kind === 'ligação'}
      <select class="field small" bind:value={c.link} aria-label="Tabela ligada" required>
        <option value="" disabled>tabela</option>
        {#each others as o}<option value={o}>{o}</option>{/each}
      </select>
    {:else}<span></span>{/if}
    <label class="check-label"><input type="checkbox" bind:checked={c.required} /> obrigatório</label>
    <label class="check-label"><input type="checkbox" bind:checked={c.unique} /> sem repetidos</label>
    {#if i != null && newTable.columns.length > 1}
      <button type="button" class="icon-btn" aria-label="Tirar coluna" onclick={() => newTable.columns.splice(i, 1)}><Icon name="close" size={14} /></button>
    {:else}<span></span>{/if}
  </div>
{/snippet}

{#if modal === 'create'}
  <Modal title="Nova tabela" onClose={close} width={760}>
    <form class="form" onsubmit={createTable}>
      <label class="stack"><span class="label">Nome da tabela</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={newTable.name} placeholder="ex.: entregas" required autofocus />
      </label>
      <div class="stack">
        <span class="label">Colunas</span>
        <p class="hint"><Icon name="key" size={13} /> A coluna <span class="mono">id</span> é criada sozinha e numera cada linha.</p>
        {#each newTable.columns as c, i}{@render colFields(c, i)}{/each}
        <button type="button" class="glass btn small add-col" onclick={() => newTable.columns.push(blankCol())}><Icon name="plus" size={14} /> Mais uma coluna</button>
      </div>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Criar tabela</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'column'}
  <Modal title="Nova coluna em {current.name}" onClose={close} width={760}>
    <form class="form" onsubmit={addColumn}>
      {@render colFields(newCol, null)}
      <p class="hint">Linhas que já existem ficam com a coluna vazia. Por isso "obrigatório" só funciona em tabela vazia.</p>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Criar coluna</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'rename'}
  <Modal title="Renomear tabela" onClose={close} width={440}>
    <form class="form" onsubmit={renameTable}>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field" bind:value={renameTo} aria-label="Novo nome" required autofocus />
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Renomear</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'dropcol'}
  <Modal title="Apagar coluna" onClose={close} width={440}>
    <form class="form" onsubmit={dropColumn}>
      <p class="msg">Apagar <strong>{dropCol.name}</strong> de {current.name}? Os valores dessa coluna somem de todas as linhas.</p>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="glass btn btn-danger solid" disabled={busy}><Icon name="trash" size={15} /> Apagar coluna</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'drop'}
  <Modal title="Apagar tabela" onClose={close} width={460}>
    <form class="form" onsubmit={dropTable}>
      <p class="msg">Isso apaga <strong>{current.name}</strong> e todas as linhas dela. Não dá para desfazer.</p>
      <label class="stack"><span class="label">Digite {current.name} para confirmar</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={dropConfirm} autocomplete="off" autofocus />
      </label>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="glass btn btn-danger solid" disabled={busy || dropConfirm !== current.name}><Icon name="trash" size={15} /> Apagar para sempre</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .cols-head, .col-row { display: grid; grid-template-columns: minmax(160px, 1.3fr) minmax(120px, 1fr) minmax(200px, 1.4fr) 44px; align-items: center; gap: 16px; padding: 0 20px; }
  .cols-head { height: 46px; font-size: 10px; letter-spacing: .08em; text-transform: uppercase; color: var(--lavender-3); border-bottom: 1px solid var(--line-2); }
  .col-row { min-height: 56px; border-bottom: 1px solid #1A1C46; font-size: 14px; }
  .col-row:last-child { border-bottom: 0; }
  .col-row:hover { background: rgba(154,160,224,.04); }
  .col-name { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .col-name strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .col-name .icon-btn { opacity: 0; }
  .col-row:hover .col-name .icon-btn, .col-name .icon-btn:focus-visible { opacity: 1; }
  .col-name form { flex: 1; }
  a.badge { text-decoration: none; }
  a.badge:hover { border-color: rgba(154,160,224,.6); }
  .rules { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .end { display: flex; justify-content: flex-end; }
  .danger:hover:not(:disabled) { color: var(--err); border-color: rgba(255,122,138,.4); }

  .form { display: flex; flex-direction: column; gap: 16px; }
  .stack { display: flex; flex-direction: column; gap: 8px; }
  .hint { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--lavender-2); }
  .col-form { display: grid; grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr) minmax(0, 1fr) auto auto 30px; align-items: center; gap: 10px; padding: 10px; border-radius: 12px; background: rgba(12,13,40,.5); border: 1px solid #1E2152; }
  .col-form .field { width: 100%; }
  .check-label { white-space: nowrap; }
  .add-col { align-self: flex-start; }
  .msg { color: var(--soft); font-size: 15px; line-height: 22px; }
  .msg strong { color: var(--text); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .modal-actions button:disabled { opacity: .5; cursor: not-allowed; }

  @media (max-width: 1100px) {
    .cols-head { display: none; }
    .col-row { grid-template-columns: 1fr auto; padding: 12px 16px; gap: 8px; }
    .col-form { grid-template-columns: 1fr 1fr; }
  }
</style>
