import { initials } from "../services/theme";

interface Props {
  name: string;
  logoUrl: string | null;
  size?: number;
}

/** Logo da empresa, ou as iniciais sobre a cor primária quando não há logo. */
export function BrandMark({ name, logoUrl, size = 40 }: Props) {
  if (logoUrl) {
    return (
      <img className="brand-mark" src={logoUrl} alt={`Logo de ${name}`} width={size} height={size} />
    );
  }
  return (
    <span className="brand-mark brand-mark--initials" style={{ width: size, height: size }} aria-hidden>
      {initials(name) || "?"}
    </span>
  );
}
