import { useState, type ComponentProps, type KeyboardEvent } from "react";

import { Field } from "./Field";

type Props = Omit<ComponentProps<typeof Field>, "type" | "action">;

/** Campo de senha com "mostrar/ocultar" e aviso de Caps Lock. */
export function PasswordField({ hint, onKeyUp, ...props }: Props) {
  const [visible, setVisible] = useState(false);
  const [capsLock, setCapsLock] = useState(false);

  const checkCaps = (e: KeyboardEvent<HTMLInputElement>) => {
    setCapsLock(e.getModifierState("CapsLock"));
    onKeyUp?.(e);
  };

  return (
    <Field
      {...props}
      type={visible ? "text" : "password"}
      onKeyUp={checkCaps}
      hint={capsLock ? <span className="caps-warning">Caps Lock está ativado.</span> : hint}
      action={
        <button
          type="button"
          className="field-action"
          aria-pressed={visible}
          onClick={() => setVisible((v) => !v)}
        >
          {visible ? "Ocultar" : "Mostrar"}
        </button>
      }
    />
  );
}
