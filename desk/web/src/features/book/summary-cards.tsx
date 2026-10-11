import type { Position } from "@/lib/api"
import { formatAmount } from "@/lib/format"
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

import type { BookStatus } from "./use-book"

type SummaryCardsProps = {
  rows: Position[]
  status: BookStatus
}

export function SummaryCards({ rows, status }: SummaryCardsProps) {
  const open = rows.filter((row) => row.quantity !== 0)
  const net = open.reduce((total, row) => total + row.quantity, 0)
  const realized = rows.reduce((total, row) => total + Number(row.realized_pnl), 0)
  const ready = status === "ready"

  return (
    <section className="grid gap-3 min-[32rem]:grid-cols-3" aria-live="polite">
      <Metric label="Open names" value={ready ? String(open.length) : "—"} />
      <Metric label="Net quantity" value={ready ? String(net) : "—"} />
      <Metric
        label="Realized PnL"
        value={ready ? formatAmount(realized) : "—"}
        tone={ready && realized < 0 ? "short" : "long"}
      />
    </section>
  )
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string
  value: string
  tone?: "long" | "short"
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardDescription className="text-xs tracking-wide uppercase">
          {label}
        </CardDescription>
        <CardTitle
          className={
            tone === "short"
              ? "font-mono text-2xl text-red-600 dark:text-red-400"
              : tone === "long"
                ? "font-mono text-2xl text-emerald-700 dark:text-emerald-400"
                : "font-mono text-2xl"
          }
        >
          {value}
        </CardTitle>
      </CardHeader>
    </Card>
  )
}
