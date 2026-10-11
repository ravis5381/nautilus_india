import { useEffect, useMemo, useState } from "react"

import { fetchPositions, type Position } from "@/lib/api"

const STORAGE_KEY = "desk.bookStrategy"

export type BookStatus = "loading" | "ready" | "error"

export function useBook() {
  const [book, setBook] = useState<Position[]>([])
  const [selected, setSelected] = useState(
    () => localStorage.getItem(STORAGE_KEY) ?? "",
  )
  const [status, setStatus] = useState<BookStatus>("loading")

  useEffect(() => {
    let cancelled = false
    fetchPositions()
      .then((rows) => {
        if (cancelled) return
        setBook(rows)
        setStatus("ready")
      })
      .catch(() => {
        if (!cancelled) setStatus("error")
      })
    return () => {
      cancelled = true
    }
  }, [])

  const strategies = useMemo(() => {
    return [
      ...new Set(
        book.filter((row) => row.quantity !== 0).map((row) => row.strategy),
      ),
    ].sort()
  }, [book])

  const active = strategies.includes(selected) ? selected : (strategies[0] ?? "")

  useEffect(() => {
    if (active) localStorage.setItem(STORAGE_KEY, active)
  }, [active])

  const rows = book.filter((row) => row.strategy === active)

  function choose(strategy: string) {
    setSelected(strategy)
    localStorage.setItem(STORAGE_KEY, strategy)
  }

  return { rows, strategies, selected: active, choose, status }
}
