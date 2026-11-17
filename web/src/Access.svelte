<script>
  import { api } from './api.js';
  import { roleLabel } from './lib.js';
  import { isAdmin, session } from './session.svelte.js';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';

  let { onAuthLost } = $props();

  let users = $state([]);
  let me = $state(null);
  let error = $state('');
  let modal = $state(null); // 'new' | 'password' | { link: { name, url, invite } }
  let copied = $state(false);
  let formError = $state('');
  let busy = $state(false);
  let form = $state({});
  let done = $state('');

  const roles = [['admin', 'Administrador'], ['editor', 'Editor'], ['leitor', 'Leitor']];
  const fail = (e) => (e.status === 401 ? onAuthLost() : (error = e.message));

  async function load() {
    if (!isAdmin()) return;
    try {
      const r = await api('/users');
      users = r.users;
      me = r.me;
      error = '';
    } catch (e) {
      fail(e);
    }
  }
  load();

  function open(what, initial = {}) {
    modal = what;
    form = initial;
    formError = '';
    busy = false;
  }

  async function submit(e, fn, message) {
    e.preventDefault();
    busy = true;
    formError = '';
    try {
      await fn();
      modal = null;
      done = message;
      await load();
    } catch (err) {
      busy = false;
      err.status === 401 ? onAuthLost() : (formError = err.message);
    }
  }

  // Link de criar senha. O servidor devolve só o token; o endereço é o deste painel.
  const linkFor = (token) => `${location.origin}/#/senha/${token}`;

  async function newPerson(e) {
    e.preventDefault();
    busy = true;
    formError = '';
    try {
      const r = await api('/users', { method: 'POST', body: form });
      await load();
      if (r.token) { copied = false; modal = { link: { name: form.name, url: linkFor(r.token), invite: true } }; }
      else { modal = null; done = `${form.name} já pode entrar.`; }
    } catch (err) {
      busy = false;
      err.status === 401 ? onAuthLost() : (formError = err.message);
    }
  }

  async function passwordLink(u) {
    try {
      const r = await api(`/users/${u.id}/link`, { method: 'POST' });
      copied = false;
      modal = { link: { name: u.name, url: linkFor(r.token), invite: false } };
    } catch (e) { fail(e); }
  }

  async function copyLink() {
    try { await navigator.clipboard.writeText(modal.link.url); copied = true; } catch { copied = false; }
  }

  const change = (u, body) => api(`/users/${u.id}`, { method: 'PATCH', body }).then(load).catch(fail);
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Acesso</h1>
      <p>Quem entra neste Masterbase e o que cada pessoa pode fazer.</p>
    </div>
    <div class="actions">
      <button class="glass btn" onclick={() => open('password', { current: '', next: '' })}><Icon name="key" size={15} stroke="#C9CCEB" /> Trocar minha senha</button>
      {#if isAdmin()}
        <button class="btn-primary" onclick={() => open('new', { name: '', email: '', password: '', role: 'editor' })}><Icon name="plus" size={14} stroke="#12133A" weight={2.6} /> Nova pessoa</button>
      {/if}
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if done}<p class="ok" role="status">{done}</p>{/if}

  <div class="roles">
    {#each [['Administrador', 'Tudo: estrutura, pessoas, chaves, backups e editor SQL.'], ['Editor', 'Vê os dados e muda as linhas. Não mexe na estrutura.'], ['Leitor', 'Só vê os dados e roda consultas.']] as [t, d]}
      <div class="panel-edge"><div class="panel role"><span class="label">{t}</span><p>{d}</p></div></div>
    {/each}
  </div>

  {#if isAdmin()}
    <div class="panel-edge"><div class="panel scroll-x">
      <table class="simple">
        <thead><tr><th>Pessoa</th><th>E-mail</th><th>Papel</th><th>Situação</th><th></th></tr></thead>
        <tbody>
          {#each users as u (u.id)}
            <tr class:off={!u.active}>
              <td><strong>{u.name}</strong>{#if u.id === me} <span class="badge">você</span>{/if}</td>
              <td>{u.email}</td>
              <td>
                {#if u.id === me}<span class="badge lime">Administrador</span>
                {:else}
                  <select class="field small pick" value={u.role} onchange={(e) => change(u, { role: e.currentTarget.value })} aria-label="Papel de {u.name}">
                    {#each roles as [v, l]}<option value={v}>{l}</option>{/each}
                  </select>
                {/if}
              </td>
              <td>{u.active ? 'ativa' : 'desativada'}</td>
              <td class="end">
                {#if u.id !== me}
                  {#if u.active}<button class="glass btn small" onclick={() => passwordLink(u)}>Link de senha</button>{/if}
                  <button class="glass btn small" class:btn-danger={u.active} onclick={() => change(u, { active: !u.active })}>{u.active ? 'Desativar' : 'Reativar'}</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div></div>
    <p class="hint">Esqueceu a senha? Gere um <strong>link de senha</strong> e mande para a pessoa: vale 24 horas, uma vez só, e encerra as sessões abertas dela. Conta desativada não entra mais, mas continua no histórico da Atividade.</p>
  {:else}
    <p class="hint">Seu papel é <strong>{roleLabel(session.user.role)}</strong>. Só administradores veem e mudam a lista de pessoas.</p>
  {/if}
</section>

{#if modal === 'new'}
  <Modal title="Nova pessoa" onClose={() => (modal = null)} width={480}>
    <form class="form" onsubmit={newPerson}>
      <label class="stack"><span class="label">Nome</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={form.name} required autofocus /></label>
      <label class="stack"><span class="label">E-mail</span><input class="field" type="email" bind:value={form.email} required autocomplete="off" /></label>
      <label class="stack"><span class="label">Senha inicial (opcional)</span><input class="field" bind:value={form.password} minlength="10" autocomplete="off" /></label>
      <label class="stack"><span class="label">Papel</span>
        <select class="field" bind:value={form.role}>{#each roles as [v, l]}<option value={v}>{l}</option>{/each}</select></label>
      <p class="hint">Deixe a senha em branco para gerar um link de convite: a pessoa abre e cria a própria senha.</p>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (modal = null)}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Dar acesso</button>
      </div>
    </form>
  </Modal>
{:else if modal === 'password'}
  <Modal title="Trocar minha senha" onClose={() => (modal = null)} width={440}>
    <form class="form" onsubmit={(e) => submit(e, () => api('/password', { method: 'POST', body: { current: form.current, new: form.next } }), 'Senha trocada.')}>
      <label class="stack"><span class="label">Senha atual</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" type="password" bind:value={form.current} required autocomplete="current-password" autofocus /></label>
      <label class="stack"><span class="label">Senha nova</span><input class="field" type="password" bind:value={form.next} minlength="10" required autocomplete="new-password" /></label>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (modal = null)}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Trocar senha</button>
      </div>
    </form>
  </Modal>
{:else if modal?.link}
  <Modal title={modal.link.invite ? `Convite para ${modal.link.name}` : `Link de senha para ${modal.link.name}`} onClose={() => (modal = null)} width={520}>
    <div class="form">
      <p class="hint">{modal.link.invite ? 'Mande este link para a pessoa criar a senha dela.' : 'Mande este link para a pessoa criar uma senha nova.'} Vale 24 horas e funciona uma vez só. Ele aparece só agora.</p>
      <code class="code linkbox">{modal.link.url}</code>
      {#if copied}<p class="ok" role="status">Link copiado.</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (modal = null)}>Fechar</button>
        <button type="button" class="btn-primary" onclick={copyLink}><Icon name="copy" size={15} stroke="#12133A" /> Copiar link</button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .roles { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
  .role { padding: 16px 20px; display: flex; flex-direction: column; gap: 8px; }
  .role .label { color: var(--lavender-2); }
  .role p { font-size: 14px; line-height: 21px; color: var(--soft); }
  .pick { width: 140px; }
  .off td { color: var(--lavender-3); }
  .end .btn { margin-left: 6px; }
  .ok { font-size: 13px; color: var(--lime); }
  .hint strong { color: var(--text); }
  .linkbox { display: block; padding: 12px 14px; overflow-wrap: anywhere; font-size: 12px; line-height: 1.6; }
  @media (max-width: 1100px) { .roles { grid-template-columns: minmax(0, 1fr); } }
</style>
