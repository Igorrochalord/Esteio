import type { PlanCode } from "./types";

/** Textos dos planos (preços e limites vêm do núcleo: plans.rs). */
export const PLAN_COPY: Record<PlanCode, { name: string; tagline: string; audience: string; features: string[]; missing?: string[] }> = {
  gratuito: {
    name: "Gratuito",
    tagline: "Para organizar a gestão",
    audience: "Pessoa física ou jurídica",
    features: ["Gestão financeira", "Cadastros, vendas e estoque", "Até 3 contas bancárias", "Usuários ilimitados", "Suporte por e-mail"],
    missing: ["Emissão de NFS-e", "Ordens de serviço"],
  },
  starter: {
    name: "Starter",
    tagline: "Para quem está começando",
    audience: "Autônomos e pequenas empresas",
    features: [
      "Tudo do Gratuito",
      "Emissão de NFS-e (25 por mês)",
      "Ordens de serviço ilimitadas",
      "Contas bancárias ilimitadas",
      "Conciliação bancária",
      "Até 5 CNPJs adicionais",
      "Suporte por e-mail, WhatsApp e telefone",
    ],
  },
  platinum: {
    name: "Platinum",
    tagline: "Para crescer com volume",
    audience: "Empresas em expansão",
    features: ["Tudo do Starter", "100 NFS-e por mês", "Relatórios avançados com DRE", "Até 10 CNPJs adicionais", "Suporte prioritário"],
  },
  infinite: {
    name: "Infinite",
    tagline: "Para quem não pode parar",
    audience: "Alto volume e atendimento exclusivo",
    features: ["Tudo do Platinum", "1.000 NFS-e por mês", "CNPJs adicionais ilimitados", "Suporte prioritário dedicado"],
  },
};
