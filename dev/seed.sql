-- Dados de exemplo (Padaria Aurora). Só para desenvolvimento.
CREATE DOMAIN email AS text CHECK (VALUE ~ '^[^@\s]+@[^@\s]+\.[^@\s]+$');
CREATE TYPE plano AS ENUM ('grátis', 'pro');

CREATE TABLE clientes (
    id        integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    nome      text NOT NULL,
    "e-mail"  email,
    plano     plano NOT NULL DEFAULT 'grátis',
    cidade    text,
    criado_em date NOT NULL DEFAULT current_date
);

INSERT INTO clientes (nome, "e-mail", plano, cidade, criado_em) VALUES
('Café Bom Dia','contato@cafebomdia.com','pro','Curitiba','2026-03-12'),
('Mercearia Sol','compras@merceariasol.com','pro','Porto Alegre','2026-03-28'),
('Oficina Ribeiro','oficina@ribeiro.com','grátis','Caxias do Sul','2026-03-30'),
('Studio Lume','oi@studiolume.com.br','pro','Florianópolis','2026-04-02'),
('Hotel Brisa','reservas@hotelbrisa.com','pro','Salvador','2026-04-19'),
('Ateliê Nanquim','ola@nanquim.art','pro','Recife','2026-05-07'),
('Bistrô Quintal','pedidos@bistroquintal.com','pro','Belo Horizonte','2026-05-21'),
('Barbearia Navalha','agenda@navalha.com','grátis','Goiânia','2026-05-25'),
('Clínica Vida','adm@clinicavida.med.br','pro','Campinas','2026-06-03'),
('Livraria Porto','loja@livrariaporto.com','pro','Santos','2026-06-18'),
('Floricultura Ipê','vendas@floripe.com','grátis','Brasília','2026-06-27'),
('Empório Serra','contato@emporioserra.com','pro','Gramado','2026-07-09'),
('Pet Shop Faro','faro@petshopfaro.com','pro','Niterói','2026-07-30'),
('Escola Ciranda','secretaria@ciranda.edu.br','pro','Londrina','2026-08-14'),
('Açougue Central','central@acouguecentral.com','pro','Joinville','2026-09-02'),
('Lavanderia Nuvem','oi@lavanuvem.com','grátis','Fortaleza','2026-09-15');

CREATE TABLE produtos (
    id    integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    nome  text NOT NULL,
    preco numeric(10,2) NOT NULL CHECK (preco >= 0),
    ativo boolean NOT NULL DEFAULT true
);
INSERT INTO produtos (nome, preco) VALUES
('Pão francês (kg)', 18.90), ('Croissant', 9.50), ('Bolo de fubá', 32.00), ('Café coado 300 ml', 7.00), ('Pão de queijo (un)', 4.50);

CREATE TABLE pedidos (
    id         integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    cliente_id integer NOT NULL REFERENCES clientes(id),
    total      numeric(10,2) NOT NULL,
    feito_em   timestamptz NOT NULL DEFAULT now()
);
INSERT INTO pedidos (cliente_id, total, feito_em)
SELECT 1 + (g % 16), round((20 + random() * 300)::numeric, 2), now() - (g || ' hours')::interval
FROM generate_series(1, 120) g;

CREATE TABLE fornecedores (
    id      integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    nome    text NOT NULL,
    contato email
);
INSERT INTO fornecedores (nome, contato) VALUES
('Moinho Paraná', 'vendas@moinhopr.com'), ('Laticínios Serra Azul', 'pedidos@serraazul.com'), ('Café Mantiqueira', 'contato@mantiqueira.com');
