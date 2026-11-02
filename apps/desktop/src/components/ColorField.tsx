import { useId } from "react";

import { isHex } from "../services/theme";

interface Props {
  label: string;
  value: string;
  onChange: (value: string) => void;
  hint?: string;
}

/** Seletor nativo + campo hex, mantidos em sincronia. */
export function ColorField({ label, value, onChange, hint }: Props) {
  const id = useId();
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      <div className="color-field">
        <input
          type="color"
          aria-label={`${label}: seletor`}
          value={isHex(value) ? value : "#000000"}
          onChange={(e) => onChange(e.target.value)}
        />
        <input
          id={id}
          value={value}
          maxLength={7}
          spellCheck={false}
          aria-invalid={!isHex(value)}
          onChange={(e) => onChange(e.target.value.startsWith("#") ? e.target.value : `#${e.target.value}`)}
        />
      </div>
      {!isHex(value) ? (
        <p className="field-error">Use o formato #RRGGBB.</p>
      ) : hint ? (
        <p className="field-hint">{hint}</p>
      ) : null}
    </div>
  );
}
