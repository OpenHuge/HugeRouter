export function formatCurrency(value: number) {
  return new Intl.NumberFormat('en-US', {
    currency: 'USD',
    maximumFractionDigits: 0,
    style: 'currency'
  }).format(value)
}

export function formatPercent(value: number) {
  return new Intl.NumberFormat('en-US', {
    maximumFractionDigits: 1,
    style: 'percent'
  }).format(value)
}
