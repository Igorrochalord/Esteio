import { HashRouter, Navigate, Route, Routes } from "react-router";

import { AuthProvider } from "./components/AuthProvider";
import { RequireAnonymous, RequireAuth, RequirePermission } from "./components/guards";
import { AppLayout } from "./layouts/AppLayout";
import { AuthLayout } from "./layouts/AuthLayout";
import { AuditPage } from "./pages/AuditPage";
import { BackupPage } from "./pages/BackupPage";
import { CompanyPage } from "./pages/CompanyPage";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { UsersPage } from "./pages/UsersPage";

// HashRouter: o app carrega arquivos locais; não há servidor para fallback de rotas.
export function App() {
  return (
    <AuthProvider>
      <HashRouter>
        <Routes>
          <Route element={<RequireAnonymous><AuthLayout /></RequireAnonymous>}>
            <Route path="/login" element={<LoginPage />} />
            <Route path="/onboarding" element={<OnboardingPage />} />
          </Route>
          <Route element={<RequireAuth><AppLayout /></RequireAuth>}>
            <Route index element={<DashboardPage />} />
            <Route path="/usuarios" element={<RequirePermission module="users"><UsersPage /></RequirePermission>} />
            <Route path="/empresa" element={<RequirePermission module="settings"><CompanyPage /></RequirePermission>} />
            <Route path="/backup" element={<RequirePermission module="backup" level="write"><BackupPage /></RequirePermission>} />
            <Route path="/auditoria" element={<RequirePermission module="audit"><AuditPage /></RequirePermission>} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </HashRouter>
    </AuthProvider>
  );
}
