<script>
  import { api } from './api.js';
  import { bytes } from './lib.js';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { onAuthLost } = $props();

  let items = $state([]);
  let cfg = $state(null);
  let error = $state('');
  let done = $state('');
  let busy = $state(false);
  let restoring = $state(null); // backup escolhido
  let confirm = $state('');
  let formError = $state('');
  let keyOpen = $state(false);
  let keyPw = $state('');
  let key = $state('');
  let keySaved = $state(false);
  let keyMsg = $state('');

  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));
  const load = () => api('/backups').then((r) => (items = r)).catch(fail);
  const loadCfg = () => api('/backups/settings').then((r) => (cfg = r)).catch(fail);
  load();
  loadCfg();

  const kindLabel = { manual: 'Manual', auto: 'Automático', antes: 'Antes de restaurar' };

  function when(ms) {
    const d = new Date(ms);
    return `${d.toLocaleDateString('pt-BR')} às ${d.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' })}`;
  }

  // O servidor guarda o minuto do dia em UTC; a tela mostra e escolhe no horário de quem vê.
  const tz = new Date().getTimezoneOffset();
  const localHour = (minute) => Math.floor((((minute - tz) % 1440) + 1440) % 1440 / 60);
  const toUtc = (hour) => (((hour * 60 + tz) % 1440) + 1440) % 1440;
  const hh = (h) => `${String(h).padStart(2, '0')}:00`;

  function nextText(ms) {
    if (ms - Date.now() < 5 * 60 * 1000) return 'nos próximos minutos';
    const d = new Date(ms);
    const today = new Date().toDateString() === d.toDateString();
    return `${today ? 'hoje' : 'amanhã'} às ${d.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' })}`;
  }

  async function change(body) {
    error = '';
    try { cfg = await api('/backups/settings', { method: 'PATCH', body }); } catch (e) { fail(e); }
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
      await api(`/backups/${restoring.name}/restore`, { method: 'POST', body: { confirm } });
      done = `Banco restaurado para ${when(restoring.at)}. O estado anterior ficou guardado num backup "Antes de restaurar".`;
      restoring = null;
      await load();
    } catch (err) {
      err.status === 401 ? onAuthLost() : (formError = err.message);
    }
    busy = false;
  }

  function openKey() {
    keyOpen = true; keyPw = ''; key = ''; keyMsg = ''; formError = '';
    keySaved = !!cfg?.key_saved;
  }

  async function revealKey(e) {
    e.preventDefault();
    formError = '';
    try { key = (await api('/backups/key', { method: 'POST', body: { password: keyPw } })).key; keyPw = ''; }
    catch (err) { err.status === 401 ? onAuthLost() : (formError = err.message); }
  }

  async function copyKey() {
    try { await navigator.clipboard.writeText(key); keyMsg = 'Chave copiada.'; }
    catch { keyMsg = 'Selecione a chave acima e copie.'; }
  }

  async function closeKey() {
    if (key && keySaved !== !!cfg?.key_saved) await change({ key_saved: keySaved });
    keyOpen = false; key = '';
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

  {#if cfg?.encrypted && !cfg.key_saved}
    <div class="panel-edge warn-edge"><div class="panel warn">
      <Icon name="key" size={18} stroke="var(--lime)" />
      <p><strong>Guarde a chave de recuperação.</strong> Os backups saem criptografados. Se este servidor se perder, só essa chave abre os arquivos em outra máquina.</p>
      <button class="glass btn" onclick={openKey}>Ver chave</button>
    </div></div>
  {/if}

  {#if cfg}
    <div class="panel-edge"><div class="panel card">
      <div class="row">
        <div class="grow">
          <span class="label">Backup automático</span>
          <p class="hint">
            {#if cfg.enabled}
              Todo dia. O próximo sai {nextText(cfg.next)}. Os mais velhos que a regra abaixo são apagados sozinhos.
            {:else}
              Desligado. Só existe backup quando alguém clica em "Fazer backup agora".
            {/if}
          </p>
        </div>
        <label class="time" class:off={!cfg.enabled}>
          <span class="label dim">Horário</span>
          <select class="field small" disabled={!cfg.enabled} value={localHour(cfg.minute)} onchange={(e) => change({ minute: toUtc(+e.currentTarget.value) })}>
            {#each Array(24) as _, h}<option value={h}>{hh(h)}</option>{/each}
          </select>
        </label>
        <label class="switch">
          <input type="checkbox" checked={cfg.enabled} onchange={(e) => change({ enabled: e.currentTarget.checked })} />
          {cfg.enabled ? 'Ligado' : 'Desligado'}
        </label>
      </div>
      {#if cfg.enabled}
        <div class="keep">
          <label class="time"><span class="label dim">Guardar os diários</span>
            <select class="field small" value={cfg.keep_daily} onchange={(e) => change({ keep_daily: +e.currentTarget.value })}>
              {#each [3, 7, 14, 30, 60] as n}<option value={n}>últimos {n}</option>{/each}
            </select></label>
          <label class="time"><span class="label dim">E mais um por semana</span>
            <select class="field small" value={cfg.keep_weekly} onchange={(e) => change({ keep_weekly: +e.currentTarget.value })}>
              {#each [0, 4, 8, 12, 26, 52] as n}<option value={n}>{n === 0 ? 'nenhum' : `${n} semanas`}</option>{/each}
            </select></label>
        </div>
      {/if}
      {#if cfg.failed}<p class="error" role="alert">O último backup automático falhou: {cfg.failed} Tentamos de novo em uma hora.</p>{/if}
      <div class="facts">
        <span class="badge" class:lime={cfg.encrypted}><Icon name="key" size={12} /> {cfg.encrypted ? 'Criptografados' : 'Sem criptografia'}</span>
        {#if cfg.encrypted}<button class="link" onclick={openKey}>Chave de recuperação</button>{/if}
      </div>
    </div></div>

    <div class="panel-edge"><div class="panel card">
      <span class="label">Cópia fora do servidor</span>
      {#if cfg.copy.dir || cfg.copy.hook}
        <p class="hint">
          Cada backup novo vai também para
          {#if cfg.copy.dir}a pasta <code class="mono">{cfg.copy.dir}</code>{/if}{#if cfg.copy.dir && cfg.copy.hook} e {/if}{#if cfg.copy.hook}o comando de cópia configurado{/if}.
          {#if cfg.copy.last}
            {#if cfg.copy.last.error}<span class="error">A última cópia falhou ({when(cfg.copy.last.at)}): {cfg.copy.last.error}</span>
            {:else}Última cópia: {when(cfg.copy.last.at)}.{/if}
          {/if}
        </p>
      {:else}
        <p class="hint">Desligada. Se o disco deste servidor morrer, os backups vão junto. Para mandar cada backup para outro lugar, coloque no arquivo de configuração <code class="mono">MB_BACKUP_COPY_DIR</code> (uma pasta em outro disco) ou <code class="mono">MB_BACKUP_HOOK</code> (um comando, como o rclone para Google Drive, S3 ou Backblaze) e reinicie. O passo a passo está em docs/instalacao.md.</p>
      {/if}
    </div></div>
  {/if}

  <div class="panel-edge"><div class="panel scroll-x">
    <table class="simple">
      <thead><tr><th>Quando</th><th>Tipo</th><th>Tamanho</th><th></th></tr></thead>
      <tbody>
        {#each items as b (b.name)}
          <tr>
            <td><strong>{when(b.at)}</strong></td>
            <td><span class="kind">{kindLabel[b.kind]}</span>{#if !b.encrypted}<span class="badge plain" title="Feito antes da criptografia ou sem chave">aberto</span>{/if}</td>
            <td class="mono">{bytes(b.size)}</td>
            <td class="end">
              <a class="glass btn small" href="/api/backups/{b.name}" download><Icon name="download" size={14} stroke="#C9CCEB" /> Baixar</a>
              <button class="glass btn small" onclick={() => { restoring = b; confirm = ''; formError = ''; }}><Icon name="restore" size={14} stroke="#C9CCEB" /> Restaurar</button>
              <button class="glass btn small btn-danger" onclick={() => remove(b)}>Apagar</button>
            </td>
          </tr>
        {:else}
          <tr><td class="none" colspan="4">Nenhum backup ainda.</td></tr>
        {/each}
      </tbody>
    </table>
  </div></div>
  <p class="hint">O backup guarda as tabelas e os dados. Contas, chaves de API e histórico de atividade não entram. Os arquivos criptografados só abrem com a chave de recuperação: <code class="mono">masterbase recuperar arquivo.dump.age</code> restaura em qualquer servidor.</p>
</section>

{#if restoring}
  <Modal title="Restaurar backup" onClose={() => (restoring = null)} width={480}>
    <form class="form" onsubmit={restore}>
      <p class="msg">As tabelas voltam a ser como eram em <strong>{when(restoring.at)}</strong>. O que mudou depois disso se perde nessas tabelas.</p>
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

{#if keyOpen}
  <Modal title="Chave de recuperação" onClose={closeKey} width={540}>
    {#if !key}
      <form class="form" onsubmit={revealKey}>
        <p class="msg">É ela que abre os backups. Fica no servidor, mas se o servidor se perder, a chave vai junto: guarde uma cópia num gerenciador de senhas.</p>
        <label class="stack"><span class="label">Sua senha, para mostrar a chave</span>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field" type="password" bind:value={keyPw} required autocomplete="current-password" autofocus /></label>
        {#if formError}<p class="error" role="alert">{formError}</p>{/if}
        <div class="modal-actions">
          <button type="button" class="glass btn" onclick={closeKey}>Cancelar</button>
          <button class="btn-primary">Mostrar chave</button>
        </div>
      </form>
    {:else}
      <div class="form">
        <code class="code key">{key}</code>
        <div class="copy">
          <button class="glass btn" onclick={copyKey}><Icon name="copy" size={15} stroke="#C9CCEB" /> Copiar</button>
          {#if keyMsg}<span class="ok">{keyMsg}</span>{/if}
        </div>
        <p class="hint">Quem tiver esta chave e um arquivo de backup lê os dados. Não mande por e-mail nem chat. No servidor, <code class="mono">masterbase chave-backup</code> mostra a mesma chave.</p>
        <label class="check"><input type="checkbox" bind:checked={keySaved} /> Guardei a chave num lugar seguro, fora deste servidor</label>
        <div class="modal-actions">
          <button class="btn-primary" onclick={closeKey}>Fechar</button>
        </div>
      </div>
    {/if}
  </Modal>
{/if}

<style>
  .end > * { margin-left: 6px; }
  .ok { font-size: 13px; color: var(--lime); }
  button:disabled { opacity: .6; }
  .card { padding: 18px 22px; display: flex; flex-direction: column; gap: 12px; margin-bottom: 16px; }
  .card .label { color: var(--lavender-2); }
  .label.dim { color: var(--lavender-3); }
  .row { display: flex; align-items: flex-end; gap: 18px; flex-wrap: wrap; }
  .grow { flex: 1 1 320px; display: flex; flex-direction: column; gap: 6px; }
  .time { display: flex; flex-direction: column; gap: 6px; }
  .time.off { opacity: .5; }
  .time select { width: 150px; }
  .keep { display: flex; gap: 18px; flex-wrap: wrap; }
  .hint .error { display: block; margin-top: 6px; }
  .switch, .check { display: inline-flex; align-items: center; gap: 8px; font-size: 14px; color: var(--soft); cursor: pointer; height: 36px; }
  .check { height: auto; line-height: 20px; }
  .facts { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .link { background: none; border: 0; padding: 0; color: var(--lavender); font-size: 13px; text-decoration: underline; text-underline-offset: 3px; cursor: pointer; }
  .warn-edge { margin-bottom: 16px; background: linear-gradient(135deg, rgba(230,255,0,.55), rgba(230,255,0,.08) 40%, rgba(230,255,0,.4)); }
  .warn { display: flex; align-items: center; gap: 14px; padding: 14px 18px; flex-wrap: wrap; }
  .warn p { flex: 1 1 300px; font-size: 14px; line-height: 21px; color: var(--soft); }
  .warn strong { color: var(--lime); font-weight: 600; }
  .kind { font-size: 14px; color: var(--soft); }
  .badge.plain { margin-left: 8px; }
  .key { user-select: all; font-size: 13px; }
  .copy { display: flex; align-items: center; gap: 12px; }
  .hint code { color: var(--soft); font-size: 12px; }
</style>
