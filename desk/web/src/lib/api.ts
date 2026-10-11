export type Side = "BUY" | "SELL"

export type Position = {
  strategy: string
  instrument_id: string
  quantity: number
  side: Side | null
  avg_price: string
  realized_pnl: string
}

export class SignInRequired extends Error {
  constructor() {
    super("Sign in to the desk.")
  }
}

export async function fetchPositions(): Promise<Position[]> {
  const response = await fetch("/api/positions")
  if (response.status === 401) {
    throw new SignInRequired()
  }
  if (!response.ok) {
    throw new Error("The position book could not be loaded.")
  }
  return response.json() as Promise<Position[]>
}

export async function signOut(): Promise<void> {
  await fetch("/api/logout", { method: "POST" })
}
