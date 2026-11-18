<script>
  // Editor SQL: só administradores. Roda num acesso que lê e muda linhas, numa transação.
  import { api } from './api.js';
  import { n } from './lib.js';
  import Icon from './Icon.svelte';

  let { onAuthLost } = $props();
  let sql = $state(load() ?? "SELECT table_name FROM information_schema.tables\nWHERE table_schema = 'public'\nORDER BY 1;");
  let readOnly = $state(true);
  let busy = $state(false);
  let error = $state('');
  let result = $state(null);
  let tables = $state([]);
  api('/tables').then((t) => (tables = t.map((x) => x.name))).catch(() => {});

  // Atalho (bom no celular, onde digitar SQL cansa): começa uma consulta na tabela tocada.
  const start = (t) => (sql = `SELECT *
FROM "${t.replaceAll('"', '""')}"
LIMIT 50;`);

  function load() {
    try { return localStorage.getItem('mb.sql'); } catch { return null; }
  }
  $effect(() => {
    try { localStorage.setItem('mb.sql', sql); } catch {}
  });

  async function run() {
    if (busy || !sql.trim()) return;
    busy = true;
    error = '';
    try {
      result = await api('/sql', { method: 'POST', body: { sql, read_only: readOnly } });
    } catch (e) {
      if (e.status === 401) return onAuthLost();
      error = e.message;
      result = null;
    } finally {
      busy = false;
    }
  }

  function key(e) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); run(); }
    // Tab insere espaços em vez de sair do campo.
    if (e.key === 'Tab' && !e.shiftKey) {
      e.preventDefault();
      const t = e.currentTarget, s = t.selectionStart;
      sql = sql.slice(0, s) + '  ' + sql.slice(t.selectionEnd);
      requestAnimationFrame(() => (t.selectionStart = t.selectionEnd = s + 2));
    }
  }

  const summary = $derived.by(() => {
    if (!result) return '';
    const parts = [];
    if (result.columns.length) parts.push(`${n(result.rows.length)} ${result.rows.length === 1 ? 'linha' : 'linhas'}${result.truncated ? ' (mostrando as primeiras 1.000)' : ''}`);
    else parts.push(`${n(result.affected)} ${result.affected === 1 ? 'linha afetada' : 'linhas afetadas'}`);
    if (result.statements > 1) parts.push(`${result.statements} comandos, mostrando o último`);
    parts.push(`${result.ms} ms`);
    return parts.join(' · ');
  });
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Editor SQL</h1>
      <p>Para quem sabe SQL. Lê e muda as linhas das suas tabelas direto no Postgres. Criar e mudar tabelas fica na Estrutura.</p>
    </div>
    <div class="actions run-top">
      <label class="check-label" title="Nada é gravado. Comandos que mudam dados dão erro.">
        <input type="checkbox" bind:checked={readOnly} /> Só leitura
      </label>
      <button class="btn-primary" onclick={run} disabled={busy}><Icon name="play" size={14} stroke="#12133A" weight={2.4} /> Rodar</button>
    </div>
  </div>

  {#if tables.length}
    <div class="starts" aria-label="Começar pela tabela">
      {#each tables as t}<button type="button" class="pill" onclick={() => start(t)}>{t}</button>{/each}
    </div>
  {/if}

  <div class="panel-edge"><div class="panel editor">
    <textarea class="mono" bind:value={sql} onkeydown={key} spellcheck="false" aria-label="Comando SQL" placeholder="SELECT * FROM clientes;"></textarea>
    <div class="bar mono">
      <span class="desk">Ctrl+Enter roda</span>
      <span>{readOnly ? 'só leitura' : 'pode mudar dados'}</span>
    </div>
  </div></div>

  <div class="run-bottom">
    <label class="check-label"><input type="checkbox" bind:checked={readOnly} /> Só leitura</label>
    <button class="btn-primary" onclick={run} disabled={busy}><Icon name="play" size={14} stroke="#12133A" weight={2.4} /> Rodar</button>
  </div>

  {#if !readOnly}
    <p class="warn">Com "só leitura" desmarcado, UPDATE, DELETE e DROP valem de verdade. Faça um backup antes de mudanças grandes.</p>
  {/if}
  {#if error}<p class="error mono" role="alert">{error}</p>{/if}

  {#if result}
    <p class="hint">{summary}</p>
    {#if result.columns.length}
      <div class="cards">
        {#each result.rows as row, i (i)}
          <div class="card-edge"><div class="card">
            {#each result.columns as c, j}
              <div class="card-kv"><span>{c}</span><strong class:dim={row[j] === null}>{row[j] ?? 'vazio'}</strong></div>
            {/each}
          </div></div>
        {:else}
          <p class="none">Nenhuma linha.</p>
        {/each}
      </div>
      <div class="panel-edge results"><div class="panel scroll-x">
        <table class="simple">
          <thead><tr>{#each result.columns as c}<th>{c}</th>{/each}</tr></thead>
          <tbody>
            {#each result.rows as row}
              <tr>{#each row as v}<td class:dim={v === null} title={v ?? ''}>{v ?? 'vazio'}</td>{/each}</tr>
            {/each}
          </tbody>
        </table>
        {#if !result.rows.length}<p class="none">Nenhuma linha.</p>{/if}
      </div></div>
    {/if}
  {/if}
</section>

<style>
  .editor { display: flex; flex-direction: column; overflow: hidden; }
  textarea {
    min-height: 180px; height: 220px; resize: vertical; padding: 16px 18px; border: 0; outline: none;
    background: #0C0D28; color: var(--soft); font-size: 13px; line-height: 21px; tab-size: 2;
    border-radius: 17px 17px 0 0;
  }
  textarea:focus { box-shadow: inset 0 0 0 1px rgba(230,255,0,.35); }
  .bar { height: 34px; padding: 0 16px; display: flex; align-items: center; justify-content: space-between; font-size: 10px; letter-spacing: .06em; text-transform: uppercase; color: var(--lavender-3); border-top: 1px solid var(--line-2); }
  .warn { font-size: 13px; color: #FFC2CA; }
  .results { flex-shrink: 0; }
  .results .scroll-x { max-height: 60vh; overflow: auto; }
  .simple td { font-family: 'Martian Mono', monospace; font-size: 12px; height: 40px; }
  .starts { display: flex; gap: 6px; overflow-x: auto; scrollbar-width: none; flex-shrink: 0; }
  .pill { height: 30px; padding: 0 12px; flex-shrink: 0; border-radius: 8px; border: 1px solid #2A2E6A; background: #0C0D28; font-size: 13px; color: var(--lavender-3); font-family: 'Martian Mono', monospace; }
  .pill:hover { color: var(--text); border-color: rgba(154,160,224,.5); }
  .run-bottom { display: none; }

  .cards { display: none; flex-direction: column; gap: 10px; }
  .card-edge { padding: 1px; border-radius: 16px; background: var(--edge); }
  .card { border-radius: 15px; padding: 12px 16px; display: flex; flex-direction: column; gap: 6px; background: linear-gradient(180deg, #17194A 0%, #11123A 100%); box-shadow: var(--inner); }
  .card-kv { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
  .card-kv span { color: var(--lavender-2); flex-shrink: 0; font-family: 'Martian Mono', monospace; font-size: 11px; }
  .card-kv strong { font-weight: 500; font-family: 'Martian Mono', monospace; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .card-kv .dim { color: var(--lavender-3); }
  .cards .none { padding: 24px 0; text-align: center; color: var(--lavender-2); }

  /* Celular: texto a 16px (o iPhone não dá zoom ao tocar), Rodar logo abaixo do editor, resultado em fichas. */
  @media (max-width: 700px) {
    .run-top, .desk { display: none; }
    textarea { font-size: 16px; line-height: 24px; height: 180px; }
    .run-bottom { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
    .run-bottom .btn-primary { flex: 1; max-width: 220px; justify-content: center; height: 44px; }
    .cards { display: flex; }
    .results { display: none; }
  }
</style>
