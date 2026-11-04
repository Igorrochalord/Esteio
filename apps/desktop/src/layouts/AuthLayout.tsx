import { Outlet } from "react-router";

/** Login, cadastro e onboarding: cada página define o próprio enquadramento. */
export function AuthLayout() {
  return <Outlet />;
}
