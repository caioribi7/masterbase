<script>
  import { api } from './api.js';
  import Login from './Login.svelte';
  import Tables from './Tables.svelte';

  let user = $state(undefined); // undefined = carregando, null = sem sessão

  async function loadMe() {
    user = await api('/me').catch(() => null);
  }
  loadMe();
</script>

{#if user === null}
  <Login onDone={loadMe} />
{:else if user}
  <Tables {user} onLogout={() => (user = null)} />
{/if}
