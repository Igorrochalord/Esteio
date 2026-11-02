import { useState } from "react";

import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { auditApi, usersApi } from "../services/endpoints";
import { ACTION_LABEL, FIELD_LABEL, formatDateTime, formatValue } from "../services/labels";

const PAGE_SIZE = 25;

function describeChanges(before: Record<string, unknown> | null, after: Record<string, unknown> | null) {
  if (!after) return "";
  return Object.entries(after)
    .filter(([k]) => k in FIELD_LABEL)
    .map(([k, v]) =>
      before && k in before
        ? `${FIELD_LABEL[k]}: ${formatValue(k, before[k])} → ${formatValue(k, v)}`
        : `${FIELD_LABEL[k]}: ${formatValue(k, v)}`,
    )
    .join("; ");
}

export function AuditPage() {
  const { can } = useAuth();
  const [offset, setOffset] = useState(0);
  const { data, error, loading } = useAsync(() => auditApi.list({ limit: PAGE_SIZE, offset }), [offset]);
  const { data: users } = useAsync(() => (can("users") ? usersApi.list() : Promise.resolve([])), []);
  const names = new Map(users?.map((u) => [u.id, u.name]));

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Histórico</h1>
          <p className="muted">Tudo o que foi alterado na empresa, por quem e quando.</p>
        </div>
      </header>
      {error && <Notice tone="error">{error}</Notice>}
      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : data && data.items.length === 0 ? (
        <p className="muted">Nada registrado ainda.</p>
      ) : (
        <>
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Quando</th>
                  <th>Quem</th>
                  <th>O quê</th>
                  <th>Detalhes</th>
                </tr>
              </thead>
              <tbody>
                {data?.items.map((i) => (
                  <tr key={i.id}>
                    <td className="num nowrap">{formatDateTime(i.created_at)}</td>
                    <td>{(i.user_id && names.get(i.user_id)) || "—"}</td>
                    <td>{ACTION_LABEL[i.action] ?? i.action}</td>
                    <td className="details">{describeChanges(i.before, i.after)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {data && (
            <div className="pager">
              <span className="muted num">
                {data.total === 0 ? 0 : offset + 1}–{Math.min(offset + PAGE_SIZE, data.total)} de {data.total}
              </span>
              <button className="btn btn--ghost" disabled={offset === 0}
                onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>Anteriores</button>
              <button className="btn btn--ghost" disabled={offset + PAGE_SIZE >= data.total}
                onClick={() => setOffset(offset + PAGE_SIZE)}>Próximos</button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
