import React from "react";
import {
  AreaChart, Area, ResponsiveContainer,
  Tooltip, XAxis, YAxis, CartesianGrid
} from "recharts";

type Point = { timestamp: string; shares: number };

type Props = { data: Point[] };

const CustomTooltip = ({ active, payload, label }: any) => {
  if (active && payload?.length) {
    return (
      <div style={{
        background: "#0c0f1e", border: "1px solid #2a3560",
        borderRadius: 8, padding: "8px 12px", fontSize: 12,
      }}>
        <div style={{ color: "#6c7a9c", marginBottom: 4 }}>{label}</div>
        <div style={{ color: "#00d4ff", fontFamily: "JetBrains Mono, monospace", fontWeight: 600 }}>
          {payload[0].value} shares
        </div>
      </div>
    );
  }
  return null;
};

export default function HashrateChart({ data }: Props) {
  return (
    <div className="card chart-card">
      <div className="card-label">Share Rate — Last 60 Min</div>
      <ResponsiveContainer width="100%" height={210}>
        <AreaChart data={data} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
          <defs>
            <linearGradient id="cyanGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#00d4ff" stopOpacity={0.25} />
              <stop offset="95%" stopColor="#00d4ff" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="#1a2040" vertical={false} />
          <XAxis
            dataKey="timestamp"
            tick={{ fontSize: 10, fill: "#3d4a6b" }}
            axisLine={false} tickLine={false}
            interval="preserveStartEnd"
          />
          <YAxis
            tick={{ fontSize: 10, fill: "#3d4a6b" }}
            axisLine={false} tickLine={false}
          />
          <Tooltip content={<CustomTooltip />} cursor={{ stroke: "#2a3560", strokeWidth: 1 }} />
          <Area
            type="monotone" dataKey="shares"
            stroke="#00d4ff" strokeWidth={2}
            fill="url(#cyanGrad)" dot={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  );
}
