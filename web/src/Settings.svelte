<script>
  // Configurações: minha conta para todos; como este Masterbase roda, só para administradores.
  import { api } from './api.js';
  import { roleLabel } from './lib.js';
  import { isAdmin, session } from './session.svelte.js';
  import Icon from './Icon.svelte';

  let { onAuthLost, onProfile } = $props();
  const me = $derived(session.user);
  let name = $state(session.user.name);
  let pw = $state({ current: '', next: '' });
  let info = $state(null);
  let msg = $state({ name: '', pw: '', copy: '' });
  let err = $state({ name: '', pw: '', info: '' });
  let busy = $state('');

  const fail = (k) => (e) => (e.status === 401 ? onAuthLost() : (err[k] = e.message));

  if (isAdmin()) api('/settings').then((r) => (info = r)).catch(fail('info'));

  async function saveName(e) {
    e.preventDefault();
    busy = 'name'; err.name = ''; msg.name = '';
    try {
      await api('/me', { method: 'PATCH', body: { name } });
      msg.name = 'Nome salvo.';
      onProfile();
    } catch (x) { fail('name')(x); } finally { busy = ''; }
  }

  async function savePw(e) {
    e.preventDefault();
    busy = 'pw'; err.pw = ''; msg.pw = '';
    try {
      await api('/password', { method: 'POST', body: { current: pw.current, new: pw.next } });
      msg.pw = 'Senha trocada.';
      pw = { current: '', next: '' };
    } catch (x) { fail('pw')(x); } finally { busy = ''; }
  }

  const docUrl = `${location.origin}/llms.txt`;
  async function copyDoc() {
    try { await navigator.clipboard.writeText(docUrl); msg.copy = 'Endereço copiado.'; }
    catch { msg.copy = 'Copie o endereço acima.'; }
  }

  const rows = $derived(info ? [
    ['Versão do Masterbase', info.version],
    ['Banco', `${info.database} em ${info.host || '?'}`],
    ['Postgres', `${info.postgres.split(' ')[0]} · ${info.size}`],
    ['Endereço do painel', info.addr],
    ['Quem acessa', info.access === 'local' ? 'Só este computador (ou um proxy com HTTPS)' : 'Outras máquinas da rede'],
    ['Login', info.https_cookie ? 'Exige HTTPS fora deste computador' : 'Aceita sem HTTPS (rede local)'],
    ['Pasta de backups', info.backup_dir],
    ['Arquivo de configuração', info.config],
  ] : []);
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Configurações</h1>
      <p>Sua conta e como este Masterbase está rodando.</p>
    </div>
  </div>

  <div class="grid">
    <div class="panel-edge"><div class="panel card">
      <span class="label">Minha conta</span>
      <form class="form" onsubmit={saveName}>
        <label class="stack"><span class="label dim">Nome</span><input class="field" bind:value={name} maxlength="80" required /></label>
        <div class="kv"><span>E-mail</span><strong>{me.email}</strong></div>
        <div class="kv"><span>Papel</span><span class="badge" class:lime={me.role === 'admin'}>{roleLabel(me.role)}</span></div>
        {#if err.name}<p class="error" role="alert">{err.name}</p>{/if}
        {#if msg.name}<p class="ok" role="status">{msg.name}</p>{/if}
        <div><button class="glass btn" disabled={busy === 'name' || name.trim() === me.name}>Salvar nome</button></div>
      </form>
    </div></div>

    <div class="panel-edge"><div class="panel card">
      <span class="label">Trocar senha</span>
      <form class="form" onsubmit={savePw}>
        <label class="stack"><span class="label dim">Senha atual</span><input class="field" type="password" bind:value={pw.current} required autocomplete="current-password" /></label>
        <label class="stack"><span class="label dim">Senha nova (10 ou mais caracteres)</span><input class="field" type="password" bind:value={pw.next} minlength="10" required autocomplete="new-password" /></label>
        {#if err.pw}<p class="error" role="alert">{err.pw}</p>{/if}
        {#if msg.pw}<p class="ok" role="status">{msg.pw}</p>{/if}
        <div><button class="glass btn" disabled={busy === 'pw'}>Trocar senha</button></div>
      </form>
    </div></div>
  </div>

  <div class="panel-edge"><div class="panel card">
    <span class="label">Integrar com outro sistema</span>
    <p class="hint">Mande este endereço para uma IA (ChatGPT, Claude, Copilot) e peça o código da integração. Ele explica a API inteira. Depois crie uma chave em API e chaves.</p>
    <div class="copy">
      <code class="code grow">{docUrl}</code>
      <button class="glass btn" onclick={copyDoc}><Icon name="copy" size={15} stroke="#C9CCEB" /> Copiar</button>
      <a class="glass btn" href="/llms.txt" target="_blank" rel="noopener">Abrir</a>
    </div>
    {#if msg.copy}<p class="ok" role="status">{msg.copy}</p>{/if}
  </div></div>

  {#if isAdmin()}
    <div class="panel-edge"><div class="panel card">
      <span class="label">Este Masterbase</span>
      {#if err.info}<p class="error" role="alert">{err.info}</p>{/if}
      {#each rows as [k, v]}
        <div class="kv"><span>{k}</span><strong class="mono small">{v}</strong></div>
      {/each}
      <p class="hint">Para mudar porta, banco ou quem acessa, rode <code class="mono">masterbase configurar</code> no servidor e reinicie o Masterbase.</p>
      <p class="hint">Para acompanhar pelo terminal do servidor: o próprio <code class="mono">masterbase</code> mostra uma linha por pedido e por mudança (quem, o quê, comandos do editor SQL). Rodando como serviço, veja com <code class="mono">journalctl -u masterbase -f</code>.</p>
    </div></div>
  {/if}
</section>

<style>
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
  .card { padding: 20px 22px; display: flex; flex-direction: column; gap: 14px; }
  .card > .label { color: var(--lavender-2); }
  .label.dim { color: var(--lavender-3); }
  .kv { min-height: 36px; display: flex; align-items: center; justify-content: space-between; gap: 16px; font-size: 14px; border-bottom: 1px solid #1A1C46; }
  .kv span:first-child { color: var(--lavender); flex-shrink: 0; }
  .kv strong { font-weight: 500; text-align: right; overflow-wrap: anywhere; }
  .kv .small { font-size: 12px; color: var(--soft); }
  .ok { font-size: 13px; color: var(--lime); }
  .copy { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .copy .code { padding: 10px 14px; min-width: 0; }
  .hint code { color: var(--soft); font-size: 12px; }
  @media (max-width: 900px) { .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
