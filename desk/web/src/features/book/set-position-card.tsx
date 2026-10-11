import { useState, type FormEvent, type ReactNode } from "react"

import { setPosition, SignInRequired } from "@/lib/api"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"

import { ALL_STRATEGIES } from "./use-book"

type SetPositionCardProps = {
  strategy: string
  onSaved: (strategy: string) => void
  onSignOut: () => void
}

export function SetPositionCard({ strategy, onSaved, onSignOut }: SetPositionCardProps) {
  const [name, setName] = useState(strategy === ALL_STRATEGIES ? "" : strategy)
  const [instrumentId, setInstrumentId] = useState("")
  const [quantity, setQuantity] = useState("")
  const [avgPrice, setAvgPrice] = useState("")
  const [realizedPnl, setRealizedPnl] = useState("0")
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState<string | null>(null)
  const [pending, setPending] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    const wholeQuantity = Number(quantity)
    if (!Number.isInteger(wholeQuantity)) {
      setSaved(null)
      setError("Quantity must be a whole number.")
      return
    }
    setPending(true)
    setError(null)
    setSaved(null)
    try {
      const [stored] = await setPosition({
        strategy: name,
        instrument_id: instrumentId,
        quantity: wholeQuantity,
        avg_price: avgPrice,
        realized_pnl: realizedPnl.trim() || "0",
      })
      if (!stored) {
        setError("The position could not be set.")
        return
      }
      setName(stored.strategy)
      setInstrumentId(stored.instrument_id)
      setSaved(`${stored.instrument_id} on ${stored.strategy}`)
      onSaved(stored.strategy)
    } catch (caught) {
      if (caught instanceof SignInRequired) {
        onSignOut()
        return
      }
      setError(caught instanceof Error ? caught.message : "The position could not be set.")
    } finally {
      setPending(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Set position</CardTitle>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={submit}>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Strategy" id="set-strategy">
              <Input
                id="set-strategy"
                value={name}
                onChange={(event) => setName(event.target.value)}
                placeholder="opening-drive"
                autoComplete="off"
                required
              />
            </Field>
            <Field label="Instrument" id="set-instrument">
              <Input
                id="set-instrument"
                value={instrumentId}
                onChange={(event) => setInstrumentId(event.target.value)}
                placeholder="NIFTY-20260625-25000-CE.NFO"
                autoComplete="off"
                className="font-mono"
                required
              />
            </Field>
            <Field label="Quantity" id="set-quantity">
              <Input
                id="set-quantity"
                value={quantity}
                onChange={(event) => setQuantity(event.target.value)}
                placeholder="+10 long, -10 short"
                inputMode="numeric"
                required
              />
            </Field>
            <Field label="Average" id="set-average">
              <Input
                id="set-average"
                value={avgPrice}
                onChange={(event) => setAvgPrice(event.target.value)}
                placeholder="2450.05"
                inputMode="decimal"
                required
              />
            </Field>
            <Field label="Realized" id="set-realized">
              <Input
                id="set-realized"
                value={realizedPnl}
                onChange={(event) => setRealizedPnl(event.target.value)}
                inputMode="decimal"
                required
              />
            </Field>
          </div>
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
          {saved ? <p className="text-sm text-muted-foreground">Saved {saved}.</p> : null}
          <div>
            <Button type="submit" disabled={pending}>
              {pending ? "Saving" : "Set position"}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}

function Field({
  label,
  id,
  children,
}: {
  label: string
  id: string
  children: ReactNode
}) {
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  )
}
