// Passkeys: conversa com o navegador (navigator.credentials) e troca os binários por base64url para a API.
const b64 = (buf) => btoa(String.fromCharCode(...new Uint8Array(buf))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
const unb64 = (s) => Uint8Array.from(atob(s.replace(/-/g, '+').replace(/_/g, '/') + '='.repeat((4 - (s.length % 4)) % 4)), (c) => c.charCodeAt(0));

export const passkeySupported = () => typeof window !== 'undefined' && !!window.PublicKeyCredential;

export async function createPasskey(o) {
  const cred = await navigator.credentials.create({
    publicKey: {
      ...o,
      challenge: unb64(o.challenge),
      user: { ...o.user, id: unb64(o.user.id) },
      excludeCredentials: o.excludeCredentials.map((c) => ({ ...c, id: unb64(c.id) })),
    },
  });
  return { clientDataJSON: b64(cred.response.clientDataJSON), attestationObject: b64(cred.response.attestationObject) };
}

export async function getPasskey(o) {
  const cred = await navigator.credentials.get({ publicKey: { ...o, challenge: unb64(o.challenge) } });
  return {
    id: cred.id,
    clientDataJSON: b64(cred.response.clientDataJSON),
    authenticatorData: b64(cred.response.authenticatorData),
    signature: b64(cred.response.signature),
  };
}

// Mensagem humana para o erro do navegador (cancelou, passou o tempo, aparelho sem suporte).
export function passkeyError(e) {
  if (e?.name === 'NotAllowedError') return 'Cancelado ou o tempo acabou. Tente de novo.';
  if (e?.name === 'InvalidStateError') return 'Este aparelho já tem uma passkey desta conta.';
  if (e?.name === 'SecurityError') return 'Passkey só funciona no endereço oficial do painel (com HTTPS).';
  return e?.message || 'Não deu certo com a passkey.';
}
