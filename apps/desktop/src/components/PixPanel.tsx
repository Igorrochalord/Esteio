import { CheckCircle2, Copy, QrCode, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { pixApi } from "../services/endpoints";
import { dateTime, money } from "../services/format";
import { PIX_STATUS_LABEL } from "../services/labels";
import type { PixCharge, PixProviderOption } from "../services/types";
import { Notice } from "./Notice";

/** Intervalo do "já pagou?" enquanto a tela está aberta. */
const POLL_MS = 4000;
const ACTIVE = new Set(["gerando", "aguardando"]);

function remaining(expiresAt: string | null, now: number): string | null {
  if (!expiresAt) return null;
  const secs = Math.max(0, Math.floor((new Date(expiresAt).getTime() - now) / 1000));
  const m = Math.floor(secs / 60);
  return `${m}:${String(secs % 60).padStart(2, "0")}`;
}

/**
 * Cobrança PIX de uma venda: QR Code, copia-e-cola e acompanhamento do pagamento.
 * Enquanto estiver aberta, confere o pagamento a cada poucos segundos; fechada, a fila
 * continua conferindo em segundo plano e dá baixa sozinha.
 */
export function PixPanel({ saleId, onPaid }: { saleId: string; onPaid?: (c: PixCharge) => void }) {
  const { can } = useAuth();
  const canWrite = can("sales", "write");
  const [charge, setCharge] = useState<PixCharge | null | undefined>(undefined);
  const [providers, setProviders] = useState<PixProviderOption[]>([]);
  const [provider, setProvider] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const paidNotified = useRef(false);

  useEffect(() => {
    let alive = true;
    pixApi
      .forSale(saleId)
      .then((c) => alive && setCharge(c))
      .catch((e) => alive && setError(e instanceof ApiError ? e.message : "Não foi possível carregar o PIX."));
    pixApi
      .providers()
      .then((p) => {
        if (!alive) return;
        setProviders(p);
        setProvider((cur) => cur || p[0]?.id || "");
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [saleId]);

  // confere o pagamento enquanto a cobrança está aberta
  const active = !!charge && ACTIVE.has(charge.status);
  const chargeId = charge?.id;
  useEffect(() => {
    if (!active || !chargeId) return;
    let alive = true;
    const tick = async () => {
      try {
        const c = await pixApi.check(chargeId);
        if (alive) setCharge(c);
      } catch (e) {
        if (alive) setError(e instanceof ApiError ? e.message : "Não foi possível conferir o pagamento.");
      }
    };
    void tick();
    const t = window.setInterval(tick, POLL_MS);
    const clock = window.setInterval(() => setNow(Date.now()), 1000);
    return () => {
      alive = false;
      window.clearInterval(t);
      window.clearInterval(clock);
    };
  }, [active, chargeId]);

  useEffect(() => {
    if (charge?.status === "pago" && !paidNotified.current) {
      paidNotified.current = true;
      onPaid?.(charge);
    }
  }, [charge, onPaid]);

  async function copy() {
    if (!charge?.copy_paste) return;
    try {
      await navigator.clipboard.writeText(charge.copy_paste);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2500);
    } catch {
      setError("Não foi possível copiar. Selecione o código e copie com Ctrl+C.");
    }
  }

  async function run(action: () => Promise<PixCharge>) {
    setBusy(true);
    setError(null);
    try {
      setCharge(await action());
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "Não foi possível concluir.");
    } finally {
      setBusy(false);
    }
  }

  if (charge === undefined && !error) return <p className="muted">Carregando PIX…</p>;

  const left = charge ? remaining(charge.expires_at, now) : null;
  const canRetry = canWrite && (!charge || ["expirado", "erro", "cancelado"].includes(charge.status));

  return (
    <section className="pix-panel" aria-live="polite">
      {error && <Notice tone="error">{error}</Notice>}
      {charge && (
        <>
          <header className="pix-head">
            <div>
              <strong className="pix-amount">{money(charge.amount)}</strong>
              <span className="muted">
                Venda nº {charge.sale_number} · {charge.provider_name}
              </span>
            </div>
            <span className={`tag pix-status pix-status--${charge.status}`}>{PIX_STATUS_LABEL[charge.status]}</span>
          </header>
          {charge.sandbox && (
            <p className="pix-sandbox">Ambiente de teste: este PIX não movimenta dinheiro de verdade.</p>
          )}

          {charge.status === "gerando" && (
            <div className="pix-wait">
              <RefreshCw className="spin" aria-hidden />
              <p>Gerando o código PIX… Sem internet, ele sai assim que a conexão voltar.</p>
            </div>
          )}

          {charge.status === "aguardando" && charge.qr_svg && (
            <div className="pix-body">
              <img
                className="pix-qr"
                alt="QR Code PIX para o cliente escanear"
                src={`data:image/svg+xml;utf8,${encodeURIComponent(charge.qr_svg)}`}
              />
              <div className="stack">
                <p>Mostre o QR Code ao cliente ou envie o código copia-e-cola.</p>
                <textarea className="pix-code" readOnly rows={4} value={charge.copy_paste ?? ""} aria-label="PIX copia-e-cola"
                  onFocus={(e) => e.currentTarget.select()} />
                <div className="row">
                  <button type="button" className="btn btn--primary" onClick={copy}>
                    <Copy aria-hidden /> {copied ? "Copiado!" : "Copiar código"}
                  </button>
                  {left && <span className="muted">Vale por {left}</span>}
                </div>
                <p className="muted pix-hint">
                  <RefreshCw className="spin" aria-hidden /> Aguardando o pagamento. Pode fechar esta janela: a baixa
                  acontece sozinha quando o PIX cair.
                </p>
              </div>
            </div>
          )}

          {charge.status === "pago" && (
            <div className="pix-paid">
              <CheckCircle2 aria-hidden />
              <div>
                <strong>Pagamento recebido</strong>
                <p className="muted">
                  {dateTime(charge.paid_at)} · baixa feita no financeiro.
                </p>
              </div>
            </div>
          )}

          {charge.status === "expirado" && (
            <p>O prazo deste PIX acabou sem pagamento. Gere um novo ou receba de outro jeito no Financeiro.</p>
          )}
          {charge.status === "cancelado" && <p>Cobrança PIX cancelada.</p>}
          {charge.error && <Notice tone="error">{charge.error}</Notice>}

          {canWrite && ACTIVE.has(charge.status) && (
            <div className="row row--end">
              <button type="button" className="btn btn--ghost" disabled={busy} onClick={() => run(() => pixApi.cancel(charge.id))}>
                Cancelar cobrança
              </button>
            </div>
          )}
        </>
      )}

      {canRetry && charge !== undefined && (
        providers.length ? (
          <div className="row">
            {providers.length > 1 && (
              <select aria-label="Serviço de PIX" value={provider} onChange={(e) => setProvider(e.target.value)}>
                {providers.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}{p.sandbox ? " (teste)" : ""}</option>
                ))}
              </select>
            )}
            <button type="button" className="btn btn--primary" disabled={busy || !provider}
              onClick={() => run(() => pixApi.create(saleId, provider))}>
              <QrCode aria-hidden /> {charge ? "Gerar novo PIX" : "Gerar PIX"}
            </button>
          </div>
        ) : (
          !charge && <p className="muted">Nenhum serviço de PIX ativado. O dono ou administrador ativa em Integrações.</p>
        )
      )}
    </section>
  );
}
