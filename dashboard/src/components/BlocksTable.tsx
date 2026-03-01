import React from "react";
import { BlockRow } from "../api";

type Props = { blocks: BlockRow[] };

export default function BlocksTable({ blocks }: Props) {
  return (
    <div className="card table-card">
      <div className="table-card-header">
        <div className="card-label">Recent Blocks</div>
      </div>
      <div className="table-scroll">
      <table>
        <thead>
          <tr>
            <th>Height</th>
            <th>Hash</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {blocks.map((b) => (
            <tr key={`${b.height}-${b.hash}`}>
              <td className="num-cyan">{b.height.toLocaleString()}</td>
              <td className="mono num-dim">{b.hash.slice(0, 20)}…</td>
              <td>
                <span className={`status-pill status-${b.status}`}>{b.status}</span>
              </td>
            </tr>
          ))}
          {blocks.length === 0 && (
            <tr><td colSpan={3} className="empty">No blocks submitted yet</td></tr>
          )}
        </tbody>
      </table>
      </div>
    </div>
  );
}
