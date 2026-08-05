const sessionDateFormatter = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'long',
  day: 'numeric',
});

export function sessionStartDateLabel(startedAt: string | null | undefined): string | null {
  if (!startedAt) return null;
  const date = new Date(startedAt);
  return Number.isNaN(date.getTime()) ? null : sessionDateFormatter.format(date);
}
