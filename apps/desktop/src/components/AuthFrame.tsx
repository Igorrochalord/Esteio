import { useEffect, useState, type ReactNode } from "react";

import { authApi } from "../services/endpoints";
import { applyBranding } from "../services/theme";
import type { Branding } from "../services/types";
import { AuthShowcase } from "./AuthShowcase";
import { BrandMark } from "./BrandMark";

/**
 * Enquadramento das telas de entrada: painel com a marca da empresa (cores e logo
 * já aplicados antes do login) e, ao lado, o formulário.
 */
export function AuthFrame({ children }: { children: ReactNode }) {
  const [brand, setBrand] = useState<Branding | null>(null);

  useEffect(() => {
    authApi
      .loginBranding()
      .then((b) => {
        setBrand(b);
        applyBranding(b);
      })
      .catch(() => undefined);
  }, []);

  const name = brand?.name ?? "Núcleo ERP";
  return (
    <div className="auth-split">
      <aside className="auth-brand">
        <div className="auth-brand-top">
          <BrandMark name={name} logoUrl={brand?.logo_url ?? null} size={64} />
          <p className="auth-brand-name">{name}</p>
        </div>
        <p className="auth-brand-tagline">Gestão do dia a dia da sua empresa, em um só lugar.</p>
        <AuthShowcase />
        <p className="auth-brand-foot">Os dados ficam guardados neste computador.</p>
      </aside>
      <main className="auth-main">
        <div className="auth-card">{children}</div>
      </main>
    </div>
  );
}
