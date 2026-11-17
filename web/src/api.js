// Cookie de sessão é httpOnly + SameSite=Strict; o fetch só precisa mandar JSON.
export async function api(path, { method = 'GET', body } = {}) {
  const res = await fetch(`/api${path}`, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
  const data = res.status === 204 || res.headers.get('content-length') === '0' ? null : await res.json().catch(() => null);
  if (!res.ok) throw Object.assign(new Error(data?.error ?? 'Algo deu errado.'), { status: res.status });
  return data;
}
