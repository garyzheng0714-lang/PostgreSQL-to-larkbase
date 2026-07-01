import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { SOURCE_TYPES, getSource } from "../lib/sourceTypes";
import { Button } from "./ui/Button";

interface SourceTypePickerProps {
  value: string;
  onChange: (id: string) => void;
}

const ENABLED = SOURCE_TYPES.map((s, i) => (s.active ? i : -1)).filter(
  (i) => i >= 0
);

/**
 * 数据源类型选择器。6 项中仅 PostgreSQL 可选，其余「即将支持」。
 * 自建小下拉（项少且多禁用）—— 完全控制芯片/即将支持的视觉，键盘可达（ArrowUp/Down/Enter/Esc）。
 */
export function SourceTypePicker({ value, onChange }: SourceTypePickerProps) {
  const [open, setOpen] = useState(false);
  const [activeIdx, setActiveIdx] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);
  const current = getSource(value);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, [open]);

  const openMenu = () => {
    const cur = SOURCE_TYPES.findIndex((s) => s.id === value);
    setActiveIdx(cur >= 0 ? cur : ENABLED[0] ?? 0);
    setOpen(true);
  };

  const select = (idx: number) => {
    const s = SOURCE_TYPES[idx];
    if (!s || !s.active) return;
    onChange(s.id);
    setOpen(false);
  };

  const moveActive = (dir: 1 | -1) => {
    if (!ENABLED.length) return;
    const pos = ENABLED.indexOf(activeIdx);
    const next =
      pos === -1
        ? ENABLED[0]
        : ENABLED[(pos + dir + ENABLED.length) % ENABLED.length];
    setActiveIdx(next);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(e.key)) {
        e.preventDefault();
        openMenu();
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        moveActive(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveActive(-1);
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        select(activeIdx);
        break;
      case "Escape":
      case "Tab":
        setOpen(false);
        break;
    }
  };

  return (
    <div className="db-src" ref={rootRef} style={{ position: "relative" }}>
      <Button
        className={`db-box db-box--button${open ? " db-box--focus" : ""}`}
        aria-haspopup="listbox"
        aria-expanded={open}
        onKeyDown={onKeyDown}
        onClick={() => (open ? setOpen(false) : openMenu())}
      >
        <span className="db-srcchip" aria-hidden>
          {current.badge}
        </span>
        <span className="db-src__name">{current.name}</span>
        <ChevronDown
          className="db-src__chevron"
          style={{ transform: open ? "rotate(180deg)" : undefined }}
        />
      </Button>

      {open && (
        <div className="db-srcmenu" role="listbox">
          {SOURCE_TYPES.map((s, i) => (
            <Button
              key={s.id}
              role="option"
              aria-selected={s.id === value}
              className={`db-srcmenu__item${s.id === value ? " is-selected" : ""}${
                s.active ? "" : " is-disabled"
              }${i === activeIdx && s.active ? " is-active-opt" : ""}`}
              disabled={!s.active}
              onMouseEnter={() => s.active && setActiveIdx(i)}
              onClick={() => select(i)}
            >
              <span className="db-srcchip" aria-hidden>
                {s.badge}
              </span>
              <span className="db-src__name">{s.name}</span>
              {!s.active && <span className="db-srcmenu__soon">即将支持</span>}
            </Button>
          ))}
        </div>
      )}
    </div>
  );
}
