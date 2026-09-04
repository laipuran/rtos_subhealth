import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  ResponsiveContainer,
  Legend,
  Tooltip,
} from "recharts"
import { useVitals } from "../context/VitalsContext"
import { DATA_TYPE_ORDER, metaOf } from "../utils/physio"

interface RawPoint {
  t: number
  raw: Record<string, number>
}

const MAX_POINTS = 60 // 1 Hz → 最近 ~60s

function normalize(points: RawPoint[]): { data: any[]; types: string[] } {
  const types = DATA_TYPE_ORDER.filter((t) => points.some((p) => p.raw[t] != null))
  const ranges: Record<string, [number, number]> = {}
  for (const t of types) {
    let mn = Infinity
    let mx = -Infinity
    for (const p of points) {
      const v = p.raw[t]
      if (v != null) {
        mn = Math.min(mn, v)
        mx = Math.max(mx, v)
      }
    }
    if (mn === Infinity) {
      mn = 0
      mx = 1
    }
    if (mx - mn < 1e-6) mx = mn + 1
    ranges[t] = [mn, mx]
  }
  const data = points.map((p) => {
    const o: any = { t: p.t, raw: p.raw }
    for (const t of types) {
      const [mn, mx] = ranges[t]
      const v = p.raw[t]
      o[t] = v == null ? null : ((v - mn) / (mx - mn)) * 100
    }
    return o
  })
  return { data, types }
}

export default function VitalsChart() {
  const { points } = useVitals()
  const { data, types } = normalize(points)

  return (
    <div className="border rounded bg-white p-3">
      <div className="flex items-center justify-between mb-1">
        <h3 className="text-sm font-medium text-gray-700">实时体征趋势</h3>
        <span className="text-xs text-gray-400">最近 {MAX_POINTS}s · 归一化显示</span>
      </div>
      {types.length === 0 ? (
        <div className="h-48 flex items-center justify-center text-sm text-gray-400">
          等待体征数据…
        </div>
      ) : (
        <ResponsiveContainer width="100%" height={200}>
          <LineChart data={data} margin={{ top: 4, right: 8, left: -8, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#f0f0f0" />
            <XAxis
              dataKey="t"
              hide
              type="number"
              domain={["dataMin", "dataMax"]}
            />
            <YAxis domain={[0, 100]} tick={{ fontSize: 10 }} />
            <Tooltip
              content={({ active, payload }) => {
                if (!active || !payload || !payload.length) return null
                const raw: Record<string, number> = payload[0].payload.raw
                return (
                  <div className="rounded border bg-white px-2 py-1 text-xs shadow">
                    {types.map((t) => (
                      <div key={t} className="flex items-center gap-2">
                        <span
                          className="inline-block h-2 w-2 rounded-full"
                          style={{ background: metaOf(t).color }}
                        />
                        <span className="text-gray-600">{metaOf(t).label}</span>
                        <span className="font-mono">
                          {raw[t] != null ? raw[t].toFixed(1) : "-"}
                          {metaOf(t).unit}
                        </span>
                      </div>
                    ))}
                  </div>
                )
              }}
            />
            <Legend wrapperStyle={{ fontSize: 11 }} />
            {types.map((t) => (
              <Line
                key={t}
                type="monotone"
                dataKey={t}
                stroke={metaOf(t).color}
                strokeWidth={1.8}
                dot={false}
                isAnimationActive={false}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      )}
    </div>
  )
}
