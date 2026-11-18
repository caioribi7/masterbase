// Projeto aberto: vale só neste navegador. 0 ou nada = banco principal.
const KEY = 'mb-project';
export function currentProject() {
  try { return Number(localStorage.getItem(KEY)) || 0; } catch { return 0; }
}
export function openProject(id) {
  try { id ? localStorage.setItem(KEY, String(id)) : localStorage.removeItem(KEY); } catch {}
  location.hash = '#/inicio';
  location.reload();
}

// Cookie de sessão é httpOnly + SameSite=Strict; o fetch só precisa mandar JSON.
export async function api(path, { method = 'GET', body } = {}) {
  const headers = body ? { 'Content-Type': 'application/json' } : {};
  const project = currentProject();
  if (project) headers['X-MB-Project'] = String(project);
  const res = await fetch(`/api${path}`, { method, headers, body: body ? JSON.stringify(body) : undefined });
  const data = res.status === 204 || res.headers.get('content-length') === '0' ? null : await res.json().catch(() => null);
  // O projeto guardado foi tirado do painel: volta para o principal.
  if (res.status === 404 && project && data?.error === 'Projeto não existe.') openProject(0);
  if (!res.ok) throw Object.assign(new Error(data?.error ?? 'Algo deu errado.'), { status: res.status });
  return data;
}
