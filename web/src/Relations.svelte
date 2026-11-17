<script>
  // Diagrama das ligações. Tabela que é apontada fica à esquerda; quem aponta fica à direita.
  import { api } from './api.js';
  import { href } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';

  let { onAuthLost } = $props();
  let schema = $state(null);
  let error = $state('');
  let hover = $state(null);

  api('/schema')
    .then((r) => (schema = r))
    .catch((e) => (e.status === 401 ? onAuthLost() : (error = e.message)));

  const W = 232, HEAD = 44, ROW = 30, GAP_X = 96, GAP_Y = 28, PAD = 8;

  const layout = $derived.by(() => {
    if (!schema) return null;
    const by = Object.fromEntries(schema.map((t) => [t.name, t]));
    // Profundidade = maior cadeia de ligações até uma tabela que não aponta para ninguém.
    const depth = {};
    const walk = (name, seen) => {
      if (depth[name] != null) return depth[name];
      if (seen.has(name)) return 0;
      seen.add(name);
      const targets = by[name].columns.map((c) => c.link).filter((l) => l && l !== name && by[l]);
      return (depth[name] = targets.length ? 1 + Math.max(...targets.map((l) => walk(l, seen))) : 0);
    };
    schema.forEach((t) => walk(t.name, new Set()));

    const cols = [];
    for (const t of schema) (cols[depth[t.name]] ??= []).push(t);
    const boxes = {};
    let height = 0;
    cols.forEach((list, d) => {
      let y = PAD;
      for (const t of list) {
        const h = HEAD + t.columns.length * ROW + 8;
        boxes[t.name] = { t, x: PAD + d * (W + GAP_X), y, h };
        y += h + GAP_Y;
      }
      height = Math.max(height, y);
    });

    const lines = [];
    for (const t of schema) {
      t.columns.forEach((c, i) => {
        const to = boxes[c.link];
        if (!c.link || !to) return;
        const from = boxes[t.name];
        const y1 = from.y + HEAD + i * ROW + ROW / 2;
        const y2 = to.y + HEAD / 2;
        const self = c.link === t.name;
        const x1 = self ? from.x + W : from.x;
        const x2 = to.x + W;
        const bend = self ? 40 : Math.max(40, (x1 - x2) / 2);
        const d = self
          ? `M${x1},${y1} C${x1 + bend},${y1} ${x2 + bend},${y2} ${x2},${y2}`
          : `M${x1},${y1} C${x1 - bend},${y1} ${x2 + bend},${y2} ${x2},${y2}`;
        lines.push({ d, from: t.name, to: c.link, x2, y2 });
      });
    }
    return { boxes: Object.values(boxes), lines, width: PAD * 2 + cols.length * W + (cols.length - 1) * GAP_X, height };
  });

  const lit = (l) => hover && (l.from === hover || l.to === hover);
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Relações</h1>
      <p>Como as tabelas se ligam. Para criar uma ligação, adicione uma coluna do tipo "ligação com…" na Estrutura.</p>
    </div>
    <div class="actions">
      <a class="glass btn" href={href('estrutura')}><Icon name="Estrutura" size={15} stroke="#C9CCEB" /> Abrir Estrutura</a>
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if layout?.boxes.length}
    <div class="panel-edge grow-box"><div class="panel canvas">
      <div class="stage" style:width="{layout.width}px" style:height="{layout.height}px">
        <svg width={layout.width} height={layout.height} aria-hidden="true">
          {#each layout.lines as l}
            <path d={l.d} class:lit={lit(l)} />
            <circle cx={l.x2} cy={l.y2} r="4" class:lit={lit(l)} />
          {/each}
        </svg>
        {#each layout.boxes as b (b.t.name)}
          <a class="box" class:lit={hover === b.t.name} href={href('estrutura', b.t.name)} style:left="{b.x}px" style:top="{b.y}px" style:width="{W}px"
            onmouseenter={() => (hover = b.t.name)} onmouseleave={() => (hover = null)} onfocus={() => (hover = b.t.name)} onblur={() => (hover = null)}>
            <strong class="brand">{b.t.name}</strong>
            {#each b.t.columns as c}
              <span class="col">
                <span class="name">{c.name}</span>
                <span class="mono kind" class:link={c.link}>{c.link ? `→ ${c.link}` : c.pk ? 'chave' : c.label}</span>
              </span>
            {/each}
          </a>
        {/each}
      </div>
    </div></div>
    {#if !layout.lines.length}<p class="hint">Nenhuma ligação ainda. As tabelas estão soltas.</p>{/if}
  {:else if schema}
    <div class="empty-state"><Glyph size={56} /><p>Nenhuma tabela ainda.</p></div>
  {/if}
</section>

<style>
  .grow-box { flex: 1; min-height: 320px; }
  .canvas { overflow: auto; padding: 24px; background-image: radial-gradient(rgba(154,160,224,.14) 1px, transparent 1px); background-size: 24px 24px; background-color: #10112F; }
  .stage { position: relative; }
  svg { position: absolute; inset: 0; overflow: visible; }
  path { fill: none; stroke: rgba(154,160,224,.45); stroke-width: 1.5; transition: stroke .18s ease-out; }
  circle { fill: #9AA0E0; transition: fill .18s ease-out; }
  path.lit { stroke: var(--lime); stroke-width: 2; }
  circle.lit { fill: var(--lime); }

  .box { position: absolute; padding: 0 0 8px; border-radius: 14px; display: flex; flex-direction: column; text-decoration: none; color: var(--text); background: linear-gradient(180deg, #20235A 0%, #15173F 100%); border: 1px solid rgba(154,160,224,.32); box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 0 24px rgba(154,160,224,.08); transition: border-color .18s ease-out, box-shadow .18s ease-out; }
  .box:hover, .box.lit { border-color: rgba(154,160,224,.7); box-shadow: inset 0 1px 0 rgba(255,255,255,.14), 0 0 32px rgba(154,160,224,.2); }
  .box strong { height: 44px; padding: 0 14px; display: flex; align-items: center; font-size: 16px; border-bottom: 1px solid rgba(154,160,224,.2); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .col { height: 30px; padding: 0 14px; display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 13px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--soft); }
  .kind { font-size: 10px; color: var(--lavender-3); white-space: nowrap; }
  .kind.link { color: var(--lavender); }
</style>
