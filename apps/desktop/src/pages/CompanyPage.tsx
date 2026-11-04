import { useEffect, useState, type FormEvent } from "react";

import { BrandMark } from "../components/BrandMark";
import { ColorField } from "../components/ColorField";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { companyApi } from "../services/endpoints";
import { formatCnpj } from "../services/labels";
import { isHex } from "../services/theme";

const message = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);

export function CompanyPage() {
  const { me, can, reloadMe } = useAuth();
  const canManage = can("settings", "manage");
  const { data: company, error: loadError, setData } = useAsync(companyApi.get);
  const [info, setInfo] = useState({ name: "", cnpj: "" });
  const [colors, setColors] = useState({ primary: "", secondary: "" });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (company) setInfo({ name: company.name, cnpj: formatCnpj(company.cnpj).replace("—", "") });
  }, [company]);
  useEffect(() => {
    if (me) setColors({ primary: me.company.primary_color, secondary: me.company.secondary_color });
  }, [me]);

  async function run(action: () => Promise<unknown>, done: string, fallback: string) {
    setBusy(true);
    setError(null);
    setFields({});
    setNotice(null);
    try {
      await action();
      await reloadMe();
      setNotice(done);
    } catch (e) {
      setError(message(e, fallback));
      if (e instanceof ApiError) setFields(e.fields);
    } finally {
      setBusy(false);
    }
  }

  const saveInfo = (e: FormEvent) => {
    e.preventDefault();
    void run(
      async () => setData(await companyApi.update({ name: info.name, cnpj: info.cnpj.trim() || null })),
      "Dados da empresa salvos.",
      "Não foi possível salvar os dados.",
    );
  };

  const saveColors = (e: FormEvent) => {
    e.preventDefault();
    if (!isHex(colors.primary) || !isHex(colors.secondary)) return;
    void run(
      () => companyApi.updateBranding({ primary_color: colors.primary, secondary_color: colors.secondary }),
      "Cores salvas.",
      "Não foi possível salvar as cores.",
    );
  };

  if (!me) return null;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>Empresa</h1>
          <p className="muted">Dados cadastrais e a identidade visual que sua equipe vê.</p>
        </div>
      </header>

      {(loadError || error) && <Notice tone="error">{loadError ?? error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      <section className="panel">
        <h2>Dados cadastrais</h2>
        <form className="stack form-narrow" onSubmit={saveInfo}>
          <fieldset disabled={!canManage || busy} className="stack">
            <Field label="Nome da empresa" required minLength={2} value={info.name} error={fields.name}
              onChange={(e) => setInfo({ ...info, name: e.target.value })} />
            <Field label="CNPJ" value={info.cnpj} error={fields.cnpj} inputMode="numeric"
              placeholder="00.000.000/0000-00"
              onChange={(e) => setInfo({ ...info, cnpj: e.target.value })} />
          </fieldset>
          {canManage && (
            <div className="row">
              <button className="btn btn--primary" disabled={busy}>Salvar dados</button>
            </div>
          )}
        </form>
      </section>

      <section className="panel">
        <h2>Identidade visual</h2>
        <div className="logo-row">
          <BrandMark name={me.company.name} logoUrl={me.company.logo_url} size={72} />
          {canManage && (
            <div className="stack stack--tight">
              <label className="btn btn--ghost file-btn">
                {me.company.logo_url ? "Trocar logo" : "Enviar logo"}
                <input type="file" accept="image/png,image/jpeg" disabled={busy}
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    e.target.value = "";
                    if (file) void run(() => companyApi.uploadLogo(file), "Logo atualizado.", "Não foi possível enviar o logo.");
                  }} />
              </label>
              {me.company.logo_url && (
                <button type="button" className="link-btn" disabled={busy}
                  onClick={() => void run(companyApi.removeLogo, "Logo removido.", "Não foi possível remover o logo.")}>
                  Remover logo
                </button>
              )}
              <p className="field-hint">PNG ou JPEG até 2 MB, de preferência quadrado.</p>
            </div>
          )}
        </div>

        <form className="stack form-narrow" onSubmit={saveColors}>
          <fieldset disabled={!canManage || busy} className="stack">
            <ColorField label="Cor principal" hint="Botões e destaques." value={colors.primary}
              onChange={(v) => setColors({ ...colors, primary: v })} />
            <ColorField label="Cor do menu" hint="Fundo da barra lateral." value={colors.secondary}
              onChange={(v) => setColors({ ...colors, secondary: v })} />
          </fieldset>
          {canManage && (
            <div className="row">
              <button className="btn btn--primary" disabled={busy}>Salvar cores</button>
            </div>
          )}
        </form>
      </section>
    </div>
  );
}
