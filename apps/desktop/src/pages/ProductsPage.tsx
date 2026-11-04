import { open } from "@tauri-apps/plugin-dialog";
import { useMemo, useState, type FormEvent } from "react";

import { DataGrid, type CellValue, type GridColumn } from "../components/DataGrid";
import { Dialog } from "../components/Dialog";
import { Field } from "../components/Field";
import { Notice } from "../components/Notice";
import { useAsync } from "../hooks/useAsync";
import { useAuth } from "../hooks/useAuth";
import { ApiError } from "../services/api";
import { productsApi, sheetApi, type ProductPatch } from "../services/endpoints";
import { money, parseDecimal, pct, qty, toNum } from "../services/format";
import { MOVEMENT_LABEL, UNIT_OPTIONS } from "../services/labels";
import type { Category, ImportField, ImportPreview, ImportResult, Movement, Product } from "../services/types";

const errorText = (e: unknown, fallback: string) => (e instanceof ApiError ? e.message : fallback);

/** Margem sobre o preço, em %, a partir de textos decimais. */
function marginOf(p: Product): string | null {
  if (p.cost == null || toNum(p.price) === 0) return null;
  return (((toNum(p.price) - toNum(p.cost)) / toNum(p.price)) * 100).toFixed(1);
}

export function ProductsPage() {
  const { can } = useAuth();
  const canEdit = can("products", "write");
  const { data: products, error, loading, setData, reload } = useAsync(productsApi.list);
  const { data: categories, reload: reloadCategories } = useAsync(productsApi.categories);
  const [categoryFilter, setCategoryFilter] = useState("");
  const [onlyLow, setOnlyLow] = useState(false);
  const [showInactive, setShowInactive] = useState(false);
  const [dialog, setDialog] = useState<"new" | "stock" | "categories" | "import" | null>(null);
  const [historyOf, setHistoryOf] = useState<Product | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const rows = useMemo(
    () =>
      (products ?? []).filter(
        (p) =>
          (showInactive || p.active) &&
          (!onlyLow || p.below_min) &&
          (!categoryFilter || (categoryFilter === "none" ? !p.category_id : p.category_id === categoryFilter)),
      ),
    [products, showInactive, onlyLow, categoryFilter],
  );
  const lowCount = (products ?? []).filter((p) => p.below_min).length;
  const categoryOptions = (categories ?? []).map((c) => ({ value: c.id, label: c.name }));
  const showCost = (products ?? []).some((p) => p.cost != null) || canEdit;

  const replace = (p: Product) => setData((list) => (list ?? []).map((x) => (x.id === p.id ? p : x)));

  const columns: GridColumn<Product>[] = [
    { key: "code", label: "Código", get: (p) => p.code, editable: canEdit, width: 90 },
    { key: "name", label: "Nome", get: (p) => p.name, editable: canEdit, width: 240 },
    {
      key: "category_id",
      label: "Categoria",
      kind: "select",
      get: (p) => p.category_id,
      options: categoryOptions,
      editable: canEdit,
      nullable: true,
    },
    { key: "unit", label: "Un.", kind: "select", get: (p) => p.unit, options: UNIT_OPTIONS, editable: canEdit, width: 60 },
    { key: "price", label: "Preço", kind: "money", get: (p) => p.price, editable: canEdit },
    ...(showCost
      ? ([
          { key: "cost", label: "Custo", kind: "money", get: (p) => p.cost, editable: canEdit },
          { key: "margin", label: "Margem", kind: "percent", get: (p) => marginOf(p) },
        ] satisfies GridColumn<Product>[])
      : []),
    {
      key: "stock",
      label: "Estoque",
      kind: "number",
      get: (p) => (p.track_stock ? p.stock : null),
      render: (p) =>
        p.track_stock ? (
          <span className={p.below_min ? "stock-low" : undefined} title={p.below_min ? "Abaixo do mínimo" : undefined}>
            {qty(p.stock)} {p.below_min && "⚠"}
          </span>
        ) : (
          <span className="grid-blank">não controla</span>
        ),
    },
    { key: "min_stock", label: "Mínimo", kind: "number", get: (p) => p.min_stock, editable: canEdit },
    {
      key: "stock_value",
      label: showCost ? "Valor em estoque (custo)" : "Valor em estoque",
      kind: "money",
      get: (p) =>
        p.track_stock ? ((Math.max(0, toNum(p.stock)) * toNum(showCost ? p.cost : p.price) * 100) / 100).toFixed(2) : null,
      total: "sum",
    },
    { key: "track_stock", label: "Controla estoque", kind: "boolean", get: (p) => p.track_stock, editable: canEdit },
    { key: "active", label: "Ativo", kind: "boolean", get: (p) => p.active, editable: canEdit },
  ];

  async function onEdit(p: Product, key: string, value: CellValue) {
    const patch: ProductPatch = {};
    switch (key) {
      case "code":
      case "name":
      case "unit":
        patch[key] = String(value ?? "");
        break;
      case "category_id":
        patch.category_id = (value as string | null) || null;
        break;
      case "price":
      case "cost":
      case "min_stock":
        patch[key] = String(value ?? "0");
        break;
      case "track_stock":
      case "active":
        patch[key] = !!value;
        break;
      default:
        return;
    }
    replace(await productsApi.update(p.id, patch));
  }

  return (
    <div className="page page--wide">
      <header className="page-header">
        <div>
          <h1>Produtos</h1>
          <p className="muted">
            {canEdit
              ? "Edite direto na tabela: clique na célula e digite. O estoque muda por movimentação."
              : "Consulta de preços e estoque."}
          </p>
        </div>
        {canEdit && (
          <div className="row">
            <button className="btn btn--ghost" onClick={() => setDialog("import")}>Importar planilha</button>
            <button className="btn btn--ghost" onClick={() => setDialog("categories")}>Categorias</button>
            <button className="btn btn--ghost" onClick={() => setDialog("stock")}>Movimentar estoque</button>
            <button className="btn btn--primary" onClick={() => setDialog("new")}>Novo produto</button>
          </div>
        )}
      </header>

      {error && <Notice tone="error">{error}</Notice>}
      {notice && <Notice tone="success">{notice}</Notice>}

      {loading && !products ? (
        <p className="muted">Carregando…</p>
      ) : (
        <DataGrid
          rows={rows}
          columns={columns}
          rowKey={(p) => p.id}
          onEdit={canEdit ? onEdit : undefined}
          onRowOpen={(p) => setHistoryOf(p)}
          rowClass={(p) => (!p.active ? "is-muted" : p.below_min ? "is-warning" : undefined)}
          exportName="Produtos"
          emptyText={canEdit ? "Nenhum produto ainda. Cadastre ou importe uma planilha." : "Nenhum produto."}
          initialSort={{ key: "name", dir: "asc" }}
          toolbar={
            <>
              <select aria-label="Filtrar por categoria" value={categoryFilter} onChange={(e) => setCategoryFilter(e.target.value)}>
                <option value="">Todas as categorias</option>
                <option value="none">Sem categoria</option>
                {categoryOptions.map((c) => (
                  <option key={c.value} value={c.value}>{c.label}</option>
                ))}
              </select>
              <label className="check">
                <input type="checkbox" checked={onlyLow} onChange={(e) => setOnlyLow(e.target.checked)} />
                Abaixo do mínimo {lowCount > 0 && <span className="tag tag--warn">{lowCount}</span>}
              </label>
              <label className="check">
                <input type="checkbox" checked={showInactive} onChange={(e) => setShowInactive(e.target.checked)} />
                Mostrar inativos
              </label>
            </>
          }
        />
      )}
      <p className="muted small">Enter numa célula que não é editável (como Estoque) abre o histórico do produto.</p>

      <Dialog wide open={dialog === "new"} title="Novo produto" onClose={() => setDialog(null)}>
        {dialog === "new" && (
          <NewProductForm
            categories={categories ?? []}
            onDone={(p) => {
              setData((list) => [...(list ?? []), p]);
              setNotice(`${p.name} cadastrado com o código ${p.code}.`);
              setDialog(null);
            }}
          />
        )}
      </Dialog>
      <Dialog open={dialog === "stock"} title="Movimentar estoque" onClose={() => setDialog(null)}>
        {dialog === "stock" && (
          <StockForm
            products={(products ?? []).filter((p) => p.active && p.track_stock)}
            onDone={(p, text) => {
              replace(p);
              setNotice(text);
              setDialog(null);
            }}
          />
        )}
      </Dialog>
      <Dialog open={dialog === "categories"} title="Categorias" onClose={() => setDialog(null)}>
        {dialog === "categories" && (
          <CategoriesManager
            categories={categories ?? []}
            onChange={async () => {
              await reloadCategories();
              await reload();
            }}
          />
        )}
      </Dialog>
      <Dialog wide open={dialog === "import"} title="Importar produtos de planilha" onClose={() => setDialog(null)}>
        {dialog === "import" && (
          <ImportWizard
            onDone={async (r) => {
              await reloadCategories();
              await reload();
              setNotice(`Importação concluída: ${r.created} novo(s), ${r.updated} atualizado(s).`);
            }}
          />
        )}
      </Dialog>
      <Dialog wide open={!!historyOf} title={`Histórico de estoque: ${historyOf?.name ?? ""}`} onClose={() => setHistoryOf(null)}>
        {historyOf && <MovementHistory product={historyOf} />}
      </Dialog>
    </div>
  );
}

function NewProductForm({ categories, onDone }: { categories: Category[]; onDone: (p: Product) => void }) {
  const [form, setForm] = useState({
    code: "",
    name: "",
    category_id: "",
    unit: "un",
    price: "",
    cost: "",
    min_stock: "",
    initial_stock: "",
    track_stock: true,
  });
  const [error, setError] = useState<string | null>(null);
  const [fields, setFields] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const set = (k: keyof typeof form) => (e: { target: { value: string } }) => setForm({ ...form, [k]: e.target.value });

  async function submit(e: FormEvent) {
    e.preventDefault();
    const bad: Record<string, string> = {};
    const num = (k: "price" | "cost" | "min_stock" | "initial_stock", required = false) => {
      if (!form[k].trim()) {
        if (required) bad[k] = "Obrigatório";
        return null;
      }
      const v = parseDecimal(form[k]);
      if (v == null) bad[k] = "Número inválido";
      return v;
    };
    const price = num("price", true);
    const cost = num("cost");
    const min = num("min_stock");
    const initial = num("initial_stock");
    if (Object.keys(bad).length) return setFields(bad);
    setBusy(true);
    setError(null);
    setFields({});
    try {
      onDone(
        await productsApi.create({
          code: form.code,
          name: form.name,
          category_id: form.category_id || null,
          unit: form.unit,
          price: price ?? "0",
          cost,
          min_stock: min,
          initial_stock: form.track_stock ? initial : null,
          track_stock: form.track_stock,
        }),
      );
    } catch (err) {
      setError(errorText(err, "Não foi possível cadastrar."));
      if (err instanceof ApiError) setFields(err.fields);
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="form-grid">
        <Field label="Nome" required autoFocus value={form.name} error={fields.name} onChange={set("name")} />
        <Field label="Código" hint="Vazio = automático" value={form.code} error={fields.code} onChange={set("code")} />
        <div className="field">
          <label htmlFor="np-cat">Categoria</label>
          <select id="np-cat" value={form.category_id} onChange={set("category_id")}>
            <option value="">Sem categoria</option>
            {categories.map((c) => (
              <option key={c.id} value={c.id}>{c.name}</option>
            ))}
          </select>
        </div>
        <div className="field">
          <label htmlFor="np-unit">Unidade</label>
          <select id="np-unit" value={form.unit} onChange={set("unit")}>
            {UNIT_OPTIONS.map((u) => (
              <option key={u.value} value={u.value}>{u.label}</option>
            ))}
          </select>
        </div>
        <Field label="Preço de venda (R$)" inputMode="decimal" required value={form.price} error={fields.price} onChange={set("price")} />
        <Field label="Custo (R$)" inputMode="decimal" value={form.cost} error={fields.cost} onChange={set("cost")} />
        {form.track_stock && (
          <>
            <Field label="Estoque inicial" inputMode="decimal" value={form.initial_stock} error={fields.initial_stock} onChange={set("initial_stock")} />
            <Field label="Estoque mínimo" inputMode="decimal" hint="Abaixo disso, o produto é destacado" value={form.min_stock} error={fields.min_stock} onChange={set("min_stock")} />
          </>
        )}
      </div>
      <label className="check">
        <input type="checkbox" checked={form.track_stock} onChange={(e) => setForm({ ...form, track_stock: e.target.checked })} />
        Controlar estoque deste produto (desmarque para serviços)
      </label>
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>{busy ? "Salvando…" : "Cadastrar produto"}</button>
      </div>
    </form>
  );
}

function StockForm({ products, onDone }: { products: Product[]; onDone: (p: Product, text: string) => void }) {
  const [productId, setProductId] = useState(products[0]?.id ?? "");
  const [kind, setKind] = useState<"entrada" | "saida" | "ajuste">("entrada");
  const [amount, setAmount] = useState("");
  const [cost, setCost] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const product = products.find((p) => p.id === productId);

  async function submit(e: FormEvent) {
    e.preventDefault();
    const q = parseDecimal(amount);
    const c = cost.trim() ? parseDecimal(cost) : null;
    if (q == null || (cost.trim() && c == null)) return setError("Confira os números digitados.");
    setBusy(true);
    setError(null);
    try {
      const p = await productsApi.moveStock({ product_id: productId, kind, qty: q, unit_cost: c, note: note || null });
      onDone(p, `${MOVEMENT_LABEL[kind]} registrada: ${p.name} agora tem ${qty(p.stock)} ${p.unit}.`);
    } catch (err) {
      setError(errorText(err, "Não foi possível registrar."));
    } finally {
      setBusy(false);
    }
  }

  if (!products.length) return <p className="muted">Nenhum produto ativo controla estoque.</p>;
  return (
    <form className="stack" onSubmit={submit}>
      {error && <Notice tone="error">{error}</Notice>}
      <div className="field">
        <label htmlFor="sm-prod">Produto</label>
        <select id="sm-prod" value={productId} onChange={(e) => setProductId(e.target.value)}>
          {products.map((p) => (
            <option key={p.id} value={p.id}>
              {p.code} · {p.name} (estoque {qty(p.stock)} {p.unit})
            </option>
          ))}
        </select>
      </div>
      <div className="segmented" role="radiogroup" aria-label="Tipo">
        {(["entrada", "saida", "ajuste"] as const).map((k) => (
          <label key={k} className={kind === k ? "is-on" : undefined}>
            <input type="radio" name="kind" checked={kind === k} onChange={() => setKind(k)} />
            {MOVEMENT_LABEL[k]}
          </label>
        ))}
      </div>
      <Field
        label={kind === "ajuste" ? `Quantidade contada (${product?.unit ?? ""})` : `Quantidade (${product?.unit ?? ""})`}
        hint={kind === "ajuste" ? `Hoje o sistema registra ${qty(product?.stock)}; o saldo passa a ser o contado.` : undefined}
        inputMode="decimal"
        required
        autoFocus
        value={amount}
        onChange={(e) => setAmount(e.target.value)}
      />
      {kind === "entrada" && (
        <Field label="Custo unitário (R$)" hint="Opcional: atualiza o custo do produto" inputMode="decimal" value={cost} onChange={(e) => setCost(e.target.value)} />
      )}
      <Field label="Observação" placeholder="Ex.: nota fiscal 123, perda, contagem mensal" value={note} onChange={(e) => setNote(e.target.value)} />
      <div className="row row--end">
        <button className="btn btn--primary" disabled={busy}>{busy ? "Salvando…" : "Registrar"}</button>
      </div>
    </form>
  );
}

function CategoriesManager({ categories, onChange }: { categories: Category[]; onChange: () => Promise<void> }) {
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function run(action: () => Promise<unknown>) {
    setError(null);
    try {
      await action();
      await onChange();
    } catch (e) {
      setError(errorText(e, "Não foi possível salvar."));
    }
  }

  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) void run(async () => { await productsApi.createCategory(name); setName(""); });
        }}
      >
        <input aria-label="Nova categoria" placeholder="Nova categoria" value={name} onChange={(e) => setName(e.target.value)} />
        <button className="btn btn--primary">Adicionar</button>
      </form>
      <ul className="plain-list">
        {categories.map((c) => (
          <li key={c.id}>
            <span>
              {c.name} <span className="muted">· {c.product_count} produto(s)</span>
            </span>
            <span className="row">
              <button type="button" className="btn btn--ghost" onClick={() => {
                const n = window.prompt("Novo nome da categoria", c.name);
                if (n && n.trim() && n !== c.name) void run(() => productsApi.renameCategory(c.id, n));
              }}>Renomear</button>
              <button type="button" className="btn btn--ghost" disabled={c.product_count > 0}
                title={c.product_count > 0 ? "Mova os produtos antes de excluir" : undefined}
                onClick={() => void run(() => productsApi.deleteCategory(c.id))}>Excluir</button>
            </span>
          </li>
        ))}
        {!categories.length && <li className="muted">Nenhuma categoria ainda.</li>}
      </ul>
    </div>
  );
}

const IMPORT_LABEL: Record<ImportField, string> = {
  name: "Nome *",
  code: "Código",
  price: "Preço de venda",
  cost: "Custo",
  stock: "Estoque",
  min_stock: "Estoque mínimo",
  category: "Categoria",
  unit: "Unidade",
  barcode: "Código de barras",
};

function ImportWizard({ onDone }: { onDone: (r: ImportResult) => Promise<void> }) {
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [mapping, setMapping] = useState<Partial<Record<ImportField, number>>>({});
  const [updateExisting, setUpdateExisting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function choose() {
    const file = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Planilhas", extensions: ["xlsx", "xls", "ods", "csv"] }],
    });
    if (typeof file !== "string") return;
    setError(null);
    setResult(null);
    try {
      const p = await sheetApi.preview(file);
      setPath(file);
      setPreview(p);
      setMapping(p.suggested);
    } catch (e) {
      setError(errorText(e, "Não foi possível ler o arquivo."));
    }
  }

  async function run() {
    if (!path) return;
    setBusy(true);
    setError(null);
    try {
      const r = await sheetApi.importProducts({ path, mapping, update_existing: updateExisting });
      setResult(r);
      await onDone(r);
    } catch (e) {
      setError(errorText(e, "Não foi possível importar."));
    } finally {
      setBusy(false);
    }
  }

  if (result) {
    return (
      <div className="stack">
        <Notice tone="success">
          {result.created} produto(s) novo(s), {result.updated} atualizado(s), {result.skipped} ignorado(s) por já existirem.
        </Notice>
        {result.errors.length > 0 && (
          <>
            <p><strong>{result.errors.length} linha(s) não importada(s):</strong></p>
            <ul className="plain-list small">
              {result.errors.slice(0, 50).map((e) => (
                <li key={e.row}>Linha {e.row}: {e.message}</li>
              ))}
            </ul>
          </>
        )}
      </div>
    );
  }

  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <p className="muted">
        Aceita .xlsx, .xls, .ods ou .csv. A primeira linha deve ter os nomes das colunas. Números podem estar
        no formato brasileiro (1.234,56). As categorias que não existirem são criadas.
      </p>
      <div className="row">
        <button type="button" className="btn btn--ghost" onClick={() => void choose()}>
          {path ? "Escolher outro arquivo" : "Escolher arquivo…"}
        </button>
        {path && <span className="muted small">{path.split(/[\\/]/).pop()} · {preview?.total_rows} linha(s)</span>}
      </div>

      {preview && (
        <>
          <h3>Qual coluna é cada informação?</h3>
          <div className="form-grid">
            {(Object.keys(IMPORT_LABEL) as ImportField[]).map((f) => (
              <div className="field" key={f}>
                <label htmlFor={`map-${f}`}>{IMPORT_LABEL[f]}</label>
                <select
                  id={`map-${f}`}
                  value={mapping[f] ?? ""}
                  onChange={(e) => {
                    const next = { ...mapping };
                    if (e.target.value === "") delete next[f];
                    else next[f] = Number(e.target.value);
                    setMapping(next);
                  }}
                >
                  <option value="">(não importar)</option>
                  {preview.headers.map((h, i) => (
                    <option key={i} value={i}>{h || `Coluna ${i + 1}`}</option>
                  ))}
                </select>
              </div>
            ))}
          </div>
          <div className="table-wrap">
            <table className="table small">
              <thead>
                <tr>{preview.headers.map((h, i) => <th key={i}>{h}</th>)}</tr>
              </thead>
              <tbody>
                {preview.sample.slice(0, 5).map((r, i) => (
                  <tr key={i}>{preview.headers.map((_, j) => <td key={j}>{r[j]}</td>)}</tr>
                ))}
              </tbody>
            </table>
          </div>
          <label className="check">
            <input type="checkbox" checked={updateExisting} onChange={(e) => setUpdateExisting(e.target.checked)} />
            Atualizar produtos que já existem com o mesmo código (o estoque vira o da planilha)
          </label>
          <div className="row row--end">
            <button className="btn btn--primary" disabled={busy || mapping.name == null} onClick={() => void run()}>
              {busy ? "Importando…" : `Importar ${preview.total_rows} linha(s)`}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function MovementHistory({ product }: { product: Product }) {
  const { data, error } = useAsync(() => productsApi.movements({ product_id: product.id, limit: 500 }), [product.id]);
  const columns: GridColumn<Movement>[] = [
    { key: "created_at", label: "Quando", kind: "datetime", get: (m) => m.created_at },
    { key: "kind", label: "Tipo", get: (m) => MOVEMENT_LABEL[m.kind] ?? m.kind },
    { key: "qty", label: "Quantidade", kind: "number", get: (m) => m.qty },
    { key: "balance", label: "Saldo", kind: "number", get: (m) => m.balance },
    { key: "unit_cost", label: "Custo un.", kind: "money", get: (m) => m.unit_cost },
    { key: "note", label: "Observação", get: (m) => m.note },
    { key: "user_name", label: "Quem", get: (m) => m.user_name },
  ];
  return (
    <div className="stack">
      {error && <Notice tone="error">{error}</Notice>}
      <p className="muted">
        Estoque atual: <strong>{qty(product.stock)} {product.unit}</strong> · Preço {money(product.price)}
        {product.cost != null && <> · Custo {money(product.cost)} · Margem {pct(marginOf(product))}</>}
      </p>
      <DataGrid rows={data ?? []} columns={columns} rowKey={(m) => m.id} exportName={`Estoque-${product.code}`} maxHeight="50vh" emptyText="Sem movimentações." />
    </div>
  );
}
