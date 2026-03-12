interface StepIndicatorProps {
  current: number;
}

const STEPS = ["连接", "数据源", "字段", "确认"];

export function StepIndicator({ current }: StepIndicatorProps) {
  return (
    <div className="step-nav">
      {STEPS.map((label, i) => {
        const cls =
          i < current ? "step-item done" : i === current ? "step-item active" : "step-item";
        return (
          <div key={label} style={{ display: "flex", alignItems: "center" }}>
            {i > 0 && <div className="step-line" />}
            <div className={cls}>
              <div className="step-dot">
                {i < current ? "\u2713" : i + 1}
              </div>
              {label}
            </div>
          </div>
        );
      })}
    </div>
  );
}
