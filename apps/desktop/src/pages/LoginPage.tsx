import { useState, type FormEvent } from "react";
import { Link, useLocation, useNavigate } from "react-router";

import { AuthFrame } from "../components/AuthFrame";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { PasswordField } from "../components/PasswordField";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";

// conveniência por computador: lembra o último e-mail usado (nunca a senha)
const LAST_EMAIL_KEY = "nucleo.lastEmail";

function readLastEmail(): string {
  try {
    return localStorage.getItem(LAST_EMAIL_KEY) ?? "";
  } catch {
    return "";
  }
}

function saveLastEmail(email: string, remember: boolean) {
  try {
    if (remember) localStorage.setItem(LAST_EMAIL_KEY, email.trim());
    else localStorage.removeItem(LAST_EMAIL_KEY);
  } catch {
    /* sem armazenamento: segue sem lembrar */
  }
}

interface LoginState {
  from?: { pathname: string };
  registered?: { email: string };
}

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const state = (useLocation().state as LoginState | null) ?? {};
  const [email, setEmail] = useState(state.registered?.email ?? readLastEmail());
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(readLastEmail() !== "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await login(email, password);
      saveLastEmail(email, remember);
      navigate(state.from?.pathname ?? "/", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Não foi possível entrar.");
      setPassword("");
    } finally {
      setBusy(false);
    }
  }

  return (
    <AuthFrame>
      <header className="auth-header">
        <h1 className="auth-title">Entrar</h1>
        <p className="muted">Use o e-mail e a senha cadastrados neste computador.</p>
      </header>

      {state.registered && !error && (
        <Notice tone="success">
          Pedido de acesso enviado. Assim que um administrador aprovar, você já pode entrar com a
          senha que criou.
        </Notice>
      )}
      {error && <Notice tone="error">{error}</Notice>}

      <form onSubmit={submit} className="stack">
        <Field label="E-mail" type="email" autoComplete="username" required
          autoFocus={!email} value={email} onChange={(e) => setEmail(e.target.value)} />
        <PasswordField label="Senha" autoComplete="current-password" required
          autoFocus={!!email} value={password} onChange={(e) => setPassword(e.target.value)} />
        <label className="check">
          <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
          Lembrar meu e-mail neste computador
        </label>
        <button className="btn btn--primary btn--block btn--large" disabled={busy}>
          {busy ? "Entrando…" : "Entrar"}
        </button>
      </form>

      <div className="auth-links">
        <p>
          Ainda não tem acesso? <Link to="/cadastro">Pedir cadastro</Link>
        </p>
        <p className="muted">
          Esqueceu a senha? Peça a um administrador para redefinir em <strong>Pessoas</strong>.
        </p>
      </div>
    </AuthFrame>
  );
}
