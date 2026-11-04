import { ask } from "@tauri-apps/plugin-dialog";
import {
  CheckCircle2,
  CircleAlert,
  Clock,
  FileText,
  Plug,
  QrCode,
  RefreshCw,
  Send,
  Store,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";

import { StatTile } from "../components/charts";
import { ConnectionPill } from "../components/ConnectionStatus";
import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { integrationsApi } from "../services/endpoints";
import { formatDateTime } from "../services/labels";
import type { IntegrationCatalogItem, IntegrationKind, JobStatus } from "../services/types";

const KIND_ICON: Record<IntegrationKind, LucideIcon> = {
  pix: QrCode,
  nfse: FileText,
  marketplace: Store,
  other: Plug,
};

const JOB_STATUS: Record<JobStatus, { label: string; tone: string }> = {
  pending: { label: "Na fila", tone: "tag--warn" },
  running: { label: "Enviando", tone: "tag--info" },
  done: { label: "Concluído", tone: "tag--ok" },
  failed: { label: "Falhou", tone: "tag--danger" },
  canceled: { label: "Cancelado", tone: "" },
};

const FILTERS: { value: JobStatus | ""; label: string }[] = [
  { value: "", label: "Todos" },
  { value: "pending", label: "Na fila" },
  { value: "failed", label: "Com falha" },
  { value: "done", label: "Concluídos" },
];

function statusOf(item: IntegrationCatalogItem): { label: string; tone: string } {
  const i = item.integration;
  if (!item.available) return { label: `Em breve${item.phase ? ` · fase ${item.phase}` : ""}`, tone: "" };
  if (!i) return { label: "Não configurada", tone: "" };
  if (!i.secrets_readable) return { label: "Informe as credenciais de novo", tone: "tag--warn" };
  if (!i.enabled) return { label: "Desativada", tone: "" };
  if (i.status === "ok") return { label: "Conectada", tone: "tag--ok" };
  if (i.status === "error") return { label: "Com erro", tone: "tag--danger" };
  return { label: "Ainda não testada", tone: "tag--warn" };
}

function errorText(e: unknown, fallback: string) {
  return e instanceof ApiError ? e.message : fallback;
}

export function IntegrationsPage() {
  const { can } = useAuth();
  const manage = can("integrations", "manage");
  const catalog = useAsync(integrationsApi.catalog);
  const summary = useAsync(integrationsApi.summary);
  const [filter, setFilter] = useState<JobStatus | "">("");
  const jobs = useAsync(() => integrationsApi.jobs(filter), [filter]);
  const [editing, setEditing] = useState<IntegrationCatalogItem | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const reloadAll = async () => {
    await Promise.all([catalog.reload(), summary.reload(), jobs.reload()]);
  };

  // a fila anda sozinha em segundo plano: atualiza a tela de tempos em tempos
  useEffect(() => {
    const t = window.setInterval(() => {
      void summary.reload();
      void jobs.reload();
    }, 20_000);
    return () => window.clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filter]);

  async function run(action: () => Promise<string | null>, fallback: string) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const done = await action();
      if (done) setNotice(done);
      await reloadAll();
    } catch (e) {
      setError(errorText(e, fallback));
    } finally {
      setBusy(false);
    }
  }

  const testConnection = (provider: string) =>
    run(async () => {
      const out = await integrationsApi.test(provider);
      if (out.status === "ok") return `Conexão funcionando. ${out.status_detail ?? ""}`.trim();
      setError(out.status_detail ?? "A conexão não funcionou.");
      return null;
    }, "Não foi possível testar a conexão.");

  const runNow = () =>
    run(async () => {
      const r = await integrationsApi.runNow();
      if (r.ran === 0) return "Nada pendente para enviar agora.";
      const parts = [
        r.done && `${r.done} concluído(s)`,
        r.waiting && `${r.waiting} aguardando`,
        r.retrying && `${r.retrying} sem resposta (vamos tentar de novo)`,
        r.failed && `${r.failed} com falha`,
      ].filter(Boolean);
      return `Fila processada: ${parts.join(", ")}.`;
    }, "Não foi possível processar a fila.");

  const s = summary.data;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Integrações</h1>
          <p className="muted">
            Conecte o sistema a serviços on-line (PIX, nota fiscal, lojas virtuais). Sem internet, tudo continua
            funcionando: o que precisa ser enviado espera na fila e segue sozinho quando a conexão voltar.
          </p>
        </div>
        <div className="row">
          <ConnectionPill />
          {manage && (
            <button className="btn btn--ghost" disabled={busy} onClick={() => void runNow()}>
              <Send size={16} aria-hidden /> Enviar agora
            </button>
          )}
        </div>
      </header>

      {error && <Notice tone="error">{error}</Notice>}
      {catalog.error && <Notice tone="error">{catalog.error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      <div className="stats">
        <StatTile label="Na fila" icon={<Clock size={15} aria-hidden />} value={String(s ? s.pending + s.running : "—")}
          hint="aguardando internet ou resposta" />
        <StatTile label="Com falha" icon={<CircleAlert size={15} aria-hidden />} value={String(s?.failed ?? "—")}
          tone={s && s.failed > 0 ? "warn" : undefined} hint={s && s.failed > 0 ? "precisa da sua atenção" : "tudo certo"} />
        <StatTile label="Integrações ativas" icon={<Plug size={15} aria-hidden />} value={String(s?.enabled_integrations ?? "—")} />
        <StatTile label="Último envio concluído" icon={<CheckCircle2 size={15} aria-hidden />}
          value={s?.last_success_at ? formatDateTime(s.last_success_at) : "—"} />
      </div>

      <section className="stack">
        <h2>Serviços</h2>
        <div className="integration-cards">
          {catalog.data?.map((item) => {
            const Icon = KIND_ICON[item.kind];
            const st = statusOf(item);
            return (
              <article key={item.id} className={item.available ? "integration-card" : "integration-card is-soon"}>
                <div className="integration-card-top">
                  <span className="quick-icon" aria-hidden>
                    <Icon size={20} />
                  </span>
                  <span className={`tag ${st.tone}`}>{st.label}</span>
                </div>
                <h3>{item.name}</h3>
                <p className="muted small">{item.description}</p>
                {item.integration?.status_detail && (
                  <p className="small">{item.integration.status_detail}</p>
                )}
                {item.available && manage && (
                  <div className="row">
                    <button className="btn btn--primary" disabled={busy} onClick={() => setEditing(item)}>
                      {item.integration ? "Configurar" : "Conectar"}
                    </button>
                    {item.integration && (
                      <button className="btn btn--ghost" disabled={busy} onClick={() => void testConnection(item.id)}>
                        <RefreshCw size={15} aria-hidden /> Testar conexão
                      </button>
                    )}
                  </div>
                )}
              </article>
            );
          })}
        </div>
      </section>

      <section className="stack">
        <div className="row row--fill">
          <h2>Fila de envios</h2>
          <div className="tabs" role="tablist" aria-label="Filtrar fila">
            {FILTERS.map((f) => (
              <button key={f.value} role="tab" aria-selected={filter === f.value}
                className={filter === f.value ? "tab is-on" : "tab"} onClick={() => setFilter(f.value)}>
                {f.label}
              </button>
            ))}
          </div>
        </div>
        {jobs.data && jobs.data.length === 0 ? (
          <p className="muted">Nada por aqui. Quando uma venda, nota ou pedido precisar falar com um serviço on-line, aparece nesta lista.</p>
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>Quando</th>
                  <th>Serviço</th>
                  <th>O quê</th>
                  <th>Situação</th>
                  <th>Detalhe</th>
                  {manage && <th className="actions" aria-label="Ações" />}
                </tr>
              </thead>
              <tbody>
                {jobs.data?.map((j) => {
                  const st = JOB_STATUS[j.status];
                  const name = catalog.data?.find((c) => c.id === j.provider)?.name ?? j.provider;
                  return (
                    <tr key={j.id}>
                      <td className="num nowrap">{formatDateTime(j.created_at)}</td>
                      <td>{name}</td>
                      <td>{j.kind}</td>
                      <td>
                        <span className={`tag ${st.tone}`}>{st.label}</span>
                      </td>
                      <td className="small">
                        {j.last_error ?? ""}
                        {j.status === "pending" && j.attempts > 0 && (
                          <span className="muted"> · nova tentativa {formatDateTime(j.next_run_at)}</span>
                        )}
                      </td>
                      {manage && (
                        <td className="actions nowrap">
                          {j.status === "failed" && (
                            <button className="btn btn--ghost" disabled={busy}
                              onClick={() => void run(async () => { await integrationsApi.retry(j.id); return "Voltou para a fila."; }, "Não foi possível repetir.")}>
                              Tentar de novo
                            </button>
                          )}
                          {(j.status === "failed" || j.status === "pending") && (
                            <button className="btn btn--ghost" disabled={busy}
                              onClick={() => void run(async () => { await integrationsApi.cancel(j.id); return "Envio cancelado."; }, "Não foi possível cancelar.")}>
                              Cancelar
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
      </section>

      {editing && (
        <ConfigureDialog
          item={editing}
          onClose={() => setEditing(null)}
          onSaved={async (message) => {
            setEditing(null);
            setNotice(message);
            await reloadAll();
          }}
        />
      )}
    </div>
  );
}

function ConfigureDialog({
  item,
  onClose,
  onSaved,
}: {
  item: IntegrationCatalogItem;
  onClose: () => void;
  onSaved: (message: string) => Promise<void>;
}) {
  const saved = item.integration;
  const [environment, setEnvironment] = useState<"sandbox" | "production">(saved?.environment ?? "sandbox");
  const [enabled, setEnabled] = useState(saved?.enabled ?? true);
  const [config, setConfig] = useState<Record<string, string>>(() =>
    Object.fromEntries(item.fields.filter((f) => !f.secret).map((f) => [f.key, String(saved?.config[f.key] ?? "")])),
  );
  const [secrets, setSecrets] = useState<Record<string, string>>({});
  const [fields, setFields] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const stored = saved?.secrets_readable ? new Set(saved.secrets_set) : new Set<string>();

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    setFields({});
    try {
      await integrationsApi.save({ provider: item.id, environment, enabled, config, secrets });
      let message = "Integração salva.";
      if (enabled) {
        const out = await integrationsApi.test(item.id);
        message = out.status === "ok"
          ? `Integração salva e conectada. ${out.status_detail ?? ""}`.trim()
          : `Integração salva, mas a conexão falhou: ${out.status_detail ?? "confira os dados"}`;
      }
      await onSaved(message);
    } catch (err) {
      if (err instanceof ApiError) {
        setFields(err.fields);
        setError(err.message);
      } else {
        setError("Não foi possível salvar.");
      }
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    const ok = await ask(`Remover a integração "${item.name}"? As credenciais são apagadas e os envios pendentes, cancelados.`, {
      title: "Remover integração",
      kind: "warning",
      okLabel: "Remover",
      cancelLabel: "Cancelar",
    });
    if (!ok) return;
    setBusy(true);
    try {
      await integrationsApi.remove(item.id);
      await onSaved("Integração removida.");
    } catch (err) {
      setError(errorText(err, "Não foi possível remover."));
      setBusy(false);
    }
  }

  return (
    <Dialog open title={item.name} onClose={onClose}>
      <form className="stack" onSubmit={(e) => void submit(e)}>
        {error && <Notice tone="error">{error}</Notice>}
        {saved && !saved.secrets_readable && (
          <p className="alert alert--warn">
            Os dados de acesso foram salvos em outro computador (por exemplo, numa cópia de segurança restaurada).
            Por segurança eles não viajam junto: informe-os de novo.
          </p>
        )}
        <div className="segmented" role="radiogroup" aria-label="Ambiente">
          {(["sandbox", "production"] as const).map((env) => (
            <label key={env} className={environment === env ? "is-on" : undefined}>
              <input type="radio" checked={environment === env} onChange={() => setEnvironment(env)} />
              {env === "sandbox" ? "Teste" : "Produção (valendo)"}
            </label>
          ))}
        </div>
        {item.fields.map((f) =>
          f.secret ? (
            <Field
              key={f.key}
              label={f.label}
              type="password"
              autoComplete="off"
              value={secrets[f.key] ?? ""}
              placeholder={stored.has(f.key) ? "•••••••• (guardado; deixe em branco para manter)" : ""}
              onChange={(e) => setSecrets({ ...secrets, [f.key]: e.target.value })}
              error={fields[`secrets.${f.key}`]}
              hint={f.help ?? undefined}
            />
          ) : (
            <Field
              key={f.key}
              label={f.label}
              value={config[f.key] ?? ""}
              onChange={(e) => setConfig({ ...config, [f.key]: e.target.value })}
              error={fields[`config.${f.key}`]}
              hint={f.help ?? undefined}
            />
          ),
        )}
        <label className="check">
          <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
          Ativada
        </label>
        <p className="muted small">
          Os dados de acesso ficam guardados com criptografia neste computador e nunca aparecem no histórico.
        </p>
        <div className="row row--end">
          {saved && (
            <button type="button" className="btn btn--ghost" disabled={busy} onClick={() => void remove()}>
              Remover
            </button>
          )}
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {busy ? "Salvando…" : "Salvar e testar"}
          </button>
        </div>
      </form>
    </Dialog>
  );
}
