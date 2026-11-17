<script>
  import { api } from './api.js';
  import { parseHash } from './lib.js';
  import Login from './Login.svelte';
  import Shell from './Shell.svelte';
  import Home from './Home.svelte';
  import Tables from './Tables.svelte';
  import Structure from './Structure.svelte';
  import Activity from './Activity.svelte';

  let user = $state(undefined); // undefined = carregando, null = sem sessão
  let route = $state(parseHash());

  async function loadMe() {
    user = await api('/me').catch(() => null);
  }
  loadMe();

  const lost = () => (user = null);
  const names = { inicio: 'Início', tabelas: 'Tabelas', estrutura: 'Estrutura', atividade: 'Atividade' };
  const page = $derived(names[route.page] ? route.page : 'inicio');
  // A página informa o nome que está aberto (ex.: primeira tabela quando a rota não tem nome).
  let open = $state(null);
  const crumb = $derived([names[page], ...(open && page !== 'inicio' && page !== 'atividade' ? [open] : [])]);
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
      {:else}<Activity arg={route.arg} onAuthLost={lost} />{/if}
    {/key}
  </Shell>
{/if}
