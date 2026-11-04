import { Link } from "react-router";

import { useAuth } from "../hooks/useAuth";

function greeting(): string {
  const h = new Date().getHours();
  return h < 12 ? "Bom dia" : h < 18 ? "Boa tarde" : "Boa noite";
}

export function DashboardPage() {
  const { me, can } = useAuth();
  if (!me) return null;
  const firstName = me.user.name.split(" ")[0];
  const setupPending = !me.company.logo_url;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <h1>
            {greeting()}, {firstName}
          </h1>
          <p className="muted">
            {me.company.name}
          </p>
        </div>
      </header>

      {can("settings", "manage") && (
        <section className="panel">
          <h2>Primeiros passos</h2>
          <ul className="checklist">
            <li className={setupPending ? "" : "is-done"}>
              <span>Envie o logo da empresa para aparecer no menu.</span>
              {setupPending && <Link to="/empresa">Enviar logo</Link>}
            </li>
            {can("backup", "write") && (
              <li>
                <span>O sistema guarda uma cópia de segurança por dia. Guarde também uma fora deste computador.</span>
                <Link to="/backup">Fazer cópia</Link>
              </li>
            )}
            {can("users", "manage") && (
              <li>
                <span>Adicione as pessoas que vão usar o sistema.</span>
                <Link to="/usuarios">Adicionar pessoas</Link>
              </li>
            )}
          </ul>
        </section>
      )}

      <section className="panel panel--quiet">
        <h2>Em breve neste painel</h2>
        <p className="muted">
          Vendas do mês, ticket médio, contas vencidas e produtos abaixo do estoque mínimo aparecem
          aqui assim que os módulos de estoque, vendas e financeiro forem ativados.
        </p>
      </section>
    </div>
  );
}
