<script>
  import { api } from './api.js';
  import { bytes } from './lib.js';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { onAuthLost } = $props();

  let items = $state([]);
  let error = $state('');
  let done = $state('');
  let busy = $state(false);
  let restoring = $state(null); // nome do backup
  let confirm = $state('');
  let formError = $state('');

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));
  const load = () => api('/backups').then((r) => (items = r)).catch(fail);
  load();

  // backup-20261117-053015.dump (UTC) -> 17/11/2026 às 02:30 no horário de quem vê
  function when(name) {
    const m = /backup-(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})\.dump/.exec(name);
    if (!m) return name;
    const d = new Date(Date.UTC(m[1], m[2] - 1, m[3], m[4], m[5], m[6]));
    return `${d.toLocaleDateString('pt-BR')} às ${d.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' })}`;
  }

  async function create() {
    busy = true;
    error = '';
    done = '';
    try {
      await api('/backups', { method: 'POST' });
      done = 'Backup feito.';
      await load();
    } catch (e) {
      fail(e);
    }
    busy = false;
  }

  const remove = (b) => api(`/backups/${b.name}`, { method: 'DELETE' }).then(load).catch(fail);

  async function restore(e) {
    e.preventDefault();
    busy = true;
    formError = '';
    try {
      await api(`/backups/${restoring}/restore`, { method: 'POST', body: { confirm } });
      done = `Banco restaurado para ${when(restoring)}. O estado anterior ficou guardado em um backup novo.`;
      restoring = null;
      await load();
    } catch (err) {
      err.status === 401 ? onAuthLost() : (formError = err.message);
    }
    busy = false;
  }
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Backups</h1>
      <p>Cópias das suas tabelas, guardadas no servidor. Baixe uma de vez em quando para ter fora dele também.</p>
    </div>
    <div class="actions">
      <button class="btn-primary" disabled={busy} onclick={create}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Fazer backup agora</button>
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if done}<p class="ok" role="status">{done}</p>{/if}

  <div class="panel-edge"><div class="panel scroll-x">
    <table class="simple">
      <thead><tr><th>Quando</th><th>Tamanho</th><th></th></tr></thead>
      <tbody>
        {#each items as b (b.name)}
          <tr>
            <td><strong>{when(b.name)}</strong></td>
            <td class="mono">{bytes(b.size)}</td>
            <td class="end">
              <a class="glass btn small" href="/api/backups/{b.name}" download><Icon name="download" size={14} stroke="#C9CCEB" /> Baixar</a>
              <button class="glass btn small" onclick={() => { restoring = b.name; confirm = ''; formError = ''; }}><Icon name="restore" size={14} stroke="#C9CCEB" /> Restaurar</button>
              <button class="glass btn small btn-danger" onclick={() => remove(b)}>Apagar</button>
            </td>
          </tr>
        {:else}
          <tr><td class="none" colspan="3">Nenhum backup ainda.</td></tr>
        {/each}
      </tbody>
    </table>
  </div></div>
  <p class="hint">O backup guarda as tabelas e os dados. Contas, chaves e histórico de atividade não entram. Ainda não há backup automático: faça antes de mudanças grandes.</p>
</section>

{#if restoring}
  <Modal title="Restaurar backup" onClose={() => (restoring = null)} width={480}>
    <form class="form" onsubmit={restore}>
      <p class="msg">As tabelas voltam a ser como eram em <strong>{when(restoring)}</strong>. O que mudou depois disso se perde nessas tabelas.</p>
      <p class="hint">Antes de restaurar, o Masterbase guarda um backup do estado de agora. Tabelas criadas depois desse backup continuam existindo.</p>
      <label class="stack"><span class="label">Digite restaurar para confirmar</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={confirm} autocomplete="off" autofocus /></label>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (restoring = null)}>Cancelar</button>
        <button class="glass btn btn-danger solid" disabled={busy || confirm !== 'restaurar'}><Icon name="restore" size={15} /> Restaurar</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .end > * { margin-left: 6px; }
  .ok { font-size: 13px; color: var(--lime); }
  button:disabled { opacity: .6; }
</style>
