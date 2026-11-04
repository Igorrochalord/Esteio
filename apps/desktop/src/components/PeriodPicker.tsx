import { LABELS, presetPeriod, type Period, type PresetKey } from "../services/period";

export function PeriodPicker({ value, onChange }: { value: Period; onChange: (p: Period) => void }) {
  return (
    <div className="period">
      <select
        aria-label="Período"
        value={value.preset}
        onChange={(e) => {
          const preset = e.target.value as PresetKey;
          onChange(preset === "custom" ? { ...value, preset } : presetPeriod(preset));
        }}
      >
        {(Object.keys(LABELS) as PresetKey[]).map((k) => (
          <option key={k} value={k}>
            {LABELS[k]}
          </option>
        ))}
      </select>
      {value.preset === "custom" && (
        <>
          <input
            type="date"
            aria-label="De"
            value={value.from}
            max={value.to}
            onChange={(e) => e.target.value && onChange({ ...value, from: e.target.value })}
          />
          <span className="muted">a</span>
          <input
            type="date"
            aria-label="Até"
            value={value.to}
            min={value.from}
            onChange={(e) => e.target.value && onChange({ ...value, to: e.target.value })}
          />
        </>
      )}
    </div>
  );
}
