<script>
  import { untrack } from 'svelte';
  import { api } from './api.js';
  import Glyph from './Glyph.svelte';

  let { onDone, token = null } = $props();
  const link = untrack(() => token); // o link não muda enquanto a tela está aberta

  let mode = $state(link ? 'reset' : 'login'); // 'login' | 'signup' | 'reset'
  let forgot = $state(false);
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
  api('/setup').then((r) => { instance = r.name; signupOpen = r.signup; if (r.signup && !link) mode = 'signup'; }).catch(() => {});
  if (link) api('/reset/check', { method: 'POST', body: { token: link } }).then((r) => (target = r)).catch((e) => (linkError = e.message));

  const titles = { login: 'Entrar no Masterbase', signup: 'Criar conta', reset: 'Criar senha' };

  async function submit(e) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      if (mode === 'reset') await api('/reset', { method: 'POST', body: { token: link, password } });
      else await api(`/${mode}`, { method: 'POST', body: mode === 'signup' ? { name, email, password } : { email, password } });
      onDone();
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
        {:else if target}Para <strong>{target.name}</strong> · {target.email}
        {:else if !linkError}Conferindo o link…{/if}
      </p>
    </div>

    {#if mode === 'reset' && linkError}
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

        {#if mode !== 'reset'}
        <label>
          <span class="label">E-mail</span>
          <input class="field" type="email" bind:value={email} autocomplete="email" required />
        </label>
        {:else if target}
          <!-- Para o gerenciador de senhas saber de qual conta é a senha nova. -->
          <input type="email" value={target.email} autocomplete="username" hidden readonly />
        {/if}

        <label>
          <span class="row"><span class="label">{mode === 'reset' ? 'Senha nova (10 ou mais caracteres)' : 'Senha'}</span>{#if mode === 'login'}<button type="button" class="link" onclick={() => (forgot = !forgot)}>Esqueci a senha</button>{/if}</span>
          <span class="pw">
            <input class="field" type={showPw ? 'text' : 'password'} bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength={mode === 'login' ? undefined : 10} required />
            <button type="button" class="mono show" onclick={() => (showPw = !showPw)}>{showPw ? 'OCULTAR' : 'MOSTRAR'}</button>
          </span>
        </label>

        {#if forgot && mode === 'login'}<p class="hint">Peça a um administrador o link de senha nova (página Acesso). Se você é o único administrador, rode <code>masterbase senha SEU-EMAIL</code> no servidor.</p>{/if}
        {#if error}<p class="error" role="alert">{error}</p>{/if}

        <button class="btn-primary submit" disabled={busy || (mode === 'reset' && !target)}>{mode === 'login' ? 'Entrar' : mode === 'signup' ? 'Criar conta' : 'Salvar senha e entrar'}</button>

      </form>
    </div>
    {/if}

    {#if signupOpen && mode !== 'reset'}
    <p class="switch">
      {#if mode === 'login'}
        Ainda não tem conta? <button type="button" onclick={() => ((mode = 'signup'), (error = ''))}>Criar conta</button>
      {:else}
        Já tem conta? <button type="button" onclick={() => ((mode = 'login'), (error = ''))}>Entrar</button>
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
  .hint code { font-family: var(--mono); font-size: 12px; color: var(--soft); }
  .intro strong { color: var(--text); font-weight: 600; }
  .card .btn { align-self: flex-start; }

  .pw { position: relative; display: block; }
  .pw .field { padding-right: 96px; }
  .show { position: absolute; right: 14px; top: 50%; transform: translateY(-50%); background: none; border: 0; font-size: 11px; color: var(--lavender); }
  .pw input[type='password'] { letter-spacing: .2em; }

  .submit { height: 48px; margin-top: 4px; font-size: 15px; }
  .error { color: var(--err); font-size: 13px; }

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
