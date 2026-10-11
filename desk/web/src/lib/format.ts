export function formatQuantity(quantity: number): string {
  return quantity > 0 ? `+${quantity}` : String(quantity)
}

export function formatAmount(value: number): string {
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 4,
  }).format(value)
}
