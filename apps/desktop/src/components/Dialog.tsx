import { useEffect, useId, useRef, type ReactNode } from "react";

interface Props {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  /** largo: para tabelas e formulários grandes */
  wide?: boolean;
}

/** <dialog> nativo: foco preso, Esc fecha, acessível sem biblioteca. */
export function Dialog({ open, title, onClose, children, wide }: Props) {
  const ref = useRef<HTMLDialogElement>(null);
  const titleId = useId();

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) {
      el.showModal();
      // o navegador foca o primeiro botão (o ✕); para formulários, o certo é o primeiro campo
      requestAnimationFrame(() =>
        el.querySelector<HTMLElement>(".dialog-body input:not([type=hidden]), .dialog-body select, .dialog-body textarea")?.focus(),
      );
    }
    if (!open && el.open) el.close();
  }, [open]);

  return (
    <dialog ref={ref} className={wide ? "dialog dialog--wide" : "dialog"} onClose={onClose} aria-labelledby={titleId}>
      <header className="dialog-header">
        <h2 id={titleId}>{title}</h2>
        <button type="button" className="btn btn--ghost" onClick={onClose} aria-label="Fechar">
          ✕
        </button>
      </header>
      <div className="dialog-body">{children}</div>
    </dialog>
  );
}
