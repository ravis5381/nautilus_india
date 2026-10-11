export function formatQuantity(quantity: number): string {
  return quantity > 0 ? `+${quantity}` : String(quantity)
}

export function formatPrice(value: number): string {
  return value
    .toFixed(4)
    .replace(/0+$/, "")
    .replace(/\.$/, "")
}

export function formatAmount(value: number): string {
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 4,
  }).format(value)
}
