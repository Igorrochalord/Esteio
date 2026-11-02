import { useEffect, useRef, type ReactNode } from "react";

interface Props {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
}

/** <dialog> nativo: foco preso, Esc fecha, acessível sem biblioteca. */
export function Dialog({ open, title, onClose, children }: Props) {
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    if (!open && el.open) el.close();
  }, [open]);

  return (
    <dialog ref={ref} className="dialog" onClose={onClose} aria-labelledby="dialog-title">
      <header className="dialog-header">
        <h2 id="dialog-title">{title}</h2>
        <button type="button" className="btn btn--ghost" onClick={onClose} aria-label="Fechar">
          ✕
        </button>
      </header>
      {children}
    </dialog>
  );
}
