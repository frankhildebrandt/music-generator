export function element<T extends HTMLElement = HTMLElement>(selector: string): T {
  const node = document.querySelector<T>(selector);
  if (!node) throw new Error(`Missing element: ${selector}`);
  return node;
}

export function bytes(value: number): string {
  return new Intl.NumberFormat(document.documentElement.lang, { maximumFractionDigits: 1 }).format(value / 1024 ** 3) + " GiB";
}
