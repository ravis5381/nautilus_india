export type Side = "BUY" | "SELL"

export type Position = {
  strategy: string
  instrument_id: string
  quantity: number
  side: Side | null
  avg_price: string
  realized_pnl: string
}

export async function fetchPositions(): Promise<Position[]> {
  const response = await fetch("/api/positions")
  if (!response.ok) {
    throw new Error("The position book could not be loaded.")
  }
  return response.json() as Promise<Position[]>
}
