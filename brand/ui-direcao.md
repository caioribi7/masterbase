# Masterbase — Direção de interface

> Regra fixa: **toda superfície do Masterbase tem glow.** Botões, cards, inputs, modais, abas e tabelas seguem o mesmo acabamento de vidro escuro com borda luminosa da referência *Get Started* e do ícone perolado gerado no GPT Images. Nenhum elemento chapado e sem luz entra no produto.

---

## 1. O que a referência ensina

Referência: o botão escuro "Get Started".

| Camada | O que acontece | Como fica no Masterbase |
|---|---|---|
| **Fundo da página** | Quase preto e frio, sem textura | `--mb-void #0A0B1F`, um índigo bem escuro (nunca preto puro) |
| **Preenchimento** | Gradiente vertical sutil: um pouco mais claro no topo e mais escuro embaixo | `linear-gradient(180deg, #1C1E52 0%, #12133A 100%)` |
| **Borda** | 1px fina e **não uniforme**. Ela brilha no canto superior esquerdo e no inferior direito e quase some no meio dos lados, como luz batendo num vidro | Borda em gradiente cônico de lavanda `#9AA0E0` → transparente → lavanda |
| **Inner glow** | Um filete de luz logo abaixo da borda superior. É isso que dá a sensação de volume e de vidro | `inset 0 1px 0 rgba(255,255,255,.14)` + `inset 0 0 12px rgba(154,160,224,.10)` |
| **Outer glow** | Halo muito fraco ao redor; quase não se vê, só se sente | `0 0 24px rgba(154,160,224,.10)` |
| **Texto** | Branco quente, semibold, centralizado, com respiro generoso | Geologica 600, `#F4F5FF` |
| **Raio** | Arredondado médio, sem chegar a pílula | `12px` (controles), `20px` (cards), `28px` (painéis) |
| **Proporção** | Padding horizontal ≈ 2,5× o vertical; muito ar | Botão médio: `44px` de altura, `28px` de padding lateral |

O efeito tem que ser **discreto**. A luz só sugere o material. Se o glow chamar mais atenção que o conteúdo, está forte demais.

---

## 2. Regras (obrigatórias)

1. **Tudo tem glow.** Qualquer superfície elevada (botão, card, input, menu, modal, toast, tooltip, linha selecionada) tem preenchimento em gradiente, borda luminosa de 1px, inner glow no topo e um outer glow quase imperceptível.
2. **A luz é lavanda.** Reflexos, bordas e halos usam `#9AA0E0` (ou branco em baixa opacidade). A lima nunca é usada como luz ambiente.
3. **A lima é energia, só 10%.** A lima `#E6FF00` aparece só na ação principal da tela, no estado ativo/foco e em badges de status positivo. Quando aparece, ganha um glow lima próprio. **Uma única ação lima por tela.**
4. **Hierarquia por intensidade de luz, não por cor.** Elemento em repouso = borda fraca. Hover = borda e glow mais fortes. Ativo/foco = anel lima. Desabilitado = sem glow.
5. **Composição organizada.** Grid de 8px, alinhamentos rígidos, gutters iguais e nada torto de propósito (simetria = estável, não espelhado). O premium vem da ordem e não de enfeite.
6. **Profundidade em 3 níveis, no máximo:** página (`void`), superfície (card) e elevado (menu/modal). Cada nível sobe um pouco o brilho da borda.
7. **Tipografia do sistema:** Tektur para títulos e marca, Geologica para a interface, Martian Mono para dados, IDs, SQL e rótulos técnicos.
8. **Movimento suave.** Transições de 160–220ms em `ease-out`. No hover a borda "acende" e não pula. Nada de bounce.
9. **Acessibilidade acima do efeito.** O texto mantém contraste AA. O glow nunca substitui o anel de foco: o foco é sempre visível em lima.

---

## 3. Tokens

```css
:root {
  /* cores */
  --mb-void:      #0A0B1F;  /* fundo da página */
  --mb-base:      #12133A;  /* índigo — 60% */
  --mb-raise:     #1C1E52;  /* topo do gradiente das superfícies */
  --mb-line:      #2A2E6A;  /* divisores */
  --mb-lavender:  #9AA0E0;  /* luz — 30% */
  --mb-lime:      #E6FF00;  /* acento — 10% */
  --mb-text:      #F4F5FF;
  --mb-text-dim:  #9AA0E0;

  /* superfícies com glow */
  --mb-fill:       linear-gradient(180deg, var(--mb-raise) 0%, var(--mb-base) 100%);
  --mb-edge:       conic-gradient(from 135deg, rgba(154,160,224,.70), rgba(154,160,224,.08) 25%,
                                  rgba(154,160,224,.55) 50%, rgba(154,160,224,.08) 75%, rgba(154,160,224,.70));
  --mb-inner:      inset 0 1px 0 rgba(255,255,255,.14), inset 0 0 12px rgba(154,160,224,.10);
  --mb-halo:       0 0 24px rgba(154,160,224,.10);
  --mb-halo-hover: 0 0 32px rgba(154,160,224,.22);
  --mb-halo-lime:  0 0 28px rgba(230,255,0,.28);

  /* raios */
  --mb-r-control: 12px;
  --mb-r-card:    20px;
  --mb-r-panel:   28px;
}

/* receita base: borda em gradiente + inner glow + halo */
.mb-glass {
  background: var(--mb-fill) padding-box, var(--mb-edge) border-box;
  border: 1px solid transparent;
  box-shadow: var(--mb-inner), var(--mb-halo);
  transition: box-shadow .2s ease-out, filter .2s ease-out;
}
.mb-glass:hover { box-shadow: var(--mb-inner), var(--mb-halo-hover); filter: brightness(1.06); }
.mb-glass:focus-visible { outline: 2px solid var(--mb-lime); outline-offset: 2px; }

/* ação principal */
.mb-primary {
  background: linear-gradient(180deg, #F0FF5C 0%, var(--mb-lime) 100%);
  color: var(--mb-base);
  box-shadow: inset 0 1px 0 rgba(255,255,255,.6), var(--mb-halo-lime);
}
```

### Sem perolado nos cards

O perolado fica no ícone e nas peças de marca. Cards, caixas e painéis da interface usam só o vidro da referência (`.mb-glass`). Testamos textura perolada no card do login e na logo do topo, e a ideia foi descartada.

### Brilho que passa (shimmer, opcional)

Se em algum momento quisermos movimento numa superfície de vidro, a faixa de luz fica assim no código:

```css
.mb-pearl { position: relative; overflow: hidden; }
.mb-pearl::after {
  content: ""; position: absolute; inset: 0 auto 0 -40%; width: 30%;
  background: linear-gradient(105deg, transparent 0%, rgba(255,255,255,.10) 45%,
              rgba(230,220,255,.16) 50%, rgba(255,255,255,.10) 55%, transparent 100%);
  animation: mb-shimmer 6s ease-in-out infinite;
  pointer-events: none;
}
@keyframes mb-shimmer {
  0%, 70% { transform: translateX(0); }
  100%    { transform: translateX(480%); }
}
@media (prefers-reduced-motion: reduce) { .mb-pearl::after { animation: none; opacity: 0; } }
```

A faixa passa em cerca de 1,8s e depois fica uns 4s parada, para não cansar a vista. Quem tem "reduzir movimento" ligado não vê a animação.

---

## 4. Componentes base

| Componente | Variantes |
|---|---|
| Botão | primário (lima), secundário (vidro), fantasma (só texto + borda no hover), perigo (vidro com borda vermelha) |
| Input / Select / Busca | vidro rebaixado (inner glow mais forte); no foco, borda lima |
| Card | vidro, raio 20, cabeçalho opcional com rótulo em Martian Mono |
| Tabela (grade de dados) | linhas sem glow individual; a **linha selecionada** acende; cabeçalho em Martian Mono |
| Navegação lateral | item ativo = vidro aceso + barra lima de 2px |
| Modal / Painel lateral | nível "elevado", borda mais brilhante e halo maior |
| Toast / Tooltip | vidro compacto |
| Badge de status | ok = lima, aviso = âmbar, erro = vermelho, neutro = vidro |
| Estado vazio | ícone perolado + uma frase + uma ação lima |

---

## 5. Mapa do produto

O Masterbase é um Postgres, hospedado ou self-hosted, que **quem não sabe SQL consegue usar**. Toda tela responde à pergunta "o que eu quero fazer com meus dados?", não "qual comando SQL eu rodo?".

### A. Site público (marketing)
1. **Home:** promessa ("Seu banco. Sua casa. Sua chave."), demonstração do editor visual, hospedado × self-hosted, chamada para começar.
2. **Preços:** Grátis / Pro / Equipe para o hospedado; o self-hosted é gratuito e pode ter suporte pago.
3. **Self-hosted:** instalação em um comando (Docker), requisitos e como atualizar.
4. **Segurança:** criptografia, backups, controle de acesso e onde os dados ficam.
5. **Documentação:** guias para quem não programa e referência de API para quem programa.
6. **Entrar / Criar conta.**

### B. Entrada no app
7. **Login / Cadastro** (incluindo login no servidor self-hosted).
8. **Onboarding:** escolher hospedado ou meu servidor → nome do projeto → região → "importar planilha" ou "começar do zero" ou "conectar Postgres existente".
9. **Seletor de projetos:** cards de vidro, um por banco, com status e uso.

### C. Dentro de um projeto (o coração)
Navegação lateral fixa, nesta ordem:

| # | Página | O que faz (sem SQL) |
|---|---|---|
| 10 | **Início** | Saúde do banco, uso, últimas alterações, atalhos ("criar tabela", "importar CSV") |
| 11 | **Tabelas** | Editor tipo planilha: ver, filtrar, ordenar, editar linhas e colar dados |
| 12 | **Estrutura** | Criar e editar tabelas e colunas visualmente (tipo, obrigatório, padrão, único) com linguagem humana ("Texto", "Número", "Data", "Sim/Não", "Ligação com…") |
| 13 | **Relações** | Diagrama visual das tabelas, ligado arrastando de uma tabela à outra |
| 14 | **Consultas** | Construtor visual ("mostrar *clientes* onde *plano* é *pro*"), consultas salvas, gráfico rápido do resultado e a aba **SQL** para quem quiser |
| 15 | **Acesso** | Usuários, papéis e regras de permissão por tabela em linguagem simples ("Vendedores só veem os próprios pedidos"), que viram RLS por baixo |
| 16 | **API & Chaves** | API REST/GraphQL gerada automaticamente, chaves (com revelação protegida), trechos de código prontos |
| 17 | **Arquivos** | Armazenamento de arquivos e imagens ligados às linhas (opcional na v1) |
| 18 | **Backups** | Linha do tempo de backups, restaurar para um momento, exportar |
| 19 | **Atividade** | Auditoria: quem mudou o quê e quando, com filtro por tabela e usuário |
| 20 | **Configurações** | Nome, conexão (string Postgres), domínio, membros da equipe, cobrança (hospedado) |

### D. Só no self-hosted
21. **Servidor:** CPU, memória, disco, versão, atualizações disponíveis com um clique, logs do container.
22. **Administração:** contas da instância, SMTP, SSO e certificados.

### E. Transversais
- **Busca/comandos (Ctrl+K):** vidro elevado, para pular para qualquer tabela ou ação.
- **Estados vazios, de carregamento e de erro** para cada página.
- **Assistente de importação:** CSV, Excel ou Google Sheets → detecção automática de tipos.

---

## 6. Ordem sugerida para desenhar

1. Kit de componentes com glow (botões, inputs, cards, nav, tabela) — trava o acabamento.
2. Shell do app (nav lateral + topo + Ctrl+K).
3. **Tabelas** (editor planilha) — a tela que mais vende o produto.
4. **Estrutura** e **Relações**.
5. **Consultas** (construtor visual).
6. Onboarding e seletor de projetos.
7. Acesso, Backups, Atividade e Configurações.
8. Home do site público.
