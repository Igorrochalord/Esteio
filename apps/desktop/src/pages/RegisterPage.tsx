import { useState, type FormEvent } from "react";
import { Link, useNavigate } from "react-router";

import { AuthFrame } from "../components/AuthFrame";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { PasswordField } from "../components/PasswordField";
import { ApiError } from "../services/api";
import { authApi } from "../services/endpoints";

/** Pedido de acesso: a conta só funciona depois que um administrador aprovar. */
export function RegisterPage() {
  const navigate = useNavigate();
  const [form, setForm] = useState({ name: "", email: "", password: "", confirm: "" });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);

  const set = (key: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm({ ...form, [key]: e.target.value });

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (form.password !== form.confirm) {
      setFields({ confirm: "As senhas não conferem." });
      return;
    }
    setBusy(true);
    setFields({});
    try {
      await authApi.register({ name: form.name, email: form.email, password: form.password });
      navigate("/login", { replace: true, state: { registered: { email: form.email.trim() } } });
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
        setFields(err.fields);
      } else {
        setError("Não foi possível enviar o pedido.");
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <AuthFrame>
      <header className="auth-header">
        <h1 className="auth-title">Pedir cadastro</h1>
        <p className="muted">
          Preencha seus dados. Um administrador da empresa vai aprovar o seu acesso e definir o que
          você pode usar.
        </p>
      </header>

      {error && <Notice tone="error">{error}</Notice>}

      <form onSubmit={submit} className="stack">
        <Field label="Nome completo" autoComplete="name" required minLength={2} autoFocus
          value={form.name} error={fields.name} onChange={set("name")} />
        <Field label="E-mail" type="email" autoComplete="email" required
          value={form.email} error={fields.email} onChange={set("email")} />
        <PasswordField label="Crie uma senha" autoComplete="new-password" required minLength={8}
          hint="Mínimo de 8 caracteres." value={form.password} error={fields.password}
          onChange={set("password")} />
        <PasswordField label="Repita a senha" autoComplete="new-password" required
          value={form.confirm} error={fields.confirm} onChange={set("confirm")} />
        <button className="btn btn--primary btn--block btn--large" disabled={busy}>
          {busy ? "Enviando…" : "Enviar pedido"}
        </button>
      </form>

      <div className="auth-links">
        <p>
          Já tem acesso? <Link to="/login">Entrar</Link>
        </p>
      </div>
    </AuthFrame>
  );
}
