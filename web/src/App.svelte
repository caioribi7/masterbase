<script>
  import { api } from './api.js';
  import { parseHash } from './lib.js';
  import Login from './Login.svelte';
  import Shell from './Shell.svelte';
  import Home from './Home.svelte';
  import Tables from './Tables.svelte';
  import Structure from './Structure.svelte';
  import Activity from './Activity.svelte';
  import Relations from './Relations.svelte';
  import Queries from './Queries.svelte';
  import Access from './Access.svelte';
  import Keys from './Keys.svelte';
  import Backups from './Backups.svelte';
  import { session } from './session.svelte.js';

  let user = $state(undefined); // undefined = carregando, null = sem sessão
  let route = $state(parseHash());

  async function loadMe() {
    const me = await api('/me').catch(() => null);
    session.user = me;
    user = me;
  }
  loadMe();

  const lost = () => (user = null);
  const names = { inicio: 'Início', tabelas: 'Tabelas', estrutura: 'Estrutura', relacoes: 'Relações', consultas: 'Consultas', acesso: 'Acesso', chaves: 'API e chaves', backups: 'Backups', atividade: 'Atividade' };
  const page = $derived(names[route.page] ? route.page : 'inicio');
  // A página informa o nome que está aberto (ex.: primeira tabela quando a rota não tem nome).
  let open = $state(null);
  const crumb = $derived([names[page], ...(open && (page === 'tabelas' || page === 'estrutura') ? [open] : [])]);
</script>

<svelte:window onhashchange={() => (route = parseHash())} />

{#if user === null}
  <Login onDone={loadMe} />
{:else if user}
  <Shell {user} {page} {crumb} onLogout={lost}>
    {#key page}
      {#if page === 'inicio'}<Home onAuthLost={lost} />
      {:else if page === 'tabelas'}<Tables arg={route.arg} onAuthLost={lost} onOpen={(n) => (open = n)} />
      {:else if page === 'estrutura'}<Structure arg={route.arg} onAuthLost={lost} onOpen={(n) => (open = n)} />
      {:else if page === 'relacoes'}<Relations onAuthLost={lost} />
      {:else if page === 'consultas'}<Queries onAuthLost={lost} />
      {:else if page === 'acesso'}<Access onAuthLost={lost} />
      {:else if page === 'chaves'}<Keys onAuthLost={lost} />
      {:else if page === 'backups'}<Backups onAuthLost={lost} />
      {:else}<Activity arg={route.arg} onAuthLost={lost} />{/if}
    {/key}
  </Shell>
{/if}
