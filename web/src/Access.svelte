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
  let modal = $state(null); // 'new' | 'password' | { reset: user }
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
                  <button class="glass btn small" onclick={() => open({ reset: u }, { password: '' })}>Nova senha</button>
                  <button class="glass btn small" class:btn-danger={u.active} onclick={() => change(u, { active: !u.active })}>{u.active ? 'Desativar' : 'Reativar'}</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div></div>
    <p class="hint">Conta desativada não entra mais, mas continua no histórico da Atividade.</p>
  {:else}
    <p class="hint">Seu papel é <strong>{roleLabel(session.user.role)}</strong>. Só administradores veem e mudam a lista de pessoas.</p>
  {/if}
</section>

{#if modal === 'new'}
  <Modal title="Nova pessoa" onClose={() => (modal = null)} width={480}>
    <form class="form" onsubmit={(e) => submit(e, () => api('/users', { method: 'POST', body: form }), `${form.name} já pode entrar.`)}>
      <label class="stack"><span class="label">Nome</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={form.name} required autofocus /></label>
      <label class="stack"><span class="label">E-mail</span><input class="field" type="email" bind:value={form.email} required autocomplete="off" /></label>
      <label class="stack"><span class="label">Senha inicial</span><input class="field" bind:value={form.password} minlength="10" required autocomplete="off" /></label>
      <label class="stack"><span class="label">Papel</span>
        <select class="field" bind:value={form.role}>{#each roles as [v, l]}<option value={v}>{l}</option>{/each}</select></label>
      <p class="hint">Passe a senha para a pessoa por um canal seguro. Ela pode trocar depois, aqui mesmo.</p>
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
{:else if modal?.reset}
  <Modal title="Nova senha para {modal.reset.name}" onClose={() => (modal = null)} width={440}>
    <form class="form" onsubmit={(e) => submit(e, () => api(`/users/${modal.reset.id}`, { method: 'PATCH', body: { password: form.password } }), 'Senha trocada. As sessões dessa pessoa foram encerradas.')}>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field" bind:value={form.password} minlength="10" required autocomplete="off" aria-label="Senha nova" autofocus />
      <p class="hint">A pessoa sai de todos os aparelhos e entra de novo com essa senha.</p>
      {#if formError}<p class="error" role="alert">{formError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (modal = null)}>Cancelar</button>
        <button class="btn-primary" disabled={busy}>Salvar senha</button>
      </div>
    </form>
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
  @media (max-width: 1100px) { .roles { grid-template-columns: minmax(0, 1fr); } }
</style>
