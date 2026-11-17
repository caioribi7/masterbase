<script>
  // Configurações: minha conta para todos; como este Masterbase roda, só para administradores.
  import { api } from './api.js';
  import { ago, roleLabel } from './lib.js';
  import { loadPrefs, savePrefs, sizes } from './prefs.js';
  import { isAdmin, session } from './session.svelte.js';
  import Icon from './Icon.svelte';

  let { onAuthLost, onProfile, onLogout } = $props();
  const me = $derived(session.user);
  let name = $state(session.user.name);
  let pw = $state({ current: '', next: '' });
  let info = $state(null);
  let msg = $state({ name: '', pw: '', copy: '', sessions: '', inst: '' });
  let err = $state({ name: '', pw: '', info: '', sessions: '', inst: '' });
  let busy = $state('');

  const fail = (k) => (e) => (e.status === 401 ? onAuthLost() : (err[k] = e.message));

  let sessions = $state([]);
  let showAll = $state(false);
  let prefs = $state(loadPrefs());
  let inst = $state({ name: '', session_days: 7 });

  const loadSessions = () => api('/sessions').then((r) => (sessions = r)).catch(fail('sessions'));
  loadSessions();
  if (isAdmin()) api('/settings').then((r) => { info = r; inst = { name: r.name, session_days: r.session_days }; }).catch(fail('info'));

  // "Chrome no Windows" a partir do user-agent. Só para a pessoa reconhecer o aparelho.
  function device(ua) {
    if (!ua) return 'Aparelho sem nome';
    const b = /Edg\//.test(ua) ? 'Edge' : /OPR\//.test(ua) ? 'Opera' : /Firefox\//.test(ua) ? 'Firefox' : /Chrome\//.test(ua) ? 'Chrome' : /Safari\//.test(ua) ? 'Safari' : ua.split(/[ /]/)[0];
    const os = /iPhone|iPad/.test(ua) ? 'iPhone/iPad' : /Android/.test(ua) ? 'Android' : /Windows/.test(ua) ? 'Windows' : /Mac OS/.test(ua) ? 'Mac' : /Linux/.test(ua) ? 'Linux' : '';
    return os ? `${b} no ${os}` : b;
  }

  async function endSession(s) {
    try { await api(`/sessions/${s.id}`, { method: 'DELETE' }); await loadSessions(); } catch (x) { fail('sessions')(x); }
  }

  async function endOthers() {
    busy = 'sessions'; err.sessions = ''; msg.sessions = '';
    try {
      const r = await api('/sessions', { method: 'DELETE' });
      msg.sessions = r.ended ? `Saiu de ${r.ended} ${r.ended === 1 ? 'aparelho' : 'aparelhos'}.` : 'Não havia outros aparelhos.';
      await loadSessions();
    } catch (x) { fail('sessions')(x); } finally { busy = ''; }
  }

  async function logout() {
    await api('/logout', { method: 'POST' }).catch(() => {});
    onLogout();
  }

  function setPref(k, v) {
    prefs = { ...prefs, [k]: v };
    savePrefs(prefs);
  }

  async function saveInstance(e) {
    e.preventDefault();
    busy = 'inst'; err.inst = ''; msg.inst = '';
    try {
      await api('/settings', { method: 'PATCH', body: { name: inst.name, session_days: +inst.session_days } });
      msg.inst = 'Salvo. A duração nova vale para as próximas entradas.';
      info = { ...info, ...inst };
      onProfile();
    } catch (x) { fail('inst')(x); } finally { busy = ''; }
  }

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
      msg.pw = 'Senha trocada. Os outros aparelhos saíram da conta.';
      loadSessions();
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
    ['Editor SQL', info.sql_editor ? 'Ligado, só lê e muda linhas' : 'Desligado (falta MB_SQL_URL)'],
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

  <div class="grid">
    <div class="panel-edge"><div class="panel card">
      <div class="head-row">
        <span class="label">Onde você está conectado</span>
        <button class="glass btn small" onclick={logout}><Icon name="logout" size={14} stroke="#C9CCEB" /> Sair</button>
      </div>
      {#each sessions.slice(0, showAll ? undefined : 5) as s (s.id)}
        <div class="kv">
          <span class="dev">{device(s.device)}{#if s.current} <span class="badge lime">este</span>{/if}</span>
          {#if s.current}<strong class="small">agora</strong>
          {:else}
            <span class="sess-end"><strong class="small">{ago(s.last_seen_at)}</strong>
              <button class="glass btn small" onclick={() => endSession(s)} aria-label="Encerrar sessão {device(s.device)}">Encerrar</button></span>
          {/if}
        </div>
      {/each}
      {#if sessions.length > 5 && !showAll}<button class="link-btn" onclick={() => (showAll = true)}>Ver mais {sessions.length - 5}</button>{/if}
      {#if err.sessions}<p class="error" role="alert">{err.sessions}</p>{/if}
      {#if msg.sessions}<p class="ok" role="status">{msg.sessions}</p>{/if}
      {#if sessions.length > 1}<div><button class="glass btn" disabled={busy === 'sessions'} onclick={endOthers}>Sair dos outros aparelhos</button></div>{/if}
    </div></div>

    <div class="panel-edge"><div class="panel card">
      <span class="label">Leitura e conforto</span>
      <p class="hint">Vale só neste navegador.</p>
      <div class="stack">
        <span class="label dim" id="size-label">Tamanho do texto</span>
        <div class="seg" role="radiogroup" aria-labelledby="size-label">
          {#each sizes as [v, l]}
            <button role="radio" aria-checked={prefs.size === v} class:on={prefs.size === v} onclick={() => setPref('size', v)}>{l}</button>
          {/each}
        </div>
      </div>
      <label class="check-label"><input type="checkbox" checked={prefs.calm} onchange={(e) => setPref('calm', e.currentTarget.checked)} /> Menos movimento (sem animações)</label>
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

  {#if isAdmin() && info}
    <div class="panel-edge"><div class="panel card">
      <span class="label">Instância</span>
      <form class="form inst" onsubmit={saveInstance}>
        <label class="stack"><span class="label dim">Nome que aparece na tela</span>
          <input class="field" bind:value={inst.name} maxlength="60" placeholder={info.database} /></label>
        <label class="stack"><span class="label dim">Sessão dura</span>
          <select class="field" bind:value={inst.session_days}>
            {#each [1, 7, 14, 30, 90] as d}<option value={d}>{d === 1 ? '1 dia' : `${d} dias`}</option>{/each}
          </select></label>
        <div class="inst-save"><button class="glass btn" disabled={busy === 'inst' || (inst.name === info.name && +inst.session_days === info.session_days)}>Salvar</button></div>
      </form>
      {#if err.inst}<p class="error" role="alert">{err.inst}</p>{/if}
      {#if msg.inst}<p class="ok" role="status">{msg.inst}</p>{/if}
    </div></div>
  {/if}

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
  .link-btn { align-self: flex-start; background: none; border: 0; padding: 0; font-size: 13px; color: var(--lavender); text-decoration: underline; text-underline-offset: 3px; cursor: pointer; }
  .head-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .dev { color: var(--text) !important; display: inline-flex; align-items: center; gap: 8px; }
  .sess-end { display: inline-flex; align-items: center; gap: 10px; }
  .stack { display: flex; flex-direction: column; gap: 8px; }
  .seg { display: flex; flex-wrap: wrap; gap: 6px; }
  .seg button { height: 36px; padding: 0 14px; border-radius: 9px; border: 1px solid rgba(154,160,224,.3); background: rgba(154,160,224,.06); color: var(--soft); font-size: 13px; }
  .seg button.on { color: var(--text); border-color: rgba(154,160,224,.8); background: rgba(154,160,224,.2); box-shadow: inset 0 1px 0 rgba(255,255,255,.12); }
  .inst { display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr) auto; gap: 14px; align-items: end; }
  @media (max-width: 700px) { .inst { grid-template-columns: minmax(0, 1fr); } }
  @media (max-width: 900px) { .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
