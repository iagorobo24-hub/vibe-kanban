import { cn } from "../../lib/cn";

export interface AgentOSMetricProps {
  label: string;
  value: string | number;
  detail?: string;
  tone?: "neutral" | "success" | "attention" | "danger" | "info";
  className?: string;
}

export function AgentOSMetric({
  label,
  value,
  detail,
  tone = "neutral",
  className,
}: AgentOSMetricProps) {
  return (
    <div className={cn("agentos-metric", className)}>
      <span className="agentos-metric__label">{label}</span>
      <strong className={cn("agentos-metric__value", `text-${tone}`)}>
        {value}
      </strong>
      {detail && <span className="agentos-metric__detail">{detail}</span>}
    </div>
  );
}
