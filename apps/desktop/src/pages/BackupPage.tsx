import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";

import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { backupApi } from "../services/endpoints";
import { formatDateTime } from "../services/labels";
import type { BackupInfo } from "../services/types";

const KIND_LABEL: Record<BackupInfo["kind"], string> = {
  auto: "Automática",
  manual: "Manual",
  "pre-restore": "Antes de restaurar",
};

const FILTERS = [{ name: "Cópia do Núcleo ERP", extensions: ["db"] }];

function formatSize(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1).replace(".", ",")} MB`;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

export function BackupPage() {
  const { can, endSession } = useAuth();
  const canRestore = can("backup", "manage");
  const { data, error: loadError, loading, reload } = useAsync(backupApi.list);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function run(action: () => Promise<string | null>) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const done = await action();
      if (done) setNotice(done);
      await reload();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Não foi possível concluir a operação.");
    } finally {
      setBusy(false);
    }
  }

  const backupNow = () =>
    run(async () => {
      await backupApi.create();
      return "Cópia de segurança criada.";
    });

  const backupTo = () =>
    run(async () => {
      const dest = await save({ defaultPath: `nucleo-erp-${today()}.db`, filters: FILTERS });
      if (!dest) return null;
      await backupApi.create(dest);
      return `Cópia salva em ${dest}. Guarde-a num pen drive ou na nuvem.`;
    });

  async function restore(path: string, label: string) {
    const confirmed = await ask(
      `Os dados atuais serão substituídos pela cópia "${label}". ` +
        "Antes disso o sistema guarda uma cópia do estado atual. Você precisará entrar de novo. Continuar?",
      { title: "Restaurar cópia de segurança", kind: "warning", okLabel: "Restaurar", cancelLabel: "Cancelar" },
    );
    if (!confirmed) return;
    setBusy(true);
    setError(null);
    try {
      await backupApi.restore(path);
      endSession();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Não foi possível restaurar a cópia.");
      setBusy(false);
    }
  }

  const restoreFromFile = async () => {
    const path = await open({ multiple: false, directory: false, filters: FILTERS });
    if (typeof path === "string") await restore(path, path.split(/[\\/]/).pop() ?? path);
  };

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Cópias de segurança</h1>
          <p className="muted">
            Os dados ficam só neste computador. O sistema faz uma cópia automática por dia e guarda as 7
            mais recentes; guarde também uma cópia fora daqui (pen drive ou nuvem).
          </p>
        </div>
      </header>

      {error && <Notice tone="error">{error}</Notice>}
      {loadError && <Notice tone="error">{loadError}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      <section className="panel">
        <div className="row">
          <button className="btn btn--primary" disabled={busy} onClick={() => void backupNow()}>
            Fazer cópia agora
          </button>
          <button className="btn btn--ghost" disabled={busy} onClick={() => void backupTo()}>
            Salvar cópia em…
          </button>
          {canRestore && (
            <button className="btn btn--ghost" disabled={busy} onClick={() => void restoreFromFile()}>
              Restaurar de um arquivo…
            </button>
          )}
        </div>
      </section>

      {loading && !data ? (
        <p className="muted">Carregando…</p>
      ) : data && data.length === 0 ? (
        <p className="muted">Nenhuma cópia ainda.</p>
      ) : (
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>Quando</th>
                <th>Tipo</th>
                <th>Tamanho</th>
                {canRestore && <th className="actions" aria-label="Ações" />}
              </tr>
            </thead>
            <tbody>
              {data?.map((b) => (
                <tr key={b.path}>
                  <td className="num nowrap">{formatDateTime(b.created_at)}</td>
                  <td>{KIND_LABEL[b.kind]}</td>
                  <td className="num">{formatSize(b.size_bytes)}</td>
                  {canRestore && (
                    <td className="actions">
                      <button className="btn btn--ghost" disabled={busy}
                        onClick={() => void restore(b.path, formatDateTime(b.created_at))}>
                        Restaurar
                      </button>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
