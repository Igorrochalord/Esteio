import { useState, type FormEvent } from "react";

import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { usersApi } from "../services/endpoints";
import { formatDateTime, ROLE_LABEL } from "../services/labels";
import type { Role, User } from "../services/types";

type AssignableRole = Exclude<Role, "owner">;

export function UsersPage() {
  const { me, can } = useAuth();
  const canManage = can("users", "manage");
  const { data: users, error, loading, reload } = useAsync(usersApi.list);
  const [creating, setCreating] = useState(false);
  const [resetting, setResetting] = useState<User | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  async function act(action: () => Promise<unknown>, done: string) {
    setActionError(null);
    setNotice(null);
    try {
      await action();
      setNotice(done);
      await reload();
    } catch (e) {
      setActionError(e instanceof ApiError ? e.message : "Não foi possível salvar.");
    }
  }

  const change = (user: User, patch: { role?: AssignableRole; is_active?: boolean }, done: string) =>
    act(() => usersApi.update(user.id, patch), done);

  const pending = users?.filter((u) => u.pending) ?? [];
  const members = users?.filter((u) => !u.pending) ?? [];
  const canReset = (u: User) =>
    canManage && u.id !== me?.user.id && (u.role !== "owner" || me?.user.role === "owner");

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Pessoas</h1>
          <p className="muted">Quem acessa o sistema e o que cada um pode fazer.</p>
        </div>
        {canManage && (
          <button className="btn btn--primary" onClick={() => setCreating(true)}>
            Adicionar pessoa
          </button>
        )}
      </header>

      {(error || actionError) && <Notice tone="error">{error ?? actionError}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      {canManage && pending.length > 0 && (
        <section className="panel panel--attention" aria-labelledby="pending-title">
          <h2 id="pending-title">Aguardando aprovação ({pending.length})</h2>
          <p className="muted">
            Estas pessoas pediram acesso pela tela de entrada. Escolha o papel e aprove, ou recuse.
          </p>
          <ul className="pending-list">
            {pending.map((u) => (
              <PendingRow key={u.id} user={u}
                onApprove={(role) => act(() => usersApi.approve(u.id, role),
                  `${u.name} foi aprovado como ${ROLE_LABEL[role].toLowerCase()}.`)}
                onReject={() => act(() => usersApi.reject(u.id), `O pedido de ${u.name} foi recusado.`)} />
            ))}
          </ul>
        </section>
      )}

      {loading && !users ? (
        <p className="muted">Carregando…</p>
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>Nome</th>
                <th>E-mail</th>
                <th>Papel</th>
                <th>Último acesso</th>
                <th>Situação</th>
                {canManage && <th aria-label="Ações" />}
              </tr>
            </thead>
            <tbody>
              {members.map((u) => {
                const locked = u.role === "owner" || u.id === me?.user.id || !canManage;
                return (
                  <tr key={u.id} className={u.is_active ? "" : "is-inactive"}>
                    <td>{u.name}</td>
                    <td>{u.email}</td>
                    <td>
                      {locked ? (
                        ROLE_LABEL[u.role]
                      ) : (
                        <select aria-label={`Papel de ${u.name}`} value={u.role}
                          onChange={(e) => void change(u, { role: e.target.value as AssignableRole },
                            `${u.name} agora é ${ROLE_LABEL[e.target.value as Role].toLowerCase()}.`)}>
                          <option value="admin">{ROLE_LABEL.admin}</option>
                          <option value="user">{ROLE_LABEL.user}</option>
                        </select>
                      )}
                    </td>
                    <td className="num">{formatDateTime(u.last_login_at)}</td>
                    <td>
                      <span className={`status ${u.is_active ? "status--on" : "status--off"}`}>
                        {u.is_active ? "Ativo" : "Desativado"}
                      </span>
                    </td>
                    {canManage && (
                      <td className="actions">
                        {canReset(u) && (
                          <button className="btn btn--ghost" onClick={() => setResetting(u)}>
                            Redefinir senha
                          </button>
                        )}
                        {!locked && (
                          <button className="btn btn--ghost"
                            onClick={() => void change(u, { is_active: !u.is_active },
                              u.is_active ? `${u.name} foi desativado.` : `${u.name} foi reativado.`)}>
                            {u.is_active ? "Desativar" : "Reativar"}
                          </button>
                        )}
                      </td>
                    )}
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      <Dialog open={!!resetting} title={`Redefinir senha de ${resetting?.name ?? ""}`}
        onClose={() => setResetting(null)}>
        {resetting && (
          <ResetPasswordForm user={resetting}
            onDone={() => {
              setNotice(`Senha de ${resetting.name} redefinida. Repasse a senha provisória para a pessoa.`);
              setResetting(null);
            }} />
        )}
      </Dialog>

      <Dialog open={creating} title="Adicionar pessoa" onClose={() => setCreating(false)}>
        {creating && (
          <CreateUserForm
            onCreated={async (u) => {
              setCreating(false);
              setNotice(`${u.name} foi adicionado. Envie a senha provisória para o primeiro acesso.`);
              await reload();
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

function CreateUserForm({ onCreated }: { onCreated: (u: User) => void | Promise<void> }) {
  const [form, setForm] = useState({ name: "", email: "", password: "", role: "user" as AssignableRole });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setFields({});
    try {
      await onCreated(await usersApi.create(form));
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
        setFields(err.fields);
      } else setError("Não foi possível adicionar.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <Field label="Nome" required minLength={2} autoFocus value={form.name} error={fields.name}
        onChange={(e) => setForm({ ...form, name: e.target.value })} />
      <Field label="E-mail" type="email" required value={form.email} error={fields.email}
        onChange={(e) => setForm({ ...form, email: e.target.value })} />
      <Field label="Senha provisória" type="text" required minLength={8} autoComplete="off"
        hint="Mínimo de 8 caracteres. Repasse para a pessoa." value={form.password} error={fields.password}
        onChange={(e) => setForm({ ...form, password: e.target.value })} />
      <div className="field">
        <label htmlFor="new-role">Papel</label>
        <select id="new-role" value={form.role}
          onChange={(e) => setForm({ ...form, role: e.target.value as AssignableRole })}>
          <option value="user">{ROLE_LABEL.user}: usa os módulos do dia a dia</option>
          <option value="admin">{ROLE_LABEL.admin}: também gerencia pessoas e a empresa</option>
        </select>
      </div>
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>
          {busy ? "Adicionando…" : "Adicionar pessoa"}
        </button>
      </div>
    </form>
  );
}

function PendingRow({ user, onApprove, onReject }: {
  user: User;
  onApprove: (role: AssignableRole) => Promise<void>;
  onReject: () => Promise<void>;
}) {
  const [role, setRole] = useState<AssignableRole>("user");
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    try {
      await fn();
    } finally {
      setBusy(false);
    }
  };
  return (
    <li className="pending-row">
      <div>
        <strong>{user.name}</strong>
        <span className="muted"> · {user.email} · pedido em {formatDateTime(user.created_at)}</span>
      </div>
      <div className="row">
        <select aria-label={`Papel de ${user.name}`} value={role}
          onChange={(e) => setRole(e.target.value as AssignableRole)}>
          <option value="user">{ROLE_LABEL.user}</option>
          <option value="admin">{ROLE_LABEL.admin}</option>
        </select>
        <button className="btn btn--primary" disabled={busy} onClick={() => void run(() => onApprove(role))}>
          Aprovar
        </button>
        <button className="btn btn--ghost" disabled={busy} onClick={() => void run(onReject)}>
          Recusar
        </button>
      </div>
    </li>
  );
}

function ResetPasswordForm({ user, onDone }: { user: User; onDone: () => void }) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await usersApi.resetPassword(user.id, password);
      onDone();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : "Não foi possível redefinir a senha.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <Field label="Nova senha provisória" type="text" required minLength={8} autoComplete="off" autoFocus
        hint="Mínimo de 8 caracteres. A senha antiga deixa de funcionar." value={password}
        onChange={(e) => setPassword(e.target.value)} />
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>
          {busy ? "Salvando…" : "Redefinir senha"}
        </button>
      </div>
    </form>
  );
}
