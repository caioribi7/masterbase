<script>
  import { api } from './api.js';
  import { ago } from './lib.js';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { onAuthLost } = $props();

  let keys = $state([]);
  let tables = $state([]);
  let error = $state('');
  let modal = $state(null); // 'new' | { token }
  let form = $state({ name: '', role: 'leitor' });
  let formError = $state('');
  let busy = $state(false);
  let copied = $state(false);

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));
  const load = () => api('/keys').then((r) => (keys = r)).catch(fail);
  load();
  api('/tables').then((t) => (tables = t)).catch(() => {});
  let rate = $state(null); // pedidos por minuto por chave
  api('/settings').then((r) => (rate = r.key_rate)).catch(() => {});

  const table = $derived(tables[0]?.name ?? 'sua_tabela');
  const base = $derived(`${location.origin}/api/tables/${encodeURIComponent(table)}`);

  async function create(e) {
    e.preventDefault();
    busy = true;
    formError = '';
    try {
      const r = await api('/keys', { method: 'POST', body: form });
      modal = { token: r.token };
      copied = false;
      await load();
    } catch (err) {
      err.status === 401 ? onAuthLost() : (formError = err.message);
    }
    busy = false;
  }

  const revoke = (k) => api(`/keys/${k.id}`, { method: 'DELETE' }).then(load).catch(fail);

  async function copy(text) {
    await navigator.clipboard.writeText(text).catch(() => {});
    copied = true;
  }
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">API e chaves</h1>
      <p>Para um site ou sistema ler e gravar nas tabelas sem passar pela tela.</p>
    </div>
    <div class="actions">
      <button class="btn-primary" onclick={() => { form = { name: '', role: 'leitor' }; formError = ''; modal = 'new'; }}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Nova chave</button>
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="panel-edge"><div class="panel scroll-x">
    <table class="simple">
      <thead><tr><th>Nome</th><th>Chave</th><th>Pode</th><th>Criada por</th><th>Último uso</th><th></th></tr></thead>
      <tbody>
        {#each keys as k (k.id)}
          <tr>
            <td><strong>{k.name}</strong></td>
            <td class="mono">{k.prefix}…</td>
            <td><span class="badge">{k.role === 'editor' ? 'ler e editar' : 'só ler'}</span></td>
            <td>{k.user}</td>
            <td>{k.last_used_at ? ago(k.last_used_at) : 'nunca'}</td>
            <td class="end"><button class="glass btn small btn-danger" onclick={() => revoke(k)}>Revogar</button></td>
          </tr>
        {:else}
          <tr><td class="none" colspan="6">Nenhuma chave ainda.</td></tr>
        {/each}
      </tbody>
    </table>
  </div></div>

  <div class="panel-edge"><div class="panel how">
    <span class="label">Como usar</span>
    <p class="hint">Mande a chave no cabeçalho <span class="mono">Authorization</span>. Chave nunca mexe em estrutura, pessoas ou backups.{#if rate} Cada chave faz até {rate} pedidos por minuto; passou disso, o servidor responde 429 até o minuto virar.{/if}</p>
    <span class="cap">Ler linhas (com filtro e ordem)</span>
    <pre class="code">curl -H "Authorization: Bearer SUA_CHAVE" \
  "{base}?page=1&sort=id&desc=true"</pre>
    <span class="cap">Adicionar uma linha (chave que edita)</span>
    <pre class="code">curl -X POST -H "Authorization: Bearer SUA_CHAVE" -H "Content-Type: application/json" \
  -d '{`{"values": {"coluna": "valor"}}`}' \
  "{base}/rows"</pre>
    <span class="cap">Em JavaScript</span>
    <pre class="code">{`const r = await fetch("${base}", {
  headers: { Authorization: "Bearer SUA_CHAVE" },
});
const { rows } = await r.json();`}</pre>
  </div></div>
</section>

{#if modal === 'new'}
  <Modal title="Nova chave" onClose={() => (modal = null)} width={460}>
    <form class="form" onsubmit={create}>
      <label class="stack"><span class="label">Nome (onde vai ser usada)</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={form.name} placeholder="ex.: site da padaria" required autofocus /></label>
      <label class="stack"><span class="label">O que ela pode</span>
        <select class="field" bind:value={form.role}><option value="leitor">só ler</option><option value="editor">ler e editar linhas</option></select></label>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (modal = null)}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Criar chave</button>
      </div>
    </form>
  </Modal>
{:else if modal?.token}
  <Modal title="Copie a chave agora" onClose={() => (modal = null)} width={560}>
    <p class="msg">Ela aparece só desta vez. Guardamos apenas um resumo dela, então não dá para mostrar de novo.</p>
    <pre class="code">{modal.token}</pre>
    <div class="modal-actions">
      <button class="glass btn" onclick={() => copy(modal.token)}><Icon name="copy" size={15} stroke="#C9CCEB" /> {copied ? 'Copiada' : 'Copiar'}</button>
      <button class="btn-primary" onclick={() => (modal = null)}>Já guardei</button>
    </div>
  </Modal>
{/if}

<style>
  .how { padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; }
  .how .label { color: var(--lavender-2); }
  .cap { margin-top: 6px; font-size: 13px; color: var(--soft); }
</style>
