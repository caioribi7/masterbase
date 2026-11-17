// Página de termos. O texto vem de docs/termos-de-uso.md: um lugar só para editar.
import text from '../../docs/termos-de-uso.md?raw';

const REPO = 'https://github.com/caioribi7/masterbase';

// Negrito e links. Links relativos (../LICENSE) apontam para o arquivo no GitHub.
function inline(s, key = 0) {
  const parts = [];
  const re = /\*\*(.+?)\*\*|\[(.+?)\]\((.+?)\)/g;
  let last = 0;
  let m;
  while ((m = re.exec(s))) {
    if (m.index > last) parts.push(s.slice(last, m.index));
    if (m[1]) parts.push(<strong key={`${key}-${m.index}`}>{m[1]}</strong>);
    else {
      const href = m[3].startsWith('../') ? `${REPO}/blob/main/${m[3].slice(3)}` : m[3];
      parts.push(<a key={`${key}-${m.index}`} href={href}>{m[2]}</a>);
    }
    last = re.lastIndex;
  }
  parts.push(s.slice(last));
  return parts;
}

function blocks(md) {
  const out = [];
  for (const chunk of md.trim().split(/\n\s*\n/)) {
    const lines = chunk.split('\n');
    if (chunk.startsWith('# ')) out.push({ t: 'h1', s: chunk.slice(2) });
    else if (chunk.startsWith('## ')) out.push({ t: 'h2', s: chunk.slice(3) });
    else if (lines.every((l) => l.startsWith('- '))) out.push({ t: 'ul', items: lines.map((l) => l.slice(2)) });
    else out.push({ t: 'p', s: chunk.replace(/\n/g, ' ') });
  }
  return out;
}

export default function Terms() {
  const [title, date, ...rest] = blocks(text);
  return (
    <main className="wrap terms">
      <p className="terms-kicker">Código aberto · Licença MIT</p>
      <h1>{title.s}</h1>
      <p className="terms-date">{date.s}</p>
      <article>
        {rest.map((b, i) =>
          b.t === 'h2' ? <h2 key={i}>{b.s}</h2>
          : b.t === 'ul' ? <ul key={i}>{b.items.map((it, j) => <li key={j}>{inline(it, j)}</li>)}</ul>
          : <p key={i}>{inline(b.s, i)}</p>,
        )}
      </article>
      <a className="glass btn terms-back" href="#inicio">Voltar para o início</a>
    </main>
  );
}
