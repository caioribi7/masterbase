<script>
  import { api } from './api.js';
  import Glyph from './Glyph.svelte';

  let { onDone } = $props();

  let mode = $state('login'); // 'login' | 'signup'
  let where = $state('hosted');
  let name = $state('');
  let email = $state('');
  let password = $state('');
  let showPw = $state(false);
  let busy = $state(false);
  let error = $state('');

  async function submit(e) {
    e.preventDefault();
    busy = true;
    error = '';
    try {
      const body = mode === 'signup' ? { name, email, password } : { email, password };
      await api(`/${mode}`, { method: 'POST', body });
      onDone();
    } catch (err) {
      error = err.message;
    } finally {
      busy = false;
    }
  }
</script>

<div class="page">
  <header><Glyph size={26} /><span class="brand wordmark">Masterbase</span></header>

  <main>
    <div class="intro">
      <Glyph size={68} glow />
      <h1 class="brand">{mode === 'login' ? 'Entrar no Masterbase' : 'Criar conta'}</h1>
      <p>{mode === 'login' ? 'Use o e-mail e a senha da sua conta.' : 'Leva menos de um minuto.'}</p>
    </div>

    <div class="edge">
      <form class="card" onsubmit={submit}>
        <div class="toggle" role="radiogroup" aria-label="Onde fica o banco">
          <button type="button" role="radio" aria-checked={where === 'hosted'} class:on={where === 'hosted'} onclick={() => (where = 'hosted')}>Hospedado</button>
          <button type="button" role="radio" aria-checked={where === 'self'} class:on={where === 'self'} onclick={() => (where = 'self')}>Meu servidor</button>
        </div>

        {#if mode === 'signup'}
          <label>
            <span class="label">Nome</span>
            <input class="field" bind:value={name} autocomplete="name" required />
          </label>
        {/if}

        <label>
          <span class="label">E-mail</span>
          <input class="field" type="email" bind:value={email} autocomplete="email" required />
        </label>

        <label>
          <span class="row"><span class="label">Senha</span>{#if mode === 'login'}<span class="link">Esqueci a senha</span>{/if}</span>
          <span class="pw">
            <input class="field" type={showPw ? 'text' : 'password'} bind:value={password} autocomplete={mode === 'login' ? 'current-password' : 'new-password'} minlength={mode === 'signup' ? 10 : undefined} required />
            <button type="button" class="mono show" onclick={() => (showPw = !showPw)}>{showPw ? 'OCULTAR' : 'MOSTRAR'}</button>
          </span>
        </label>

        {#if error}<p class="error" role="alert">{error}</p>{/if}

        <button class="btn-primary submit" disabled={busy}>{mode === 'login' ? 'Entrar' : 'Criar conta'}</button>

        {#if mode === 'login'}
          <div class="divider"><span class="mono">OU</span></div>
          <div class="alts">
            <button type="button" class="glass btn" disabled title="Em breve">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#C9CCEB" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="5" width="18" height="14" rx="3" /><path d="M3 7l9 6 9-6" /></svg>
              Link por e-mail
            </button>
            <button type="button" class="glass btn" disabled title="Em breve">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#C9CCEB" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="15" r="4" /><path d="M11 12l8-8M16 7l3 3M14 9l2 2" /></svg>
              Chave de acesso
            </button>
          </div>
        {/if}
      </form>
    </div>

    <p class="switch">
      {#if mode === 'login'}
        Ainda não tem conta? <button type="button" onclick={() => ((mode = 'signup'), (error = ''))}>Criar conta grátis</button>
      {:else}
        Já tem conta? <button type="button" onclick={() => ((mode = 'login'), (error = ''))}>Entrar</button>
      {/if}
    </p>
  </main>

  <footer>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#9AA0E0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="11" width="16" height="10" rx="2" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
    Seus dados ficam criptografados, no nosso servidor ou no seu.
  </footer>
</div>

<style>
  .page {
    min-height: 100vh; display: flex; flex-direction: column;
    background: radial-gradient(ellipse 60% 55% at 50% 42%, rgba(154,160,224,.16) 0%, rgba(18,19,58,.35) 45%, rgba(10,11,31,0) 75%), var(--void);
  }
  header { height: 80px; padding: 0 48px; display: flex; align-items: center; gap: 10px; }
  .wordmark { font-size: 20px; }
  main { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 28px; padding: 16px; }
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
  .link { font-size: 13px; color: var(--soft); }

  .toggle { height: 40px; padding: 4px; border-radius: 12px; background: #0C0D28; border: 1px solid #1E2152; box-shadow: inset 0 1px 3px rgba(0,0,0,.5); display: flex; gap: 4px; }
  .toggle button { flex: 1; border-radius: 9px; border: 1px solid transparent; background: none; font-size: 13px; color: var(--lavender); }
  .toggle .on {
    background: linear-gradient(180deg, #262A66 0%, #1A1C4C 100%); border-color: rgba(154,160,224,.35);
    box-shadow: inset 0 1px 0 rgba(255,255,255,.14), 0 0 14px rgba(154,160,224,.18); color: var(--text); font-weight: 600;
  }

  .pw { position: relative; display: block; }
  .pw .field { padding-right: 96px; }
  .show { position: absolute; right: 14px; top: 50%; transform: translateY(-50%); background: none; border: 0; font-size: 11px; color: var(--lavender); }
  .pw input[type='password'] { letter-spacing: .2em; }

  .submit { height: 48px; margin-top: 4px; font-size: 15px; }
  .error { color: var(--err); font-size: 13px; }
  .divider { display: flex; align-items: center; gap: 12px; font-size: 11px; color: var(--lavender-3); }
  .divider::before, .divider::after { content: ''; flex: 1; height: 1px; background: linear-gradient(90deg, transparent, #2A2E6A); }
  .divider::after { background: linear-gradient(90deg, #2A2E6A, transparent); }
  .alts { display: flex; gap: 10px; }
  .alts .btn { flex: 1; justify-content: center; height: 44px; color: var(--text); }
  .alts .btn:disabled { cursor: not-allowed; opacity: .75; }

  .switch { font-size: 14px; color: var(--lavender); }
  .switch button { background: none; border: 0; color: var(--text); font-weight: 600; padding: 0; }
  footer { height: 64px; display: flex; align-items: center; justify-content: center; gap: 8px; font-size: 13px; color: var(--lavender); }

  @media (max-width: 520px) {
    header { padding: 0 16px; }
    .card { padding: 24px; }
    h1 { font-size: 28px; }
    .alts { flex-direction: column; }
  }
</style>
