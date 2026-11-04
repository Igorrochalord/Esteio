import { useEffect, useMemo, useState, type CSSProperties, type FormEvent } from "react";
import { useNavigate } from "react-router";

import { BrandMark } from "../components/BrandMark";
import { ColorField } from "../components/ColorField";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { companyApi } from "../services/endpoints";
import { DEFAULT_PRIMARY, DEFAULT_SECONDARY, inkFor, isHex } from "../services/theme";

const STEPS = ["Empresa", "Identidade visual", "Seu acesso"] as const;

export function OnboardingPage() {
  const { startSession, reloadMe } = useAuth();
  const navigate = useNavigate();
  const [step, setStep] = useState(0);
  const [company, setCompany] = useState({ name: "", cnpj: "" });
  const [colors, setColors] = useState({ primary: DEFAULT_PRIMARY, secondary: DEFAULT_SECONDARY });
  const [logo, setLogo] = useState<File | null>(null);
  const [owner, setOwner] = useState({ name: "", email: "", password: "", confirm: "" });
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);

  const logoPreview = useMemo(() => (logo ? URL.createObjectURL(logo) : null), [logo]);
  useEffect(() => () => { if (logoPreview) URL.revokeObjectURL(logoPreview); }, [logoPreview]);

  function next(e: FormEvent) {
    e.preventDefault();
    setError(null);
    if (step === 1 && !(isHex(colors.primary) && isHex(colors.secondary))) return;
    if (step < STEPS.length - 1) {
      setStep(step + 1);
      return;
    }
    void submit();
  }

  async function submit() {
    if (owner.password !== owner.confirm) {
      setFieldErrors({ confirm: "As senhas não conferem." });
      return;
    }
    setBusy(true);
    setFieldErrors({});
    try {
      const me = await companyApi.onboard({
        company: {
          name: company.name,
          cnpj: company.cnpj.trim() || null,
          primary_color: colors.primary,
          secondary_color: colors.secondary,
        },
        owner: { name: owner.name, email: owner.email, password: owner.password },
      });
      startSession(me);
      if (logo) {
        // a empresa já existe; se o logo falhar, dá para reenviar em Empresa
        try {
          await companyApi.uploadLogo(logo);
          await reloadMe();
        } catch {
          /* segue sem logo */
        }
      }
      navigate("/", { replace: true });
    } catch (err) {
      if (err instanceof ApiError) {
        setError(err.message);
        setFieldErrors(err.fields);
        if (Object.keys(err.fields).some((k) => k.startsWith("company."))) setStep(0);
      } else {
        setError("Não foi possível concluir o cadastro.");
      }
    } finally {
      setBusy(false);
    }
  }

  const fe = (k: string) => fieldErrors[k];

  return (
    <div className="auth-shell">
      <div className="onboarding">
        <section className="onboarding-form">
          <ol className="steps" aria-label="Etapas do cadastro">
            {STEPS.map((label, i) => (
              <li key={label} className={i === step ? "is-current" : i < step ? "is-done" : ""}
                aria-current={i === step ? "step" : undefined}>
                <span className="step-index">{i + 1}</span>
                {label}
              </li>
            ))}
          </ol>

          <form onSubmit={next} className="stack" noValidate={false}>
            {error && <Notice tone="error">{error}</Notice>}

            {step === 0 && (
              <>
                <h1 className="auth-title">Cadastre sua empresa</h1>
                <Field label="Nome da empresa" required minLength={2} autoFocus
                  value={company.name} error={fe("company.name")}
                  onChange={(e) => setCompany({ ...company, name: e.target.value })} />
                <Field label="CNPJ" hint="Opcional. Pode ser informado depois." inputMode="numeric"
                  value={company.cnpj} error={fe("company.cnpj")} placeholder="00.000.000/0000-00"
                  onChange={(e) => setCompany({ ...company, cnpj: e.target.value })} />
              </>
            )}

            {step === 1 && (
              <>
                <h1 className="auth-title">Deixe o sistema com a cara da empresa</h1>
                <ColorField label="Cor principal" hint="Botões e destaques."
                  value={colors.primary} onChange={(v) => setColors({ ...colors, primary: v })} />
                <ColorField label="Cor do menu" hint="Fundo da barra lateral."
                  value={colors.secondary} onChange={(v) => setColors({ ...colors, secondary: v })} />
                <div className="field">
                  <label htmlFor="logo">Logo</label>
                  <input id="logo" type="file" accept="image/png,image/jpeg"
                    onChange={(e) => setLogo(e.target.files?.[0] ?? null)} />
                  <p className="field-hint">PNG ou JPEG até 2 MB. Opcional.</p>
                </div>
              </>
            )}

            {step === 2 && (
              <>
                <h1 className="auth-title">Crie seu acesso</h1>
                <p className="muted">Você será o proprietário da conta e poderá convidar sua equipe.</p>
                <Field label="Seu nome" required minLength={2} autoFocus autoComplete="name"
                  value={owner.name} error={fe("owner.name")}
                  onChange={(e) => setOwner({ ...owner, name: e.target.value })} />
                <Field label="E-mail" type="email" required autoComplete="username"
                  value={owner.email} error={fe("owner.email")}
                  onChange={(e) => setOwner({ ...owner, email: e.target.value })} />
                <Field label="Senha" type="password" required minLength={8} autoComplete="new-password"
                  hint="Mínimo de 8 caracteres." value={owner.password} error={fe("owner.password")}
                  onChange={(e) => setOwner({ ...owner, password: e.target.value })} />
                <Field label="Confirme a senha" type="password" required autoComplete="new-password"
                  value={owner.confirm} error={fe("confirm")}
                  onChange={(e) => setOwner({ ...owner, confirm: e.target.value })} />
              </>
            )}

            <div className="row row--end">
              {step > 0 && (
                <button type="button" className="btn btn--ghost" onClick={() => setStep(step - 1)}>
                  Voltar
                </button>
              )}
              <button className="btn btn--primary" disabled={busy}>
                {step < STEPS.length - 1 ? "Continuar" : busy ? "Criando empresa…" : "Criar empresa"}
              </button>
            </div>
          </form>
        </section>

        <Preview name={company.name || "Sua empresa"} logoUrl={logoPreview}
          primary={isHex(colors.primary) ? colors.primary : DEFAULT_PRIMARY}
          secondary={isHex(colors.secondary) ? colors.secondary : DEFAULT_SECONDARY}
          ownerName={owner.name} />
      </div>
    </div>
  );
}

/** Miniatura do app com as escolhas atuais: o cliente vê o próprio sistema nascendo. */
function Preview(props: { name: string; logoUrl: string | null; primary: string; secondary: string; ownerName: string }) {
  const style = {
    "--brand": props.primary,
    "--brand-ink": inkFor(props.primary),
    "--brand-deep": props.secondary,
    "--brand-deep-ink": inkFor(props.secondary),
  } as CSSProperties;
  return (
    <aside className="preview" style={style} aria-label="Prévia do sistema">
      <div className="preview-window">
        <div className="preview-sidebar">
          <div className="sidebar-brand">
            <BrandMark name={props.name} logoUrl={props.logoUrl} size={36} />
            <span className="sidebar-company">{props.name}</span>
          </div>
          <ul className="preview-nav">
            <li className="is-active">Início</li>
            <li>Estoque</li>
            <li>Vendas</li>
            <li>Financeiro</li>
          </ul>
        </div>
        <div className="preview-main">
          <p className="preview-greeting">Olá{props.ownerName ? `, ${props.ownerName.split(" ")[0]}` : ""}</p>
          <div className="preview-lines" aria-hidden>
            <span /><span /><span />
          </div>
          <span className="btn btn--primary preview-btn">Nova venda</span>
        </div>
      </div>
      <p className="preview-caption">Assim a sua equipe verá o sistema.</p>
    </aside>
  );
}
