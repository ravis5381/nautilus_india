import { Badge } from "@/components/ui/badge"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import type { Position } from "@/lib/api"
import { formatQuantity } from "@/lib/format"

import type { BookStatus } from "./use-book"

type PositionTableProps = {
  rows: Position[]
  strategy: string
  status: BookStatus
}

export function PositionTable({ rows, strategy, status }: PositionTableProps) {
  const open = rows.filter((row) => row.quantity !== 0)

  if (status === "loading") {
    return <p className="text-muted-foreground">Loading positions.</p>
  }
  if (status === "error") {
    return (
      <p className="text-muted-foreground">
        The position book could not be loaded.
      </p>
    )
  }
  if (!strategy) {
    return <p className="text-muted-foreground">No open position.</p>
  }
  if (open.length === 0) {
    return (
      <p className="text-muted-foreground">
        No open position in {strategy}.
      </p>
    )
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Instrument</TableHead>
          <TableHead>Quantity</TableHead>
          <TableHead>Average</TableHead>
          <TableHead>Realized</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {open.map((row) => (
          <TableRow key={row.instrument_id}>
            <TableCell className="font-mono">{row.instrument_id}</TableCell>
            <TableCell>
              <Quantity quantity={row.quantity} />
            </TableCell>
            <TableCell className="font-mono tabular-nums">
              {row.avg_price}
            </TableCell>
            <TableCell className="font-mono tabular-nums">
              {row.realized_pnl}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function Quantity({ quantity }: { quantity: number }) {
  const long = quantity > 0
  return (
    <Badge
      variant="secondary"
      className={
        long
          ? "font-mono text-emerald-700 dark:text-emerald-400"
          : "font-mono text-red-600 dark:text-red-400"
      }
    >
      {formatQuantity(quantity)}
    </Badge>
  )
}
