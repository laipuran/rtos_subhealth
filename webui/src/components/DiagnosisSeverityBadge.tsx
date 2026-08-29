import type { DiagnosisSeverity } from "../types/diagnosis"

const colorMap: Record<DiagnosisSeverity, string> = {
  normal: "text-green-600 bg-green-100",
  mild: "text-yellow-600 bg-yellow-100",
  moderate: "text-orange-600 bg-orange-100",
  severe: "text-red-600 bg-red-100",
  critical: "text-red-700 bg-red-200",
}

export default function DiagnosisSeverityBadge({ severity }: { severity: DiagnosisSeverity }) {
  return (
    <span
      className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${colorMap[severity] || "bg-gray-100"}`}
    >
      {severity}
    </span>
  )
}
