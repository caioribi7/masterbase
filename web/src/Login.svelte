<script>
  import { untrack } from 'svelte';
  import { api } from './api.js';
  import Glyph from './Glyph.svelte';
  import { getPasskey, passkeyError, passkeySupported } from './webauthn.js';

  // token: link de criar senha (#/senha/...). magic: link de entrar (#/link/...).
  let { onDone, token = null, magic = null } = $props();
  const link = untrack(() => token); // o link não muda enquanto a tela está aberta
  const magicLink = untrack(() => magic);

  let mode = $state(link ? 'reset' : magicLink ? 'magic-use' : 'login'); // login | signup | reset | forgot | magic | magic-use | code
  let forgot = $state(false);
  let sent = $state(false);
  let oauth = $state([]); // provedores ligados no servidor
  let mail = $state(false); // servidor manda e-mail
  let passkey = $state(false); // servidor aceita passkey
  let ticket = $state(''); // senha conferida, falta o código do app
  let code = $state('');
  let name = $state('');
  let email = $state('');
  let password = $state('');
  let showPw = $state(false);
  let busy = $state(false);
  let error = $state('');
  let signupOpen = $state(false);
  let instance = $state('');
  let target = $state(null); // { name, email } do link de criar senha
  let linkError = $state('');

  // Sem nenhuma conta, a tela abre no cadastro: a primeira pessoa administra a instância.
  api('/setup').then((r) => { instance = r.name; signupOpen = r.signup; oauth = r.oauth ?? []; mail = r.mail; passkey = r.passkey && passkeySupported(); if (r.signup && !link) mode = 'signup'; }).catch(() => {});

  // Volta do Google/GitHub com erro: #/entrar?erro=...
  const oauthErrors = {
    'sem-conta': 'Esse e-mail não tem conta aqui. Peça acesso a um administrador.',
    'sem-email': 'A conta não tem e-mail verificado.',
    cancelado: 'O login foi cancelado.',
    desligado: 'Esse jeito de entrar não está ligado neste servidor.',
    falhou: 'Não deu para entrar. Tente de novo.',
  };
  const back = location.hash.match(/erro=([a-z-]+)/);
  if (back && !link) {
    error = oauthErrors[back[1]] ?? oauthErrors.falhou;
    history.replaceState(null, '', '#/');
  }
  // Google/GitHub conferiu, falta o código: #/entrar?fator=TICKET
  const factor = location.hash.match(/fator=([0-9a-f]+)/);
  if (factor && !link) {
    ticket = factor[1];
    mode = 'code';
    history.replaceState(null, '', '#/');
  }

  // Resposta de um passo de login: ou entrou, ou falta o código do app.
  function next(r) {
    if (r?.second_factor) {
      ticket = r.ticket;
      code = '';
      mode = 'code';
      return;
    }
    if (magicLink || link) location.hash = '#/inicio';
    onDone();
  }

  // Link de entrar: usa assim que a tela abre.
  if (magicLink) {
    api('/magic/use', { method: 'POST', body: { token: magicLink } }).then(next).catch((e) => (linkError = e.message));
  }

  async function withPasskey() {
    busy = true;
    error = '';
    try {
      const opts = await api('/passkeys/login/options', { method: 'POST' });
      const answer = await getPasskey(opts);
      next(await api('/passkeys/login', { method: 'POST', body: answer }));
    } catch (e) {
      error = e.status ? e.message : passkeyError(e);
    } finally {
      busy = false;
    }
  }
  const providers = { google: 'Google', github: 'GitHub' };
  if (link) api('/reset/check', { method: 'POST', body: { token: link } }).then((r) => (target = r)).catch((e) => (linkError = e.message));

  const titles = { login: 'Entrar no Masterbase', signup: 'Criar conta', reset: 'Criar senha', forgot: 'Esqueci a senha', magic: 'Entrar com link', 'magic-use': 'Entrando…', code: 'Código de verificação' };
  const to = (m) => { mode = m; error = ''; sent = false; };

  async function submit(e) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      if (mode === 'forgot' || mode === 'magic') {
        await api(mode === 'forgot' ? '/forgot' : '/magic', { method: 'POST', body: { email } });
        sent = true;
        return;
      }
      if (mode === 'code') return next(await api('/login/2fa', { method: 'POST', body: { ticket, code } }));
      if (mode === 'reset') return next(await api('/reset', { method: 'POST', body: { token: link, password } }));
      next(await api(`/${mode}`, { method: 'POST', body: mode === 'signup' ? { name, email, password } : { email, password } }));
    } catch (err) {
      error = err.message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="page">
  <header><Glyph size={26} /><span class="brand wordmark">{instance || 'Masterbase'}</span></header>

  <main>
    <div class="intro">
      <Glyph size={68} glow />
      <h1 class="brand">{titles[mode]}</h1>
      <p>
        {#if mode === 'login'}Use o e-mail e a senha da sua conta.
        {:else if mode === 'signup'}Primeiro acesso. Essa conta vai administrar o Masterbase.
        {:else if mode === 'forgot'}Mandamos um link para você criar uma senha nova.
        {:else if mode === 'magic'}Mandamos um link que entra direto, sem senha.
        {:else if mode === 'code'}Abra o app autenticador e digite o código de 6 dígitos.
        {:else if mode === 'magic-use'}{linkError ? '' : 'Conferindo o link…'}
        {:else if target}Para <strong>{target.name}</strong> · {target.email}
        {:else if !linkError}Conferindo o link…{/if}
      </p>
    </div>

    {#if (mode === 'reset' || mode === 'magic-use') && linkError}
      <div class="edge"><div class="card">
        <p class="error" role="alert">{linkError}</p>
        <a class="glass btn" href="#/inicio">Ir para o login</a>
      </div></div>
    {:else}
    <div class="edge">
      <form class="card" onsubmit={submit}>
        {#if mode === 'signup'}
          <label>
            <span class="label">Nome</span>
            <input class="field" bind:value={name} autocomplete="name" required />
          </label>
        {/if}

        {#if mode === 'code'}
        <label>
          <span class="label">Código</span>
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field code" bind:value={code} inputmode="numeric" autocomplete="one-time-code" required autofocus />
        </label>
        <p class="hint">Perdeu o celular? Use um dos códigos de recuperação no lugar.</p>
        {:else if mode !== 'reset'}
        <label>
          <span class="label">E-mail</span>
          <input class="field" type="email" bind:value={email} autocomplete="email" required />
        </label>
        {:else if target}
          <!-- Para o gerenciador de senhas saber de qual conta é a senha nova. -->
          <input type="email" value={target.email} autocomplete="username" hidden readonly />
        {/if}

        {#if mode !== 'forgot' && mode !== 'magic' && mode !== 'code'}
        <label>
          <span class="row"><span class="label">{mode === 'reset' ? 'Senha nova (10 ou mais caracteres)' : 'Senha'}</span>{#if mode === 'login'}<button type="button" class="link" onclick={() => (mail ? to('forgot') : (forgot = !forgot))}>Esqueci a senha</button>{/if}</span>
          <span class="pw">
            <input class="field" type={showPw ? 'text' : 'password'} bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength={mode === 'login' ? undefined : 10} required />
            <button type="button" class="mono show" onclick={() => (showPw = !showPw)}>{showPw ? 'OCULTAR' : 'MOSTRAR'}</button>
          </span>
        </label>
        {/if}

        {#if forgot && mode === 'login'}<p class="hint">Peça a um administrador o link de senha nova (página Acesso). Se você é o único administrador, rode <code>masterbase senha SEU-EMAIL</code> no servidor.</p>{/if}
        {#if error}<p class="error" role="alert">{error}</p>{/if}

        {#if sent}<p class="ok" role="status">Se esse e-mail tiver conta, o link chega em instantes. Olhe também o spam.</p>{/if}

        <button class="btn-primary submit" disabled={busy || (mode === 'reset' && !target)}>{({ login: 'Entrar', signup: 'Criar conta', reset: 'Salvar senha e entrar', forgot: sent ? 'Mandar de novo' : 'Mandar link', magic: sent ? 'Mandar de novo' : 'Mandar link', code: 'Confirmar' })[mode]}</button>

        {#if mode === 'login' && (passkey || mail)}
          <div class="alt">
            {#if passkey}<button type="button" class="glass btn provider" onclick={withPasskey} disabled={busy}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="9" cy="8" r="4"/><path d="M3 20a6 6 0 0 1 9.5-4.9"/><circle cx="17.5" cy="15.5" r="2.5"/><path d="M17.5 18v3.5M17.5 20.5h1.5"/></svg>
              Passkey</button>{/if}
            {#if mail}<button type="button" class="glass btn provider" onclick={() => to('magic')}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="5" width="18" height="14" rx="2"/><path d="m3 7 9 6 9-6"/></svg>
              Link por e-mail</button>{/if}
          </div>
        {/if}

        {#if mode === 'login' && oauth.length}
          <div class="or"><span>ou</span></div>
          <div class="providers">
            {#each oauth as p (p)}
              <a class="glass btn provider" href="/api/oauth/{p}" aria-label="Entrar com {providers[p]}">
                {#if p === 'google'}
                  <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M21.6 12.23c0-.68-.06-1.36-.18-2.02H12v3.83h5.4a4.6 4.6 0 0 1-2 3.02v2.5h3.24c1.9-1.75 2.96-4.33 2.96-7.33ZM12 22c2.7 0 4.98-.9 6.64-2.43l-3.24-2.5c-.9.6-2.05.95-3.4.95-2.6 0-4.82-1.76-5.6-4.12H3.04v2.58A10 10 0 0 0 12 22Zm-5.6-8.1a6 6 0 0 1 0-3.8V7.52H3.04a10 10 0 0 0 0 8.96L6.4 13.9ZM12 5.98c1.47 0 2.8.5 3.84 1.5l2.87-2.87A9.6 9.6 0 0 0 12 2a10 10 0 0 0-8.96 5.52L6.4 10.1C7.18 7.74 9.4 5.98 12 5.98Z"/></svg>
                {:else}
                  <svg width="18" height="18" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.9-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.52 2.34 1.08 2.91.83.09-.65.35-1.09.63-1.34-2.22-.25-4.55-1.11-4.55-4.94 0-1.1.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.02a9.58 9.58 0 0 1 5 0c1.91-1.3 2.75-1.02 2.75-1.02.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48A10 10 0 0 0 12 2Z"/></svg>
                {/if}
                {providers[p]}
              </a>
            {/each}
          </div>
        {/if}

      </form>
    </div>
    {/if}

    {#if mode === 'forgot' || mode === 'magic' || mode === 'code'}
    <p class="switch">{mode === 'forgot' ? 'Lembrou?' : 'Mudou de ideia?'} <button type="button" onclick={() => to('login')}>Voltar para o login</button></p>
    {:else if signupOpen && mode !== 'reset'}
    <p class="switch">
      {#if mode === 'login'}
        Ainda não tem conta? <button type="button" onclick={() => to('signup')}>Criar conta</button>
      {:else}
        Já tem conta? <button type="button" onclick={() => to('login')}>Entrar</button>
      {/if}
    </p>
    {/if}
  </main>

  <footer>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="11" width="16" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
    Código aberto. Seus dados ficam no seu servidor.
  </footer>
</div>

<style>
  .page {
    min-height: 100vh; display: flex; flex-direction: column;
    background: radial-gradient(ellipse 60% 55% at 50% 42%, rgba(154,160,224,.16) 0%, rgba(18,19,58,.35) 45%, rgba(10,11,31,0) 75%), var(--void);
  }
  header { height: 80px; padding: 0 48px; display: flex; align-items: center; gap: 10px; }
  .wordmark { font-size: 20px; }
  main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 24px; padding: 16px; }
  .intro { display: flex; flex-direction: column; align-items: center; gap: 14px; text-align: center; }
  h1 { font-size: 34px; line-height: 40px; }
  .intro p { color: var(--lavender); font-size: 15px; }

  .edge { width: min(456px, 100%); padding: 1px; border-radius: 24px; background: var(--edge); }
  .card {
    border-radius: 23px; padding: 36px; display: flex; flex-direction: column; gap: 18px;
    background: linear-gradient(180deg, #1A1C4C 0%, #12133A 100%); box-shadow: var(--inner);
  }
  label { display: flex; flex-direction: column; gap: 8px; }
  .row { display: flex; justify-content: space-between; align-items: center; }
  .link { font-size: 13px; color: var(--soft); background: none; border: 0; padding: 0; }
  .link:hover { color: var(--text); }
  .hint { font-size: 13px; line-height: 20px; color: var(--lavender); }
  .hint code { font-family: 'Martian Mono', monospace; font-size: 12px; color: var(--soft); }
  .intro strong { color: var(--text); font-weight: 600; }
  .card .btn { align-self: flex-start; }

  .pw { position: relative; display: block; }
  .pw .field { padding-right: 96px; }
  .show { position: absolute; right: 14px; top: 50%; transform: translateY(-50%); background: none; border: 0; font-size: 11px; color: var(--lavender); }
  .pw input[type='password'] { letter-spacing: .2em; }

  .submit { height: 48px; margin-top: 4px; font-size: 15px; }
  .error { color: var(--err); font-size: 13px; }
  .ok { color: var(--soft); font-size: 13px; line-height: 20px; }
  .or { display: flex; align-items: center; gap: 12px; font-size: 12px; color: var(--lavender); }
  .or::before, .or::after { content: ''; flex: 1; height: 1px; background: rgba(154,160,224,.18); }
  .providers { display: flex; gap: 10px; }
  .alt { display: flex; gap: 10px; }
  .code { font-family: 'Martian Mono', monospace; font-size: 22px; letter-spacing: .3em; text-align: center; }
  .provider { flex: 1; height: 44px; justify-content: center; gap: 10px; white-space: nowrap; }

  .switch { font-size: 14px; color: var(--lavender); }
  .switch button { background: none; border: 0; color: var(--text); font-weight: 600; padding: 0; }
  footer { height: 64px; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 13px; color: var(--lavender); }

  /* Notebook com tela baixa: tudo cabe sem rolar. */
  @media (max-height: 760px) {
    header { height: 56px; }
    footer { height: 48px; }
    main { gap: 16px; padding: 8px 16px; }
    .intro { gap: 8px; }
    .intro :global(svg) { width: 44px; height: 44px; }
    h1 { font-size: 28px; line-height: 34px; }
    .card { padding: 28px; gap: 14px; }
  }

  @media (max-width: 520px) {
    header { padding: 0 16px; }
    .card { padding: 24px; }
    h1 { font-size: 28px; }
  }
</style>
