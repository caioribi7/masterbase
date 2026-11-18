<script>
  import { api, currentProject, openProject } from './api.js';
  import Modal from './Modal.svelte';
  import { href, roleLabel } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';

  let { user, page, crumb, onLogout, children } = $props();

  // Páginas com tela pronta têm rota; as outras aparecem, mas apagadas.
  const all = [
    ['Início', 'inicio'], ['Tabelas', 'tabelas'], ['Estrutura', 'estrutura'], ['Relações', 'relacoes'], ['Consultas', 'consultas'],
    ['Editor SQL', 'sql', true], ['Acesso', 'acesso'], ['API e chaves', 'chaves', true], ['Backups', 'backups', true], ['Atividade', 'atividade'],
    ['Configurações', 'configuracoes'],
  ];
  // Terceiro campo = só para quem administra.
  const nav = $derived(all.filter(([, , adminOnly]) => !adminOnly || user.role === 'admin'));
  const initials = $derived(user.name.split(' ').map((w) => w[0]).slice(0, 2).join('').toUpperCase());

  // Projetos: outros bancos Postgres que este painel abre.
  let projects = $state(null); // null = janela fechada
  let form = $state({ name: '', url: '', busy: false, error: '' });
  let removing = $state(null);
  const here = currentProject();

  async function showProjects() {
    form = { name: '', url: '', busy: false, error: '' };
    removing = null;
    try { projects = await api('/projects'); } catch (e) { projects = []; form.error = e.message; }
  }

  async function addProject(e) {
    e.preventDefault();
    form.busy = true;
    form.error = '';
    try {
      const r = await api('/projects', { method: 'POST', body: { name: form.name, url: form.url } });
      openProject(r.id);
    } catch (err) {
      form.busy = false;
      form.error = err.message;
    }
  }

  async function removeProject(p) {
    try {
      await api(`/projects/${p.id}`, { method: 'DELETE' });
      if (p.id === here) return openProject(0);
      await showProjects();
    } catch (err) {
      form.error = err.message;
    }
  }

  async function logout() {
    await api('/logout', { method: 'POST' }).catch(() => {});
    onLogout();
  }
</script>

<div class="shell">
  <aside class="side">
    <a class="logo" href={href('inicio')}><Glyph size={26} /><span class="brand">Masterbase</span></a>

    <button class="glass project" onclick={showProjects} title="Trocar de projeto" aria-label="Projeto {user.project}. Trocar de projeto">
      <span class="avatar-sq brand">{user.project.slice(0, 2).toUpperCase()}</span>
      <span class="grow"><strong>{user.project}</strong><small>Postgres {user.postgres.split(' ')[0]}</small></span>
      <svg class="chev" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m7 9 5-5 5 5M7 15l5 5 5-5"/></svg>
    </button>

    <nav>
      {#each nav as [label, route]}
        <a class="nav-item" class:active={page === route} aria-current={page === route ? 'page' : undefined} href={href(route)}>
          <Icon name={label} size={18} weight={1.8} />
          <span>{label}</span>
        </a>
      {/each}
    </nav>

    <div class="grow"></div>
    <div class="me">
      <span class="avatar">{initials}</span>
      <span class="grow"><strong>{user.name}</strong><small>{roleLabel(user.role)}</small></span>
      <button class="icon-btn" onclick={logout} title="Sair" aria-label="Sair"><Icon name="logout" /></button>
    </div>
  </aside>

  <div class="main">
    <div class="top">
      <span class="crumb">
        {#each crumb as part, i}
          {#if i}<i>/</i>{/if}
          {#if i === crumb.length - 1}<strong>{part}</strong>{:else}{part}{/if}
        {/each}
      </span>
    </div>
    <div class="body">{@render children()}</div>
  </div>
</div>

{#if projects}
  <Modal title="Projetos" onClose={() => (projects = null)} width={560}>
    <div class="plist">
      {#each projects as p (p.id)}
        <div class="prow" class:on={p.id === here}>
          <span class="avatar-sq brand">{p.name.slice(0, 2).toUpperCase()}</span>
          <span class="grow"><strong>{p.name}</strong><small>{p.main ? 'banco principal: contas, backups e atividade ficam aqui' : p.place}</small></span>
          {#if p.id === here}<span class="badge">aberto</span>
          {:else}<button class="glass btn small" onclick={() => openProject(p.id)}>Abrir</button>{/if}
          {#if !p.main && user.role === 'admin'}
            {#if removing === p.id}
              <button class="glass btn small btn-danger solid" onclick={() => removeProject(p)}>Tirar</button>
            {:else}
              <button class="icon-btn" title="Tirar do painel" aria-label="Tirar {p.name} do painel" onclick={() => (removing = p.id)}><Icon name="trash" size={15} /></button>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
    {#if removing}<p class="hint">Tirar só desconecta: o banco e os dados dele continuam onde estão.</p>{/if}

    {#if user.role === 'admin'}
      <form class="pform" onsubmit={addProject}>
        <span class="label">Conectar outro banco</span>
        <input class="field" bind:value={form.name} placeholder="Nome (ex.: loja)" aria-label="Nome do projeto" required maxlength="60" />
        <input class="field mono" bind:value={form.url} placeholder="postgres://usuario:senha@host:5432/banco" aria-label="Endereço do banco" required autocomplete="off" spellcheck="false" />
        <p class="hint">Pode ser um banco na mesma máquina ou em outro servidor. O endereço fica guardado no banco principal e não aparece de novo na tela.</p>
        <button class="btn-primary" disabled={form.busy}>{form.busy ? 'Conectando…' : 'Conectar'}</button>
      </form>
    {/if}
    {#if form.error}<p class="error" role="alert">{form.error}</p>{/if}
  </Modal>
{/if}

<style>
  .shell { display: flex; height: 100vh; overflow: hidden; }
  small { display: block; font-size: 12px; color: var(--lavender); }
  strong { font-weight: 600; }

  .side { width: 248px; flex-shrink: 0; padding: 20px 16px; background: linear-gradient(180deg, #0E0F2C 0%, #0B0C24 100%); border-right: 1px solid var(--line); display: flex; flex-direction: column; gap: 20px; overflow-y: auto; }
  .logo { height: 36px; padding: 0 6px; display: flex; align-items: center; gap: 10px; font-size: 19px; color: var(--text); text-decoration: none; }
  .project { height: 52px; flex-shrink: 0; padding: 0 12px; border-radius: 12px; display: flex; align-items: center; gap: 10px; font-size: 14px; text-align: left; color: var(--text); cursor: pointer; }
  .project .grow { min-width: 0; }
  .project strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chev { flex-shrink: 0; color: var(--lavender-2); }
  .plist { display: flex; flex-direction: column; gap: 8px; }
  .prow { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: 12px; border: 1px solid #1E2152; background: rgba(12,13,40,.5); }
  .prow.on { border-color: rgba(154,160,224,.4); }
  .prow .grow { flex: 1; min-width: 0; }
  .prow small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pform { display: flex; flex-direction: column; gap: 10px; padding-top: 16px; border-top: 1px solid var(--line-2); }
  .pform .btn-primary { align-self: flex-end; }
  .pform .mono { font-family: 'Martian Mono', monospace; font-size: 13px; }
  .hint { font-size: 13px; color: var(--lavender-2); }
  .avatar-sq { width: 30px; height: 30px; flex-shrink: 0; border-radius: 8px; background: #2A2E6A; display: grid; place-items: center; font-size: 13px; }
  nav { display: flex; flex-direction: column; gap: 2px; }
  .nav-item { position: relative; height: 38px; flex-shrink: 0; padding: 0 12px; border-radius: 10px; border: 1px solid transparent; display: flex; align-items: center; gap: 12px; font-size: 14px; color: var(--lavender); text-decoration: none; transition: background .18s ease-out, border-color .18s ease-out; }
  a.nav-item:hover:not(.active) { background: rgba(154,160,224,.06); border-color: rgba(154,160,224,.12); }
  .nav-item :global(svg) { stroke: var(--lavender-2); }
  .nav-item.active { background: linear-gradient(180deg, #23266A 0%, #191B4E 100%); border-color: rgba(154,160,224,.32); box-shadow: inset 0 1px 0 rgba(255,255,255,.12); color: var(--text); font-weight: 600; }
  .nav-item.active :global(svg) { stroke: var(--lime); }
  .nav-item.active::before { content: ''; position: absolute; left: -9px; top: 10px; width: 3px; height: 16px; border-radius: 2px; background: var(--lime); }
  .me { padding: 12px 8px 0; border-top: 1px solid var(--line); display: flex; align-items: center; gap: 10px; font-size: 13px; }
  .me small { color: var(--lavender-2); overflow: hidden; text-overflow: ellipsis; }
  .avatar { width: 32px; height: 32px; flex-shrink: 0; border-radius: 50%; background: linear-gradient(180deg, #B3B8EE, #9AA0E0); color: var(--base); display: grid; place-items: center; font-size: 12px; font-weight: 600; }

  .main { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
  .top { height: 64px; flex-shrink: 0; padding: 0 28px; border-bottom: 1px solid var(--line); display: flex; align-items: center; }
  .crumb { font-size: 14px; color: var(--lavender-2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .crumb i { font-style: normal; color: #3E4386; margin: 0 8px; }
  .crumb strong { color: var(--text); }
  .body { flex: 1; min-height: 0; display: flex; }

  /* Tela estreita: barra lateral só com ícones. */
  @media (max-width: 1100px) {
    .side { width: 68px; padding: 20px 10px; align-items: center; }
    .logo .brand, .project .grow, .project .chev, .nav-item span, .me .grow { display: none; }
    .project { width: 44px; height: 44px; padding: 0; justify-content: center; }
    .nav-item { width: 44px; padding: 0; justify-content: center; }
    .nav-item.active::before { left: -11px; }
    .me { flex-direction: column; padding: 12px 0 0; }
  }

  /* Celular: menu vira barra de ícones embaixo, como app. */
  @media (max-width: 700px) {
    .shell { height: 100dvh; flex-direction: column-reverse; }
    .side { width: 100%; height: 60px; flex-shrink: 0; padding: 0 8px; flex-direction: row; align-items: center; gap: 2px; overflow-x: auto; overflow-y: hidden; border-right: 0; border-top: 1px solid var(--line); scrollbar-width: none; }
    .logo, .side > .grow, .me .avatar { display: none; }
    nav { flex-direction: row; gap: 2px; }
    .nav-item { width: 44px; flex-shrink: 0; }
    .nav-item.active::before { left: 14px; top: -11px; width: 16px; height: 3px; }
    .me { padding: 0 0 0 6px; border-top: 0; border-left: 1px solid var(--line); flex-direction: row; }
    .top { height: 52px; padding: 0 16px; }
  }
</style>
