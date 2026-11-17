<script>
  import { api } from './api.js';
  import { go, href, intent, kinds } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import { isAdmin } from './session.svelte.js';

  let { arg, onAuthLost, onOpen } = $props();

  let schema = $state(null); // [{ name, columns: [...] }]
  let error = $state('');
  let modal = $state(null); // 'create' | 'rename' | 'drop' | 'column'
  let busy = $state(false);
  let formError = $state('');

  // Formulários
  const blankCol = () => ({ name: '', kind: 'texto', link: '', required: false, unique: false, def: 'nenhum', defValue: '' });
  let newTable = $state({ name: '', columns: [blankCol()] });
  let newCol = $state(blankCol());
  let renameTo = $state('');
  let dropConfirm = $state('');
  let editCol = $state(null); // { col, name, kind, required, def, defValue }

  // Valor padrão: o que cada tipo aceita além de "nenhum" e "fixo".
  const timeKinds = ['data', 'data e hora'];
  const defOptions = (kind) => [
    ['nenhum', 'nenhum'],
    ['fixo', 'um valor fixo'],
    ...(timeKinds.includes(kind) ? [['agora', kind === 'data' ? 'a data de hoje' : 'a hora em que a linha é criada']] : []),
    ...(kind === 'id único' ? [['id novo', 'um id novo para cada linha']] : []),
  ];
  // Agora/hoje mudam de nome conforme o tipo; o servidor usa "hoje" para data e "agora" para data e hora.
  const defMode = (kind, def) => (def === 'agora' && kind === 'data' ? 'hoje' : def);
  const defText = (d) => (!d || d.mode === 'auto' ? '' : d.mode === 'fixo' ? `padrão: ${d.value}` : d.mode === 'outro' ? 'padrão: fórmula' : `padrão: ${d.mode}`);

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

  const clean = (c) => ({
    name: c.name, kind: c.kind, required: c.required, unique: c.unique,
    link: c.kind === 'ligação' ? c.link || null : null,
    default: c.kind === 'ligação' || !c.def || c.def === 'nenhum' ? null : { mode: defMode(c.kind, c.def), value: c.defValue ?? '' },
  });

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

  function openEdit(c) {
    const d = c.default;
    const def = !d ? 'nenhum' : d.mode === 'hoje' ? 'agora' : ['fixo', 'agora', 'id novo'].includes(d.mode) ? d.mode : 'outro';
    editCol = { col: c, name: c.name, kind: kinds.includes(c.label) ? c.label : '', required: c.required, def, defValue: d?.mode === 'fixo' ? d.value : '' };
    modal = 'editcol';
  }

  function saveCol(e) {
    e.preventDefault();
    const { col, name, kind, required, def, defValue } = editCol;
    const body = {};
    if (name.trim() !== col.name) body.name = name;
    if (required !== col.required) body.required = required;
    const typeChanged = kind && kind !== col.label;
    if (typeChanged) body.kind = kind;
    // Tipo mudou: o padrão antigo cai; manda o escolhido de novo. Senão, só se mudou.
    const before = col.default ? (col.default.mode === 'hoje' ? 'agora' : col.default.mode) : 'nenhum';
    if (def !== 'outro' && (typeChanged || def !== before || (def === 'fixo' && defValue !== col.default?.value))) {
      body.default = { mode: defMode(kind || col.label, def), value: defValue };
    }
    if (!Object.keys(body).length) return close();
    act(() => api(colUrl(col.name), { method: 'PATCH', body }));
  }
</script>

<div class="side-list">
  <span class="label">Tabelas</span>
  {#each others as name}
    <a class="list-item" class:active={name === current?.name} href={href('estrutura', name)}><span>{name}</span></a>
  {/each}
  {#if isAdmin()}<button class="list-item add" onclick={openCreate}><Icon name="plus" size={14} /> Nova tabela</button>{/if}
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
        {#if isAdmin()}
        <button class="glass btn" onclick={() => { renameTo = current.name; modal = 'rename'; }}><Icon name="pencil" size={15} stroke="#C9CCEB" /> Renomear</button>
        <button class="glass btn btn-danger" onclick={() => { dropConfirm = ''; modal = 'drop'; }}><Icon name="trash" size={15} /> Apagar tabela</button>
        <button class="btn-primary" onclick={() => { newCol = blankCol(); modal = 'column'; }}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Nova coluna</button>
        {/if}
      </div>
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="panel-edge"><div class="panel">
      <div class="cols-head mono"><span>Coluna</span><span>Tipo</span><span>Regras</span><span></span></div>
      {#each current.columns as c (c.name)}
        <div class="col-row">
          <span class="col-name">
            <strong>{c.name}</strong>
          </span>
          <span>
            {#if c.link}<a class="badge" href={href('estrutura', c.link)}><Icon name="link" size={12} /> {c.link}</a>
            {:else}<span class="badge">{c.label}</span>{/if}
          </span>
          <span class="rules">
            {#if c.pk}
              <span class="badge lime"><Icon name="key" size={12} /> chave automática</span>
            {:else}
              <label class="check-label"><input type="checkbox" checked={c.required} disabled={!isAdmin()} onchange={() => setRequired(c)} /> obrigatório</label>
              {#if c.unique}<span class="badge">sem repetidos</span>{/if}
              {#if defText(c.default)}<span class="badge soft" title={c.default.value ?? ''}>{defText(c.default)}</span>{/if}
            {/if}
          </span>
          <span class="end">
            {#if !c.pk && isAdmin()}<button class="icon-btn" title="Editar coluna" aria-label="Editar {c.name}" onclick={() => openEdit(c)}><Icon name="pencil" size={15} /></button>{/if}
            {#if !c.pk && isAdmin()}<button class="icon-btn danger" title="Apagar coluna" aria-label="Apagar {c.name}" onclick={() => { dropCol = c; modal = 'dropcol'; }}><Icon name="trash" size={15} /></button>{/if}
          </span>
        </div>
      {/each}
    </div></div>
  {:else if schema}
    <div class="empty-state">
      <Glyph size={56} />
      <p>Nenhuma tabela ainda. Crie a primeira e as colunas que ela precisa.</p>
      {#if isAdmin()}<button class="btn-primary" onclick={openCreate}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Criar tabela</button>{/if}
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

{#snippet defaultFields(c, kind)}
  <div class="def-row">
    <label class="stack"><span class="label">Valor padrão</span>
      <select class="field" bind:value={c.def}>
        {#each defOptions(kind) as [v, l]}<option value={v}>{l}</option>{/each}
      </select></label>
    {#if c.def === 'fixo'}
      <label class="stack grow"><span class="label">Valor</span>
        {#if kind === 'sim/não'}
          <select class="field" bind:value={c.defValue}><option value="true">sim</option><option value="false">não</option></select>
        {:else}
          <input class="field" bind:value={c.defValue} placeholder={kind === 'data' ? 'aaaa-mm-dd' : kind === 'número' || kind === 'decimal' ? '0' : ''} />
        {/if}</label>
    {/if}
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
      {#if newCol.kind !== 'ligação'}
        {@render defaultFields(newCol, newCol.kind)}
      {/if}
      <p class="hint">Linhas que já existem ficam com o valor padrão, ou vazias se não houver. "Obrigatório" sem padrão só funciona em tabela vazia.</p>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Criar coluna</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'editcol'}
  <Modal title="Editar coluna" onClose={close} width={520}>
    <form class="form" onsubmit={saveCol}>
      <label class="stack"><span class="label">Nome</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={editCol.name} required autofocus /></label>
      <label class="stack"><span class="label">Tipo</span>
        {#if editCol.col.link}
          <span class="hint"><Icon name="link" size={13} /> Ligação com {editCol.col.link}. Para mudar, apague e crie outra coluna.</span>
        {:else}
          <select class="field" bind:value={editCol.kind}>
            {#if !editCol.kind}<option value="">{editCol.col.label} (atual)</option>{/if}
            {#each kinds.filter((k) => k !== 'ligação') as k}<option value={k}>{k}</option>{/each}
          </select>
          {#if editCol.kind && editCol.kind !== editCol.col.label}
            <span class="hint">Os valores atuais são convertidos. Se algum não servir no tipo novo, nada muda e você vê o aviso.</span>
          {/if}
        {/if}
      </label>
      {#if !editCol.col.link}
        {#if editCol.def === 'outro'}
          <p class="hint">Valor padrão: uma fórmula feita fora do Masterbase ({editCol.col.default.value}). <button type="button" class="linkish" onclick={() => (editCol.def = 'nenhum')}>Trocar</button></p>
        {:else}
          {@render defaultFields(editCol, editCol.kind || editCol.col.label)}
        {/if}
      {/if}
      <label class="check-label"><input type="checkbox" bind:checked={editCol.required} /> obrigatório (não aceita ficar vazio)</label>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={close}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Salvar</button>
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
  .cols-head, .col-row { display: grid; grid-template-columns: minmax(160px, 1.3fr) minmax(120px, 1fr) minmax(200px, 1.4fr) 84px; align-items: center; gap: 16px; padding: 0 20px; }
  .cols-head { height: 46px; font-size: 10px; letter-spacing: .08em; text-transform: uppercase; color: var(--lavender-3); border-bottom: 1px solid var(--line-2); }
  .col-row { min-height: 56px; border-bottom: 1px solid #1A1C46; font-size: 14px; }
  .col-row:last-child { border-bottom: 0; }
  .col-row:hover { background: rgba(154,160,224,.04); }
  .col-name { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .col-name strong { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge.soft { color: var(--lavender-2); max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .def-row { display: flex; gap: 12px; align-items: flex-end; flex-wrap: wrap; }
  .def-row > .stack { min-width: 200px; }
  .def-row .grow { flex: 1; }
  .linkish { background: none; border: 0; padding: 0; color: var(--text); text-decoration: underline; text-underline-offset: 3px; cursor: pointer; font-size: 13px; }
  a.badge { text-decoration: none; }
  a.badge:hover { border-color: rgba(154,160,224,.6); }
  .rules { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .end { display: flex; justify-content: flex-end; gap: 4px; }
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
