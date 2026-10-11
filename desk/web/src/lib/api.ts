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

export type UnitStatus = {
  name: string
  description: string
  active: string
  sub: string
  load: string
  file_state: string
  since: string
  pid: number | null
}

export type ServiceReport = {
  available: boolean
  message: string | null
  units: UnitStatus[]
}

export async function fetchServices(): Promise<ServiceReport> {
  const response = await fetch("/api/services")
  if (response.status === 401) {
    throw new SignInRequired()
  }
  if (!response.ok) {
    throw new Error("The service list could not be loaded.")
  }
  return response.json() as Promise<ServiceReport>
}

export async function signOut(): Promise<void> {
  await fetch("/api/logout", { method: "POST" })
}

export type PositionUpdate = {
  strategy: string
  instrument_id: string
  quantity: number
  avg_price: string
  realized_pnl: string
}

export async function setPosition(update: PositionUpdate): Promise<Position[]> {
  const response = await fetch("/api/positions", {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify([update]),
  })
  if (response.status === 401) {
    throw new SignInRequired()
  }
  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as {
      error?: string
    } | null
    throw new Error(payload?.error ?? "The position could not be set.")
  }
  return response.json() as Promise<Position[]>
}
