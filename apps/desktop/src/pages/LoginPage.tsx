import { useState, type FormEvent } from "react";
import { Link, useLocation, useNavigate } from "react-router";

import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { getApiUrl, isDefaultApiUrl, setApiUrl } from "../services/config";

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await login(email, password);
      const from = (location.state as { from?: { pathname: string } } | null)?.from?.pathname;
      navigate(from ?? "/", { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Não foi possível entrar.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="auth-card">
      <h1 className="auth-title">Entrar</h1>
      <p className="muted">Use o e-mail cadastrado na sua empresa.</p>
      <form onSubmit={submit} className="stack">
        {error && <Notice tone="error">{error}</Notice>}
        <Field label="E-mail" type="email" autoComplete="username" required autoFocus
          value={email} onChange={(e) => setEmail(e.target.value)} />
        <Field label="Senha" type="password" autoComplete="current-password" required
          value={password} onChange={(e) => setPassword(e.target.value)} />
        <button className="btn btn--primary btn--block" disabled={busy}>
          {busy ? "Entrando…" : "Entrar"}
        </button>
      </form>
      <p className="auth-alt">
        Sua empresa ainda não usa o Núcleo? <Link to="/onboarding">Cadastrar empresa</Link>
      </p>
      <ServerSetting />
    </div>
  );
}

/** Permite apontar o app para outro servidor (homologação, on-premise…). */
function ServerSetting() {
  const [editing, setEditing] = useState(false);
  const [url, setUrl] = useState(getApiUrl());

  if (!editing) {
    return (
      <p className="server-line">
        Servidor: <span className="server-url">{getApiUrl()}</span>{" "}
        <button type="button" className="link-btn" onClick={() => setEditing(true)}>
          Alterar
        </button>
      </p>
    );
  }
  return (
    <form
      className="server-form"
      onSubmit={(e) => {
        e.preventDefault();
        setApiUrl(url);
        setEditing(false);
      }}
    >
      <Field label="Endereço do servidor" type="url" required value={url}
        onChange={(e) => setUrl(e.target.value)} placeholder="https://api.suaempresa.com.br" />
      <div className="row">
        <button className="btn btn--primary">Salvar servidor</button>
        {!isDefaultApiUrl() && (
          <button type="button" className="btn btn--ghost"
            onClick={() => { setApiUrl(null); setUrl(getApiUrl()); setEditing(false); }}>
            Usar o padrão
          </button>
        )}
        <button type="button" className="btn btn--ghost" onClick={() => setEditing(false)}>
          Cancelar
        </button>
      </div>
    </form>
  );
}
