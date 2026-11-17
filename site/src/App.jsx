import { useState } from 'react';
import FluxLines from './FluxLines.jsx';

const REPO = 'https://github.com/caioribi7/masterbase';
const INSTALL = 'curl -fsSL https://raw.githubusercontent.com/caioribi7/masterbase/main/install.sh | sh';

// Âncora-chave: só o glifo, em lima, sem caixa em volta.
function Glyph({ size = 28 }) {
  return (
    <svg width={size} height={size} viewBox="34 34 188 188" fill="none" aria-hidden="true">
      <path fillRule="evenodd" fill="#E6FF00" d="M98 70a30 30 0 1 0 60 0a30 30 0 1 0 -60 0Z M120 86L123 71.48A9 9 0 1 1 133 71.48L136 86Z" />
      <rect x="120" y="98" width="16" height="114" fill="#E6FF00" />
      <rect x="130" y="116" width="30" height="15" rx="4" fill="#E6FF00" />
      <rect x="130" y="141" width="22" height="15" rx="4" fill="#E6FF00" />
      <path d="M66 150A62 62 0 0 0 190 150" stroke="#E6FF00" strokeWidth="16" />
      <path d="M52 154L66 129.75L80 154Z" fill="#E6FF00" stroke="#E6FF00" strokeWidth="8" strokeLinejoin="round" />
      <path d="M176 154L190 129.75L204 154Z" fill="#E6FF00" stroke="#E6FF00" strokeWidth="8" strokeLinejoin="round" />
    </svg>
  );
}

const paths = {
  copy: 'M9 9h10a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V10a1 1 0 0 1 1-1zM5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1',
  check: 'M5 12l5 5L20 7',
  lock: 'M6 11h12v10H6zM8 11V7a4 4 0 0 1 8 0v4',
  table: 'M3 5h18v14H3zM3 10h18M9 10v9',
  columns: 'M4 4h16v16H4zM9 4v16M15 4v16',
  link: 'M5 6h5v5H5zM14 13h5v5h-5zM10 8.5h3a2 2 0 0 1 2 2V13',
  filter: 'M4 5h16l-6 7v6l-4 2v-8z',
  people: 'M9 11a3 3 0 1 0 0-6a3 3 0 1 0 0 6M3 20a6 6 0 0 1 12 0M16 5a3 3 0 0 1 0 6M18 14a5 5 0 0 1 3 6',
  shield: 'M12 3l8 3v6c0 5-3.5 8-8 9c-4.5-1-8-4-8-9V6z M9 12l2 2l4-4',
  arrow: 'M5 12h14M13 6l6 6l-6 6',
};

function Icon({ name, size = 18 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={paths[name]} />
    </svg>
  );
}

function GitHub({ size = 18 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34c-.45-1.16-1.11-1.47-1.11-1.47c-.91-.62.07-.6.07-.6c1 .07 1.53 1.03 1.53 1.03c.9 1.52 2.34 1.08 2.91.83c.09-.65.35-1.08.63-1.33c-2.22-.25-4.55-1.11-4.55-4.94c0-1.09.39-1.98 1.03-2.68c-.1-.25-.45-1.27.1-2.64c0 0 .84-.27 2.75 1.02a9.6 9.6 0 0 1 5 0c1.91-1.29 2.75-1.02 2.75-1.02c.55 1.37.2 2.39.1 2.64c.64.7 1.03 1.59 1.03 2.68c0 3.84-2.34 4.68-4.57 4.93c.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48A10 10 0 0 0 12 2z" />
    </svg>
  );
}

function CopyLine({ text }) {
  const [done, setDone] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setDone(true);
      setTimeout(() => setDone(false), 1800);
    } catch {
      /* sem permissão de área de transferência: o texto continua selecionável */
    }
  };
  return (
    <div className="glass copyline">
      <span className="prompt" aria-hidden="true">$</span>
      <code>{text}</code>
      <button className="iconbtn" onClick={copy} aria-label={done ? 'Copiado' : 'Copiar comando'}>
        <Icon name={done ? 'check' : 'copy'} size={16} />
      </button>
    </div>
  );
}

// A tela de Tabelas, desenhada em HTML: nítida em qualquer tamanho e sem imagem pesada.
const rows = [
  [1, 'Café Bom Dia', 'Curitiba', 'pro', true],
  [2, 'Padaria Aurora', 'Londrina', 'básico', true],
  [3, 'Doceria Lua', 'Maringá', 'pro', false],
  [4, 'Moinho Sul', 'Pelotas', 'básico', true],
  [5, 'Empório Serra', 'Gramado', 'pro', true],
];

function Product() {
  return (
    <div className="product glass-edge" aria-label="Exemplo da tela de tabelas do Masterbase" role="img">
      <div className="product-in">
        <aside className="p-side">
          <div className="p-brand"><Glyph size={18} /> Masterbase</div>
          {['Início', 'Tabelas', 'Estrutura', 'Relações', 'Consultas', 'Acesso', 'Backups'].map((n) => (
            <div key={n} className={n === 'Tabelas' ? 'p-nav on' : 'p-nav'}>{n}</div>
          ))}
        </aside>
        <div className="p-main">
          <div className="p-head">
            <div>
              <div className="p-title">clientes</div>
              <div className="p-sub">5 linhas</div>
            </div>
            <div className="p-actions">
              <span className="p-chip">cidade contém “r”</span>
              <span className="p-btn">+ Nova linha</span>
            </div>
          </div>
          <div className="p-table">
            <div className="p-row p-th">
              <span>número</span><span>nome</span><span>cidade</span><span>plano</span><span>ativo</span>
            </div>
            {rows.map((r) => (
              <div key={r[0]} className={r[0] === 2 ? 'p-row sel' : 'p-row'}>
                <span className="mono">{r[0]}</span>
                <span>{r[1]}</span>
                <span>{r[2]}</span>
                <span><i className={r[3] === 'pro' ? 'tag lime' : 'tag'}>{r[3]}</i></span>
                <span>{r[4] ? 'sim' : 'não'}</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

const features = [
  ['table', 'Tabelas', 'Veja, filtre e ordene como numa planilha. Duplo clique numa célula para mudar o valor.'],
  ['columns', 'Estrutura', 'Crie tabelas e colunas com tipos em português: texto, número, data, sim/não.'],
  ['link', 'Relações', 'Ligue pedidos a clientes sem escrever chave estrangeira. O diagrama mostra as ligações.'],
  ['filter', 'Consultas', 'Monte “clientes onde plano é pro” clicando. O SQL gerado aparece ao lado.'],
  ['people', 'Acesso', 'Administrador, editor e leitor. Cada pessoa vê e muda só o que o papel permite.'],
  ['shield', 'Backups', 'Um por dia, criptografado. Os antigos são apagados sozinhos e a chave fica com você.'],
];

export default function App() {
  return (
    <>
      <header className="top">
        <div className="wrap top-in">
          <a className="brand" href="#inicio"><Glyph size={26} /> Masterbase</a>
          <nav>
            <a href="#recursos">Recursos</a>
            <a href="#servidor">Seu servidor</a>
            <a href="#integrar">Integrar</a>
            <a className="gh" href={REPO} aria-label="Código no GitHub"><GitHub /></a>
          </nav>
        </div>
      </header>

      <main>
        <section className="hero" id="inicio">
          <FluxLines
            className="hero-bg"
            colors={['#3C40A8', '#9AA0E0', '#C9CCEB', '#E6FF00']}
            speed={0.35}
            scale={1.4}
            glow={1.6}
            opacity={0.85}
            flowDirection="down"
            mouseStrength={0.8}
          />
          <div className="wrap hero-in">
            <h1>Postgres com tela, para quem não sabe SQL</h1>
            <p className="lead">Crie tabelas, edite dados e dê acesso à equipe pelo navegador. O Masterbase instala no seu servidor com um comando, e os dados não saem de lá.</p>
            <div className="cta">
              <a className="primary" href="#servidor">Instalar <Icon name="arrow" size={16} /></a>
              <a className="glass btn" href={REPO}><GitHub size={16} /> Código no GitHub</a>
            </div>
          </div>
          <div className="wrap hero-shot">
            <Product />
          </div>
        </section>

        <section className="wrap section" id="recursos">
          <h2>O banco inteiro, sem decorar comando</h2>
          <p className="section-lead">Por baixo é Postgres de verdade. Quem sabe SQL tem o editor; quem não sabe faz o mesmo clicando.</p>
          <div className="grid">
            {features.map(([icon, title, text]) => (
              <article key={title} className="glass card">
                <span className="card-icon"><Icon name={icon} /></span>
                <h3>{title}</h3>
                <p>{text}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="wrap section split" id="servidor">
          <div>
            <h2>No seu servidor, do seu jeito</h2>
            <p className="section-lead">O instalador prepara o Postgres, baixa o programa e confere a assinatura do arquivo. Depois é digitar <code>masterbase</code> e abrir o navegador.</p>
            <ul className="facts">
              <li><Icon name="check" size={16} /> Um programa só, sem Docker nem dependência para instalar à mão.</li>
              <li><Icon name="check" size={16} /> Funciona em Ubuntu, Debian e macOS.</li>
              <li><Icon name="check" size={16} /> Backup diário criptografado, com chave de recuperação.</li>
              <li><Icon name="check" size={16} /> Código aberto, licença MIT.</li>
            </ul>
          </div>
          <div className="terminal glass">
            <div className="term-bar"><i /><i /><i /></div>
            <pre>
              <span className="dim"># instalar</span>{'\n'}
              <span className="lime">$</span> curl -fsSL …/install.sh | sh{'\n\n'}
              <span className="dim"># abrir o painel</span>{'\n'}
              <span className="lime">$</span> masterbase{'\n'}
              <span className="dim">Masterbase em http://127.0.0.1:8080</span>{'\n\n'}
              <span className="dim"># deixar ligado com o servidor</span>{'\n'}
              <span className="lime">$</span> sudo masterbase servico
            </pre>
          </div>
          <div className="install">
            <CopyLine text={INSTALL} />
          </div>
        </section>

        <section className="wrap section split" id="integrar">
          <div>
            <h2>Peça a integração para uma IA</h2>
            <p className="section-lead">Todo Masterbase serve a própria documentação em <code>/llms.txt</code>. Mande o endereço para o ChatGPT, o Claude ou o Copilot e peça o código para o seu site ler e gravar nas tabelas.</p>
            <p className="section-lead">A chave de API pode só ler ou ler e editar linhas. Nenhuma chave mexe em estrutura, pessoas ou backups.</p>
          </div>
          <div className="terminal glass">
            <div className="term-bar"><i /><i /><i /></div>
            <pre>
              <span className="dim">{'// pedidos de um cliente'}</span>{'\n'}
              <span className="kw">const</span> r = <span className="kw">await</span> fetch({'\n'}
              {'  '}<span className="str">`${'{'}URL{'}'}/api/tables/pedidos?col=cliente&val=3`</span>,{'\n'}
              {'  '}{'{'} headers: {'{'} Authorization: <span className="str">`Bearer ${'{'}CHAVE{'}'}`</span> {'}'} {'}'}{'\n'}
              );{'\n'}
              <span className="kw">const</span> {'{'} rows {'}'} = <span className="kw">await</span> r.json();
            </pre>
          </div>
        </section>
      </main>

      <footer className="wrap foot">
        <a className="brand" href="#inicio"><Glyph size={22} /> Masterbase</a>
        <p><Icon name="lock" size={15} /> Código aberto. Seus dados ficam no seu servidor.</p>
        <a className="gh" href={REPO}><GitHub size={16} /> GitHub</a>
      </footer>
    </>
  );
}
