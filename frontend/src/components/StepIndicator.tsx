import { Steps } from "@douyinfe/semi-ui";

interface StepIndicatorProps {
  current: number;
}

const STEP_ITEMS = [
  { title: "Database Connection", description: "Enter credentials" },
  { title: "Data Source", description: "Select table or SQL" },
  { title: "Field Config", description: "Select and rename fields" },
  { title: "Confirm", description: "Review and save" },
];

export function StepIndicator({ current }: StepIndicatorProps) {
  return (
    <Steps current={current} size="small" style={{ marginBottom: 24 }}>
      {STEP_ITEMS.map((item) => (
        <Steps.Step
          key={item.title}
          title={item.title}
          description={item.description}
        />
      ))}
    </Steps>
  );
}
