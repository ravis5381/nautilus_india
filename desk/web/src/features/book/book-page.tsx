import { LoginPage } from "@/features/auth/login-page"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { ThemeToggle } from "@/components/theme-toggle"

import { PositionTable } from "./position-table"
import { StrategyField } from "./strategy-field"
import { SummaryCards } from "./summary-cards"
import { useBook } from "./use-book"

export function BookPage() {
  const book = useBook()

  if (book.status === "loading") {
    return <p className="px-4 py-8 text-sm text-muted-foreground">Checking the desk.</p>
  }
  if (book.status === "unauthorized") {
    return <LoginPage onSuccess={book.reload} />
  }

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-4 py-8">
      <header className="flex flex-col gap-3">
        <div className="flex items-start justify-between gap-4">
          <div className="grid gap-1">
            <p className="text-xs tracking-widest text-muted-foreground uppercase">
              India desk
            </p>
            <h1 className="text-4xl font-medium tracking-tight">Positions</h1>
          </div>
          <div className="flex shrink-0 items-center gap-1">
            <Button variant="link" asChild>
              <a href="/docs">API docs</a>
            </Button>
            <Button variant="outline" onClick={() => void book.lock()}>
              Sign out
            </Button>
            <ThemeToggle />
          </div>
        </div>
        <p className="max-w-xl text-sm text-muted-foreground">
          Each strategy keeps its own quantity for an instrument.
        </p>
      </header>

      <SummaryCards rows={book.rows} status={book.status} />

      <Card>
        <CardHeader>
          <div className="flex w-full flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
            <div className="grid gap-1">
              <CardTitle className="text-lg">Open book</CardTitle>
              <CardDescription>
                Each strategy keeps its own quantity for the same instrument.
              </CardDescription>
            </div>
            <StrategyField
              strategies={book.strategies}
              selected={book.selected}
              onChange={book.choose}
            />
          </div>
        </CardHeader>
        <CardContent>
          <PositionTable
            rows={book.rows}
            strategy={book.selected}
            status={book.status}
          />
        </CardContent>
      </Card>
    </div>
  )
}
