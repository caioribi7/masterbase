<script>
  // Diagrama das ligações. Tabela que é apontada fica à esquerda; quem aponta fica à direita.
  import { api } from './api.js';
  import { href } from './lib.js';
  import Glyph from './Glyph.svelte';
  import Icon from './Icon.svelte';
  import Modal from './Modal.svelte';
  import { isAdmin } from './session.svelte.js';

  let { onAuthLost } = $props();
  let schema = $state(null);
  let error = $state('');
  let hover = $state(null);

  const load = () => api('/schema')
    .then((r) => (schema = r))
    .catch((e) => (e.status === 401 ? onAuthLost() : (error = e.message)));
  load();

  // Arrastar o ponto de uma tabela até outra cria a ligação (uma coluna "ligação com..." na de origem).
  let stage = $state(null);
  let linking = $state(null); // { from, x1, y1, x2, y2 }
  let pending = $state(null); // { from, to, name }
  let linkError = $state('');
  let saving = $state(false);
  const point = (e) => { const r = stage.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  function linkStart(e, b) {
    e.stopPropagation();
    e.preventDefault();
    try { e.currentTarget.setPointerCapture(e.pointerId); } catch {}
    const [x, y] = point(e);
    linking = { from: b.t.name, x1: b.x, y1: b.y + HEAD / 2, x2: x, y2: y };
  }
  function linkMove(e) {
    if (!linking) return;
    [linking.x2, linking.y2] = point(e);
  }
  function linkEnd(e) {
    if (!linking) return;
    const [x, y] = point(e);
    const to = layout.boxes.find((b) => x >= b.x && x <= b.x + W && y >= b.y && y <= b.y + b.h);
    if (to) {
      const taken = new Set(schema.find((t) => t.name === linking.from).columns.map((c) => c.name));
      let name = `${to.t.name}_id`;
      for (let i = 2; taken.has(name); i++) name = `${to.t.name}_id${i}`;
      pending = { from: linking.from, to: to.t.name, name };
      linkError = '';
    }
    linking = null;
  }
  async function createLink(e) {
    e.preventDefault();
    saving = true;
    linkError = '';
    try {
      await api(`/schema/${encodeURIComponent(pending.from)}/columns`, { method: 'POST', body: { name: pending.name, kind: 'ligação', link: pending.to } });
      pending = null;
      await load();
    } catch (err) {
      if (err.status === 401) return onAuthLost();
      linkError = err.message;
    }
    saving = false;
  }

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

  // Arrastar com o mouse move o diagrama. Se arrastou, o clique na tabela não abre a Estrutura.
  let drag = null;
  let moved = $state(false);
  let grabbing = $state(false);
  function down(e) {
    if (e.button !== 0 || e.pointerType === 'touch') return;
    const el = e.currentTarget;
    drag = { el, x: e.clientX, y: e.clientY, left: el.scrollLeft, top: el.scrollTop, id: e.pointerId };
    moved = false;
  }
  function move(e) {
    if (!drag) return;
    const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
    if (!moved && Math.hypot(dx, dy) < 4) return;
    if (!moved) { moved = true; grabbing = true; drag.el.setPointerCapture(drag.id); }
    drag.el.scrollLeft = drag.left - dx;
    drag.el.scrollTop = drag.top - dy;
  }
  function up() {
    drag = null;
    grabbing = false;
  }
  function click(e) {
    if (moved) { e.preventDefault(); e.stopPropagation(); moved = false; }
  }
</script>

<section class="content">
  <div class="page-head">
    <div>
      <h1 class="brand">Relações</h1>
      <p>Como as tabelas se ligam.<span class="desk"> Arraste o fundo para mover{#if isAdmin()} e o ponto de uma tabela até outra para ligar as duas{/if}.</span> Para criar uma ligação, adicione uma coluna do tipo "ligação com…" na Estrutura.</p>
    </div>
    <div class="actions">
      <a class="glass btn" href={href('estrutura')}><Icon name="Estrutura" size={15} stroke="#C9CCEB" /> Abrir Estrutura</a>
    </div>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if layout?.boxes.length}
    <!-- Celular: lista de quem aponta para quem. O diagrama não cabe na tela. -->
    <div class="list">
      {#each schema as t (t.name)}
        {@const out = t.columns.filter((c) => c.link)}
        {@const into = schema.flatMap((o) => o.columns.filter((c) => c.link === t.name).map((c) => ({ table: o.name, col: c.name })))}
        <a class="item-edge" href={href('estrutura', t.name)}><span class="item">
          <span class="item-head"><strong class="brand">{t.name}</strong><span class="mono count">{t.columns.length} colunas</span></span>
          {#each out as c}
            <span class="rel"><span class="arrow">→</span><span><span class="mono">{c.name}</span> liga com <strong>{c.link}</strong></span></span>
          {/each}
          {#each into as r}
            <span class="rel"><span class="arrow in">←</span><span><strong>{r.table}</strong> liga aqui por <span class="mono">{r.col}</span></span></span>
          {/each}
          {#if !out.length && !into.length}<span class="rel solta">Sem ligações</span>{/if}
        </span></a>
      {/each}
    </div>

    <div class="panel-edge grow-box"><!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="panel canvas" class:grabbing onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onclickcapture={click}>
      <div class="stage" bind:this={stage} style:width="{layout.width}px" style:height="{layout.height}px">
        <svg width={layout.width} height={layout.height} aria-hidden="true">
          {#each layout.lines as l}
            <path d={l.d} class:lit={lit(l)} />
            <circle cx={l.x2} cy={l.y2} r="4" class:lit={lit(l)} />
          {/each}
          {#if linking}
            <path class="drawing" d="M{linking.x1},{linking.y1} C{linking.x1 - 60},{linking.y1} {linking.x2 + 60},{linking.y2} {linking.x2},{linking.y2}" />
          {/if}
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
          {#if isAdmin()}
            <button type="button" class="handle" style:left="{b.x - 7}px" style:top="{b.y + HEAD / 2 - 7}px"
              aria-label="Ligar {b.t.name} a outra tabela" title="Arraste até outra tabela para ligar"
              onpointerdown={(e) => linkStart(e, b)} onpointermove={linkMove} onpointerup={linkEnd} onpointercancel={() => (linking = null)}></button>
          {/if}
        {/each}
      </div>
    </div></div>
    {#if !layout.lines.length}<p class="hint desk">Nenhuma ligação ainda. As tabelas estão soltas.</p>{/if}
  {:else if schema}
    <div class="empty-state"><Glyph size={56} /><p>Nenhuma tabela ainda.</p></div>
  {/if}
</section>

{#if pending}
  <Modal title="Ligar {pending.from} a {pending.to}" onClose={() => (pending = null)} width={460}>
    <form class="form" onsubmit={createLink}>
      <p class="hint">Cada linha de <strong>{pending.from}</strong> vai poder apontar para uma linha de <strong>{pending.to}</strong>. Isso cria uma coluna nova em {pending.from}.</p>
      <label><span class="label">Nome da coluna</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field" bind:value={pending.name} required maxlength="63" autofocus />
      </label>
      {#if linkError}<p class="error" role="alert">{linkError}</p>{/if}
      <div class="modal-actions">
        <button type="button" class="glass btn" onclick={() => (pending = null)}>Cancelar</button>
        <button class="btn-primary" disabled={saving}>Ligar</button>
      </div>
    </form>
  </Modal>
{/if}

<style>
  .grow-box { flex: 1; min-height: 320px; display: flex; }
  .grow-box > .canvas { flex: 1; min-width: 0; }
  .canvas { overflow: auto; overscroll-behavior: contain; cursor: grab; user-select: none; scrollbar-width: thin; scrollbar-color: #2A2E6A transparent; padding: 24px; background-image: radial-gradient(rgba(154,160,224,.14) 1px, transparent 1px); background-size: 24px 24px; background-color: #10112F; }
  .canvas.grabbing, .canvas.grabbing .box { cursor: grabbing; }
  .stage { position: relative; }
  .box :global(*) { pointer-events: none; }
  .box { -webkit-user-drag: none; }
  svg { position: absolute; inset: 0; overflow: visible; }
  path { fill: none; stroke: rgba(154,160,224,.45); stroke-width: 1.5; transition: stroke .18s ease-out; }
  circle { fill: #9AA0E0; transition: fill .18s ease-out; }
  path.lit { stroke: var(--lime); stroke-width: 2; }
  path.drawing { stroke: var(--lime); stroke-width: 2; stroke-dasharray: 6 5; }
  .handle { position: absolute; z-index: 2; width: 14px; height: 14px; padding: 0; border-radius: 50%; border: 2px solid #9AA0E0; background: #10112F; cursor: crosshair; touch-action: none; transition: background .16s ease-out, transform .16s ease-out; }
  .handle:hover, .handle:focus-visible { background: var(--lime); border-color: var(--lime); transform: scale(1.2); }
  .hint strong { color: var(--text); }
  circle.lit { fill: var(--lime); }

  .box { position: absolute; padding: 0 0 8px; border-radius: 14px; display: flex; flex-direction: column; text-decoration: none; color: var(--text); background: linear-gradient(180deg, #20235A 0%, #15173F 100%); border: 1px solid rgba(154,160,224,.32); box-shadow: inset 0 1px 0 rgba(255,255,255,.12), 0 0 24px rgba(154,160,224,.08); transition: border-color .18s ease-out, box-shadow .18s ease-out; }
  .box:hover, .box.lit { border-color: rgba(154,160,224,.7); box-shadow: inset 0 1px 0 rgba(255,255,255,.14), 0 0 32px rgba(154,160,224,.2); }
  .box strong { height: 44px; padding: 0 14px; display: flex; align-items: center; font-size: 16px; border-bottom: 1px solid rgba(154,160,224,.2); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .col { height: 30px; padding: 0 14px; display: flex; align-items: center; justify-content: space-between; gap: 10px; font-size: 13px; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--soft); }
  .kind { font-size: 10px; color: var(--lavender-3); white-space: nowrap; }
  .kind.link { color: var(--lavender); }

  .list { display: none; flex-direction: column; gap: 10px; }
  .item-edge { padding: 1px; border-radius: 16px; background: var(--edge); text-decoration: none; color: var(--text); }
  .item { border-radius: 15px; padding: 14px 16px; display: flex; flex-direction: column; gap: 8px; background: linear-gradient(180deg, #17194A 0%, #11123A 100%); box-shadow: var(--inner); }
  .item-head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
  .item-head strong { font-size: 17px; }
  .count { font-size: 11px; color: var(--lavender-3); }
  .rel { display: flex; gap: 10px; font-size: 14px; line-height: 20px; color: var(--lavender-2); }
  .rel strong { color: var(--text); font-weight: 600; }
  .rel .mono { font-size: 12px; color: var(--soft); }
  .arrow { width: 16px; flex-shrink: 0; color: var(--lime); }
  .arrow.in { color: var(--lavender); }
  .solta { color: var(--lavender-3); font-size: 13px; }

  @media (max-width: 700px) {
    .desk { display: none; }
    .list { display: flex; }
    .grow-box { display: none; }
  }
</style>
