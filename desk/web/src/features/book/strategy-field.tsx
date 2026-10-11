import { ALL_STRATEGIES } from "@/features/book/use-book"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

type StrategyFieldProps = {
  strategies: string[]
  selected: string
  onChange: (strategy: string) => void
}

export function StrategyField({
  strategies,
  selected,
  onChange,
}: StrategyFieldProps) {
  const empty = strategies.length === 0

  return (
    <div className="grid gap-2">
      <Label htmlFor="book-strategy">Strategy</Label>
      <Select
        key={empty ? "empty" : "book"}
        value={empty ? undefined : selected}
        onValueChange={onChange}
        disabled={empty}
      >
        <SelectTrigger id="book-strategy" className="w-full sm:w-56">
          <SelectValue placeholder="No open strategies" />
        </SelectTrigger>
        <SelectContent>
          {strategies.map((strategy) => (
            <SelectItem key={strategy} value={strategy}>
              {strategy === ALL_STRATEGIES ? "All" : strategy}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
