// Quem está logado, visível para todas as páginas. O servidor confere tudo de novo; isto só esconde botão.
export const session = $state({ user: null });
export const canEdit = () => session.user?.role !== 'leitor';
export const isAdmin = () => session.user?.role === 'admin';
