import React from "react";

type Accent = "cyan" | "green" | "purple" | "orange" | "yellow" | "red";

type Props = {
  label: string;
  value: string;
  sub?: string;
  icon?: string;
  accent?: Accent;
  valueColor?: Accent | "text";
};

export default function StatsCard({ label, value, sub, icon, accent = "cyan", valueColor }: Props) {
  const vc = valueColor ?? accent;
  return (
    <div className={`card card-accent-${accent}`}>
      {icon && <span className="card-icon">{icon}</span>}
      <div className="card-label">{label}</div>
      <div className={`card-value card-value-${vc}`}>{value}</div>
      {sub && <div className="card-sub">{sub}</div>}
    </div>
  );
}
