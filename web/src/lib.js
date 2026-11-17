// Ícones (traço 24x24), formatação e rota por hash. Sem dependência extra.

export const icons = {
  'Início': 'M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z',
  'Tabelas': 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM3 10h18M9 4v16',
  'Estrutura': 'M4 6h16M4 12h16M4 18h10',
  'Relações': 'M6 3a3 3 0 1 0 0 6a3 3 0 1 0 0-6M18 15a3 3 0 1 0 0 6a3 3 0 1 0 0-6M9 6h3a3 3 0 0 1 3 3v6',
  'Consultas': 'M4 5h16l-6 7v6l-4 2v-8z',
  'Acesso': 'M12 4a4 4 0 1 0 0 8a4 4 0 1 0 0-8M4 21a8 8 0 0 1 16 0',
  'API e chaves': 'M8 11a4 4 0 1 0 0 8a4 4 0 1 0 0-8M11 12l9-9M17 6l3 3',
  'Backups': 'M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 7v5l3 2',
  'Atividade': 'M3 12h4l3-8 4 16 3-8h4',
  'Editor SQL': 'M4 4h16a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM7 9l3 3-3 3M13 15h4',
  'Configurações': 'M4 7h9M17 7h3M4 17h3M11 17h9M15 5v4M9 15v4',
  plus: 'M12 5v14M5 12h14',
  trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
  pencil: 'M4 20h4L19 9l-4-4L4 16zM13 7l4 4',
  close: 'M6 6l12 12M18 6L6 18',
  download: 'M12 3v12M7 10l5 5 5-5M4 15v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4',
  filter: 'M4 5h16l-6 7v6l-4 2v-8z',
  left: 'M15 6l-6 6 6 6',
  right: 'M9 6l6 6-6 6',
  up: 'M6 15l6-6 6 6',
  down: 'M6 9l6 6 6-6',
  link: 'M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1 1M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1-1',
  key: 'M8 11a4 4 0 1 0 0 8a4 4 0 1 0 0-8M11 12l9-9M17 6l3 3',
  logout: 'M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3M10 17l5-5-5-5M15 12H4',
  upload: 'M12 15V3M7 8l5-5 5 5M4 15v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4',
  copy: 'M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1zM5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1',
  play: 'M7 4l13 8-13 8z',
  save: 'M5 3h11l4 4v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM8 3v6h7M8 21v-7h8v7',
  restore: 'M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5M12 7v5l3 2',
  grid: 'M3 6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM3 10h18M9 4v16',
};

// Página <-> hash. #/tabelas/clientes -> { page: 'tabelas', arg: 'clientes' }
export function parseHash() {
  const [page = 'inicio', ...rest] = location.hash.replace(/^#\/?/, '').split('/');
  return { page: page || 'inicio', arg: rest.length ? decodeURIComponent(rest.join('/')) : null };
}
export const href = (page, arg) => `#/${page}${arg ? '/' + encodeURIComponent(arg) : ''}`;
export const go = (page, arg) => (location.hash = href(page, arg));

// Nome do papel para mostrar na tela.
export const roleLabel = (r) => ({ admin: 'Administrador', editor: 'Editor', leitor: 'Leitor' })[r] ?? r;

export const n = (x) => Number(x).toLocaleString('pt-BR');

const rtf = new Intl.RelativeTimeFormat('pt-BR', { numeric: 'auto' });
export function ago(iso) {
  const s = (new Date(iso) - Date.now()) / 1000;
  const steps = [[60, 'second'], [3600, 'minute', 60], [86400, 'hour', 3600], [604800, 'day', 86400]];
  if (-s < 45) return 'agora';
  for (const [limit, unit, div = 1] of steps) if (-s < limit) return rtf.format(Math.round(s / div), unit);
  return new Date(iso).toLocaleDateString('pt-BR');
}

// Registro de atividade em frase. Partes { t: texto, b: destaque }.
export function phrase(a) {
  const d = a.detail ?? {};
  const T = { b: d.table };
  switch (a.action) {
    case 'login': return [{ t: 'entrou' }];
    case 'signup': return [{ t: 'criou a conta de administrador' }];
    case 'sql': return [{ t: 'rodou SQL: ' }, { b: d.sql?.length > 70 ? d.sql.slice(0, 70) + '…' : d.sql }];
    case 'update': return [{ t: 'editou ' }, { b: d.column }, { t: ' em ' }, T];
    case 'insert': return [{ t: 'adicionou uma linha em ' }, T];
    case 'delete': return [{ t: `apagou ${d.rows} ${d.rows === 1 ? 'linha' : 'linhas'} de ` }, T];
    case 'create_table': return [{ t: 'criou a tabela ' }, T];
    case 'rename_table': return [{ t: 'renomeou ' }, T, { t: ' para ' }, { b: d.to }];
    case 'drop_table': return [{ t: 'apagou a tabela ' }, T];
    case 'add_column': return [{ t: 'criou a coluna ' }, { b: d.column }, { t: ' em ' }, T];
    case 'drop_column': return [{ t: 'apagou a coluna ' }, { b: d.column }, { t: ' de ' }, T];
    case 'change_column':
      return d.renamed && d.renamed !== d.column
        ? [{ t: 'renomeou a coluna ' }, { b: d.column }, { t: ' para ' }, { b: d.renamed }, { t: ' em ' }, T]
        : [{ t: 'mudou a coluna ' }, { b: d.column }, { t: ' em ' }, T];
    case 'import': return [{ t: `importou ${d.rows} ${d.rows === 1 ? 'linha' : 'linhas'} em ` }, T];
    case 'add_user': return [{ t: 'deu acesso a ' }, { b: d.email }, { t: ` como ${roleLabel(d.role)}` }];
    case 'change_user': return [{ t: d.active === false ? 'desativou ' : d.active ? 'reativou ' : d.password ? 'trocou a senha de ' : 'mudou o papel de ' }, { b: d.email }, ...(d.role ? [{ t: ` para ${roleLabel(d.role)}` }] : [])];
    case 'change_password': return [{ t: 'trocou a própria senha' }];
    case 'add_key': return [{ t: 'criou a chave de API ' }, { b: d.name }];
    case 'drop_key': return [{ t: 'revogou a chave de API ' }, { b: d.name }];
    case 'backup': return [{ t: 'fez um backup' }];
    case 'drop_backup': return [{ t: 'apagou um backup' }];
    case 'restore': return [{ t: 'restaurou o banco a partir de um backup' }];
    default: return [{ t: a.action }];
  }
}

// Tipos que a Estrutura oferece (iguais a catalog.rs::kind_sql) + ligação.
export const kinds = ['texto', 'número', 'decimal', 'sim/não', 'data', 'data e hora', 'dados', 'id único', 'ligação'];

// Pedido entre páginas (ex.: Início -> "Criar tabela" abre o formulário na Estrutura).
export const intent = { createTable: false };

// CSV simples (RFC 4180): aspas, vírgula ou ponto e vírgula. Devolve matriz de textos.
export function parseCsv(text) {
  text = text.replace(/^\uFEFF/, '');
  const first = text.split(/\r?\n/, 1)[0];
  const sep = (first.match(/;/g)?.length ?? 0) > (first.match(/,/g)?.length ?? 0) ? ';' : ',';
  const rows = [];
  let row = [], cell = '', quoted = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"' && text[i + 1] === '"') { cell += '"'; i++; }
      else if (ch === '"') quoted = false;
      else cell += ch;
    } else if (ch === '"') quoted = true;
    else if (ch === sep) { row.push(cell); cell = ''; }
    else if (ch === '\n' || ch === '\r') {
      if (ch === '\r' && text[i + 1] === '\n') i++;
      row.push(cell); cell = '';
      if (row.some((c) => c !== '')) rows.push(row);
      row = [];
    } else cell += ch;
  }
  row.push(cell);
  if (row.some((c) => c !== '')) rows.push(row);
  return rows;
}

export const bytes = (b) => (b < 1024 ? `${b} B` : b < 1048576 ? `${(b / 1024).toFixed(0)} kB` : `${(b / 1048576).toFixed(1)} MB`);
