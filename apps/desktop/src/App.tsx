import { HashRouter, Navigate, Route, Routes } from "react-router";

import { AuthProvider } from "./components/AuthProvider";
import { RequireAnonymous, RequireAuth, RequirePermission } from "./components/guards";
import { AppLayout } from "./layouts/AppLayout";
import { AuthLayout } from "./layouts/AuthLayout";
import { AuditPage } from "./pages/AuditPage";
import { BackupPage } from "./pages/BackupPage";
import { CompanyPage } from "./pages/CompanyPage";
import { CustomersPage } from "./pages/CustomersPage";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { PlansPage } from "./pages/PlansPage";
import { ProductsPage } from "./pages/ProductsPage";
import { RegisterPage } from "./pages/RegisterPage";
import { ReportsPage } from "./pages/ReportsPage";
import { SalesPage } from "./pages/SalesPage";
import { UsersPage } from "./pages/UsersPage";

// HashRouter: o app carrega arquivos locais; não há servidor para fallback de rotas.
export function App() {
  return (
    <AuthProvider>
      <HashRouter>
        <Routes>
          <Route element={<RequireAnonymous><AuthLayout /></RequireAnonymous>}>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/cadastro" element={<RegisterPage />} />
            <Route path="/onboarding" element={<OnboardingPage />} />
          </Route>
          <Route element={<RequireAuth><AppLayout /></RequireAuth>}>
            <Route index element={<DashboardPage />} />
            <Route path="/vendas" element={<RequirePermission module="sales"><SalesPage /></RequirePermission>} />
            <Route path="/produtos" element={<RequirePermission module="products"><ProductsPage /></RequirePermission>} />
            <Route path="/clientes" element={<RequirePermission module="customers"><CustomersPage /></RequirePermission>} />
            <Route path="/relatorios" element={<RequirePermission module="reports"><ReportsPage /></RequirePermission>} />
            <Route path="/usuarios" element={<RequirePermission module="users"><UsersPage /></RequirePermission>} />
            <Route path="/empresa" element={<RequirePermission module="settings"><CompanyPage /></RequirePermission>} />
            <Route path="/planos" element={<PlansPage />} />
            <Route path="/backup" element={<RequirePermission module="backup" level="write"><BackupPage /></RequirePermission>} />
            <Route path="/auditoria" element={<RequirePermission module="audit"><AuditPage /></RequirePermission>} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </HashRouter>
    </AuthProvider>
  );
}
