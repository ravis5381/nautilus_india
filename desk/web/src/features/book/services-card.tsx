import { useEffect, useRef, useState } from "react"

import { Badge } from "@/components/ui/badge"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { fetchServices, SignInRequired, type UnitStatus } from "@/lib/api"

type ServicesCardProps = {
  onSignOut: () => void
}

export function ServicesCard({ onSignOut }: ServicesCardProps) {
  const [units, setUnits] = useState<UnitStatus[]>([])
  const [message, setMessage] = useState<string | null>(null)
  const signOut = useRef(onSignOut)

  useEffect(() => {
    signOut.current = onSignOut
  }, [onSignOut])

  useEffect(() => {
    let cancelled = false
    async function load() {
      try {
        const report = await fetchServices()
        if (cancelled) return
        setUnits(report.units)
        setMessage(report.message)
      } catch (caught) {
        if (cancelled) return
        if (caught instanceof SignInRequired) {
          signOut.current()
          return
        }
        setUnits([])
        setMessage(
          caught instanceof Error
            ? caught.message
            : "The service list could not be loaded.",
        )
      }
    }
    void load()
    const timer = window.setInterval(() => void load(), 5000)
    return () => {
      cancelled = true
      window.clearInterval(timer)
    }
  }, [])

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Services</CardTitle>
      </CardHeader>
      <CardContent>
        {units.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {message ?? "No systemd units are running."}
          </p>
        ) : (
          <div className="grid gap-3">
            {message ? (
              <p className="text-sm text-muted-foreground">{message}</p>
            ) : null}
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Unit</TableHead>
                  <TableHead>State</TableHead>
                  <TableHead>Since</TableHead>
                  <TableHead>PID</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {units.map((unit) => (
                  <TableRow key={unit.name}>
                    <TableCell>
                      <div className="grid gap-0.5">
                        <span className="font-mono">{unit.name}</span>
                        {unit.description ? (
                          <span className="text-muted-foreground">
                            {unit.description}
                          </span>
                        ) : null}
                      </div>
                    </TableCell>
                    <TableCell>
                      <StateBadge unit={unit} />
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {unit.since || "—"}
                    </TableCell>
                    <TableCell className="font-mono tabular-nums">
                      {unit.pid ?? "—"}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

function StateBadge({ unit }: { unit: UnitStatus }) {
  const label = unit.load === "not-found" ? "not found" : unit.sub || unit.active
  const running = unit.active === "active" && unit.sub === "running"
  const failed = unit.active === "failed" || unit.sub === "failed"
  return (
    <Badge
      variant="secondary"
      className={
        running
          ? "text-emerald-700 dark:text-emerald-400"
          : failed
            ? "text-red-600 dark:text-red-400"
            : undefined
      }
    >
      {label}
    </Badge>
  )
}
