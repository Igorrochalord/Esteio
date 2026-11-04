import { useId, type InputHTMLAttributes, type ReactNode } from "react";

interface FieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  error?: string;
  hint?: ReactNode;
  /** botão dentro do campo, à direita (ex.: mostrar senha) */
  action?: ReactNode;
}

export function Field({ label, error, hint, action, className, ...input }: FieldProps) {
  const id = useId();
  const describedBy = error ? `${id}-err` : hint ? `${id}-hint` : undefined;
  const control = <input id={id} aria-invalid={!!error} aria-describedby={describedBy} {...input} />;
  return (
    <div className={`field ${className ?? ""}`}>
      <label htmlFor={id}>{label}</label>
      {action ? (
        <div className="field-control">
          {control}
          {action}
        </div>
      ) : (
        control
      )}
      {error ? (
        <p className="field-error" id={`${id}-err`}>
          {error}
        </p>
      ) : hint ? (
        <p className="field-hint" id={`${id}-hint`}>
          {hint}
        </p>
      ) : null}
    </div>
  );
}
