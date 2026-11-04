import {
  ArrowLeft,
  ArrowRight,
  BarChart3,
  ClipboardList,
  Boxes,
  Building2,
  DatabaseBackup,
  HelpCircle,
  House,
  Keyboard,
  Landmark,
  Plug,
  ShoppingCart,
  Sparkles,
  UserRound,
  Users,
  type LucideIcon,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import type { Role } from "../services/types";

/**
 * Tutorial em tela cheia, com passos por papel. Cada passo tem uma "cena" animada
 * (ícone flutuando + etiquetas entrando) e o texto. Teclado: ← → Enter, Esc fecha.
 */

interface Step {
  icon: LucideIcon;
  title: string;
  text: string;
  /** etiquetas da cena: atalhos, palavras-chave */
  chips?: string[];
  /** dica destacada */
  tip?: string;
}

function stepsFor(role: Role, firstName: string, company: string): Step[] {
  const welcome: Step = {
    icon: Sparkles,
    title: `Bem-vindo, ${firstName}!`,
    text:
      role === "owner"
        ? `Este é o sistema da ${company}. Como proprietário, você vê e configura tudo. Em 1 minuto mostramos o essencial.`
        : role === "admin"
          ? `Este é o sistema da ${company}. Como administrador, você cuida das vendas, do catálogo, das pessoas e da empresa.`
          : `Este é o sistema da ${company}. Vamos mostrar como registrar vendas e atender clientes.`,
    chips: ["Rápido", "Sem internet", "Seus dados aqui"],
  };
  const done: Step = {
    icon: HelpCircle,
    title: "Pronto para começar",
    text: "Você pode rever este tutorial quando quiser em “Como usar”, no menu à esquerda.",
    chips: ["Como usar"],
  };
  const shortcuts: Step = {
    icon: Keyboard,
    title: "Atalhos que economizam tempo",
    text: "Nas tabelas: clique numa célula e digite para editar, Enter grava e desce, Esc cancela. Na venda: F9 finaliza. Ctrl+C copia uma célula.",
    chips: ["Enter", "Esc", "F9", "Ctrl+C", "↑ ↓ ← →"],
  };
  const sale: Step = {
    icon: ShoppingCart,
    title: "Registrar uma venda",
    text: "Em Vendas → Nova venda, busque o produto pelo nome ou código (o leitor de código de barras funciona direto), ajuste a quantidade, escolha o pagamento e finalize.",
    chips: ["Nova venda", "Código de barras", "Pix", "Fiado"],
    tip: "Fiado exige um cliente cadastrado.",
  };
  const serviceOrders: Step = {
    icon: ClipboardList,
    title: "Ordens de serviço",
    text:
      role === "user"
        ? "Em Ordens de serviço abra a OS do cliente, registre o problema, adicione mão de obra e peças e arraste o cartão no quadro conforme o trabalho anda. Concluída, é só faturar."
        : "Abra a OS, defina técnico e previsão, lance mão de obra e peças e acompanhe tudo no quadro (aberta → em andamento → aguardando → concluída). Faturar gera a venda: peças saem do estoque e o valor cai no financeiro.",
    chips: ["Quadro", "Mão de obra + peças", "Faturar", "Imprimir OS"],
    tip: "OS com previsão vencida ficam marcadas em vermelho.",
  };
  const customers: Step = {
    icon: UserRound,
    title: "Clientes",
    text: "Cadastre nome, CPF/CNPJ, telefone, cidade e UF. Com a cidade preenchida, o cliente aparece no mapa de vendas. Dá para cadastrar na hora da venda pelo botão +.",
    chips: ["CPF/CNPJ", "Cidade e UF", "+ na venda"],
  };

  if (role === "user") {
    return [
      welcome,
      { icon: House, title: "Seu início", text: "Aqui ficam os atalhos e quanto você vendeu hoje.", chips: ["Suas vendas hoje"] },
      sale,
      serviceOrders,
      customers,
      {
        icon: Boxes,
        title: "Consultar produtos",
        text: "Em Produtos você vê preços e estoque. Produtos com ⚠ estão abaixo do mínimo: avise o responsável.",
        chips: ["Preço", "Estoque", "⚠ mínimo"],
      },
      shortcuts,
      done,
    ];
  }

  return [
    welcome,
    {
      icon: House,
      title: "Início: o painel da empresa",
      text: "Vendido hoje contra ontem, o mês contra o mês passado, os últimos 30 dias, os mais vendidos e alertas como estoque baixo e pedidos de acesso.",
      chips: ["Hoje × ontem", "Mês × mês", "Alertas"],
    },
    sale,
    {
      icon: Boxes,
      title: "Produtos como planilha",
      text: "Edite preço, custo e mínimo direto na tabela. O estoque muda por movimentação (entrada, saída, ajuste por contagem). Importe sua planilha de produtos em Importar planilha.",
      chips: ["Editar na célula", "Importar Excel", "Movimentar estoque"],
      tip: "Enter numa célula de estoque abre o histórico do produto.",
    },
    customers,
    serviceOrders,
    {
      icon: Landmark,
      title: "Financeiro sem retrabalho",
      text: "Vendas à vista entram sozinhas no caixa e o fiado vira conta a receber. Lance contas a pagar (com parcelas ou repetição mensal), dê baixa, importe o extrato do banco (OFX) para conciliar e acompanhe o fluxo de caixa e a DRE.",
      chips: ["A pagar", "A receber", "Extrato OFX", "DRE"],
      tip: "Contas vencidas aparecem em Atenção, no Início.",
    },
    {
      icon: BarChart3,
      title: "Relatórios e comparação",
      text: "Vendas por dia, semana ou mês, formas de pagamento, curva ABC, estoque, mapa por estado e desempenho de cada vendedor. Compare com o período anterior e exporte para Excel ou PDF.",
      chips: ["Curva ABC", "Mapa por UF", "Excel", "PDF"],
    },
    {
      icon: Users,
      title: "Pessoas e permissões",
      text: "Adicione quem usa o sistema e escolha o papel. Pedidos de cadastro feitos na tela de entrada aparecem aqui para aprovar. Esqueceu a senha? Redefina por aqui.",
      chips: ["Aprovar pedidos", "Papéis", "Redefinir senha"],
    },
    {
      icon: Building2,
      title: "Sua empresa, do seu jeito",
      text: "Em Empresa troque logo e cores: o sistema inteiro assume a sua marca. No ícone ao lado do seu nome, escolha tema claro, escuro ou automático.",
      chips: ["Logo", "Cores", "Tema escuro"],
    },
    {
      icon: Plug,
      title: "Integrações on-line",
      text: "Em Integrações você conecta o sistema a serviços on-line, como PIX, nota fiscal e lojas virtuais. Sem internet nada para: o que precisa ser enviado espera na fila e segue sozinho quando a conexão voltar.",
      chips: ["On-line", "Fila automática", "Dados de acesso protegidos"],
      tip: "O aviso no rodapé do menu mostra se há algo esperando para ser enviado.",
    },
    {
      icon: DatabaseBackup,
      title: "Seus dados estão protegidos",
      text:
        role === "owner"
          ? "O sistema faz uma cópia de segurança por dia. Guarde também uma cópia fora do computador em Cópias de segurança; só você pode restaurar."
          : "O sistema faz uma cópia de segurança por dia. Em Cópias de segurança você pode salvar uma cópia num pen drive ou na nuvem.",
      chips: ["Cópia diária", "Salvar em…", role === "owner" ? "Restaurar" : "Pen drive"],
    },
    {
      icon: Sparkles,
      title: "Planos",
      text: "Hoje está tudo liberado. Na página Planos você vê o que cada plano vai oferecer quando a cobrança começar.",
      chips: ["Tudo liberado", "Gratuito", "Starter", "Platinum"],
    },
    shortcuts,
    done,
  ];
}

export function Tour({
  open,
  role,
  firstName,
  company,
  onClose,
}: {
  open: boolean;
  role: Role;
  firstName: string;
  company: string;
  onClose: () => void;
}) {
  const steps = stepsFor(role, firstName, company);
  const [i, setI] = useState(0);
  const [dir, setDir] = useState<"next" | "prev">("next");
  const ref = useRef<HTMLDialogElement>(null);
  const nextRef = useRef<HTMLButtonElement>(null);
  const last = i === steps.length - 1;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) {
      setI(0);
      el.showModal();
      // o diálogo foca o primeiro botão ("Pular"); Enter deve avançar
      requestAnimationFrame(() => nextRef.current?.focus());
    }
    if (!open && el.open) el.close();
  }, [open]);

  const go = useCallback(
    (to: number) => {
      if (to < 0 || to >= steps.length) return;
      setDir(to > i ? "next" : "prev");
      setI(to);
    },
    [i, steps.length],
  );

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowRight" || e.key === "Enter") {
      e.preventDefault();
      if (last) onClose();
      else go(i + 1);
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      go(i - 1);
    }
  };

  const step = steps[i];
  return (
    <dialog ref={ref} className="tour" onClose={onClose} onKeyDown={onKey} aria-label="Como usar o sistema">
      <div className="tour-progress" aria-hidden>
        <span style={{ width: `${((i + 1) / steps.length) * 100}%` }} />
      </div>
      <div className="tour-body">
        <div key={`scene-${i}`} className={`tour-scene tour-${dir}`} aria-hidden>
          <span className="tour-blob" />
          <span className="tour-icon">
            {last ? (
              <svg viewBox="0 0 52 52" width="64" height="64" className="tour-check">
                <circle cx="26" cy="26" r="24" />
                <path d="M15 27 l7 7 l15 -16" />
              </svg>
            ) : (
              <step.icon size={56} strokeWidth={1.6} />
            )}
          </span>
          <span className="tour-chips">
            {step.chips?.map((c, k) => (
              <span key={c} className="tour-chip" style={{ animationDelay: `${180 + k * 90}ms` }}>
                {c}
              </span>
            ))}
          </span>
        </div>
        <div key={`text-${i}`} className={`tour-text tour-${dir}`}>
          <span className="tour-count">
            {i + 1} de {steps.length}
          </span>
          <h2>{step.title}</h2>
          <p>{step.text}</p>
          {step.tip && <p className="tour-tip">💡 {step.tip}</p>}
        </div>
      </div>
      <footer className="tour-footer">
        <button type="button" className="btn btn--ghost" onClick={onClose}>
          {last ? "Fechar" : "Pular tutorial"}
        </button>
        <div className="tour-dots" role="tablist" aria-label="Passos">
          {steps.map((s, k) => (
            <button
              key={k}
              type="button"
              role="tab"
              aria-selected={k === i}
              aria-label={`Passo ${k + 1}: ${s.title}`}
              className={k === i ? "is-on" : undefined}
              onClick={() => go(k)}
            />
          ))}
        </div>
        <div className="row">
          {i > 0 && (
            <button type="button" className="btn btn--ghost" onClick={() => go(i - 1)} aria-label="Voltar">
              <ArrowLeft size={16} aria-hidden />
            </button>
          )}
          <button ref={nextRef} type="button" className="btn btn--primary" onClick={() => (last ? onClose() : go(i + 1))}>
            {last ? "Começar a usar" : "Próximo"} {!last && <ArrowRight size={16} aria-hidden />}
          </button>
        </div>
      </footer>
    </dialog>
  );
}
