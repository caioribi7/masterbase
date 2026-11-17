// Preferências deste navegador (tamanho do texto, menos movimento). Ficam só aqui, não vão para o servidor.
const KEY = 'mb-prefs';
export const sizes = [['90', 'Menor'], ['100', 'Normal'], ['115', 'Maior'], ['130', 'Bem maior']];

export function loadPrefs() {
  try { return { size: '100', calm: false, ...JSON.parse(localStorage.getItem(KEY) || '{}') }; }
  catch { return { size: '100', calm: false }; }
}

export function applyPrefs(p = loadPrefs()) {
  const root = document.documentElement;
  // zoom aumenta tudo junto (texto, botões, tabelas) e mantém o layout. Funciona em todos os navegadores atuais.
  root.style.zoom = p.size === '100' ? '' : `${p.size}%`;
  root.classList.toggle('calm', !!p.calm);
}

export function savePrefs(p) {
  try { localStorage.setItem(KEY, JSON.stringify(p)); } catch {}
  applyPrefs(p);
}
