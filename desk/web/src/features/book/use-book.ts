import { useEffect, useMemo, useState } from "react"

import { fetchPositions, signOut, SignInRequired, type Position } from "@/lib/api"
import { formatPrice } from "@/lib/format"

const STORAGE_KEY = "desk.bookStrategy"
export const ALL_STRATEGIES = "all"

export type BookStatus = "loading" | "unauthorized" | "ready" | "error"

export function useBook() {
  const [book, setBook] = useState<Position[]>([])
  const [selected, setSelected] = useState(
    () => localStorage.getItem(STORAGE_KEY) ?? "",
  )
  const [status, setStatus] = useState<BookStatus>("loading")
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    let cancelled = false
    fetchPositions()
      .then((rows) => {
        if (cancelled) return
        setBook(rows)
        setStatus("ready")
      })
      .catch((error: unknown) => {
        if (cancelled) return
        setBook([])
        setStatus(error instanceof SignInRequired ? "unauthorized" : "error")
      })
    return () => {
      cancelled = true
    }
  }, [attempt])

  const strategies = useMemo(() => {
    const names = [
      ...new Set(
        book.filter((row) => row.quantity !== 0).map((row) => row.strategy),
      ),
    ].sort()
    return names.length === 0 ? [] : [ALL_STRATEGIES, ...names]
  }, [book])

  const names = strategies.filter((name) => name !== ALL_STRATEGIES)
  const active =
    selected === ALL_STRATEGIES && names.length > 0
      ? ALL_STRATEGIES
      : names.includes(selected)
        ? selected
        : (names[0] ?? "")

  useEffect(() => {
    if (active) localStorage.setItem(STORAGE_KEY, active)
  }, [active])

  const rows =
    active === ALL_STRATEGIES
      ? aggregateByInstrument(book)
      : book.filter((row) => row.strategy === active)

  function choose(strategy: string) {
    setSelected(strategy)
    localStorage.setItem(STORAGE_KEY, strategy)
  }

  function reload() {
    setStatus("loading")
    setAttempt((value) => value + 1)
  }

  function refresh() {
    setAttempt((value) => value + 1)
  }

  async function lock() {
    await signOut()
    setBook([])
    setStatus("unauthorized")
  }

  return { rows, strategies, selected: active, choose, status, reload, refresh, lock }
}

function aggregateByInstrument(rows: Position[]): Position[] {
  const grouped = new Map<string, Position[]>()
  for (const row of rows) {
    const group = grouped.get(row.instrument_id) ?? []
    group.push(row)
    grouped.set(row.instrument_id, group)
  }

  return [...grouped.keys()].sort().map((instrumentId) => {
    const group = grouped.get(instrumentId) ?? []
    let longQuantity = 0
    let longCost = 0
    let shortQuantity = 0
    let shortCost = 0
    let realized = 0
    for (const row of group) {
      const average = Number(row.avg_price)
      realized += Number(row.realized_pnl)
      if (row.quantity > 0) {
        longQuantity += row.quantity
        longCost += row.quantity * average
      } else if (row.quantity < 0) {
        const size = -row.quantity
        shortQuantity += size
        shortCost += size * average
      }
    }
    const quantity = longQuantity - shortQuantity
    const average =
      quantity > 0
        ? longCost / longQuantity
        : quantity < 0
          ? shortCost / shortQuantity
          : 0
    return {
      strategy: ALL_STRATEGIES,
      instrument_id: instrumentId,
      quantity,
      side: quantity > 0 ? "BUY" : quantity < 0 ? "SELL" : null,
      avg_price: formatPrice(average),
      realized_pnl: formatPrice(realized),
    }
  })
}
