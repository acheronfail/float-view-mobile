const green = [22, 163, 74];
const yellow = [234, 179, 8];
const orange = [234, 88, 12];
const red = [220, 38, 38];

function interpolate(start: number[], end: number[], amount: number) {
  return start.map((value, index) => Math.round(value + (end[index] - value) * amount));
}

function rgb(color: number[]) {
  return `rgb(${color.join(', ')})`;
}

export function colorForBattery(percent: number | null) {
  if (percent == null || percent >= 50) return rgb(green);
  if (percent <= 20) return rgb(red);
  if (percent < 35) return rgb(interpolate(yellow, red, (35 - percent) / 15));
  return rgb(interpolate(green, yellow, (50 - percent) / 15));
}

export function colorForTemperature(value: number | null | undefined, redAt = 60) {
  if (value == null || value <= 30) return rgb(green);
  if (value >= redAt) return rgb(red);
  const orangeAt = (30 + redAt) / 2;
  if (value >= orangeAt) return rgb(interpolate(orange, red, (value - orangeAt) / (redAt - orangeAt)));
  return rgb(interpolate(green, orange, (value - 30) / (orangeAt - 30)));
}
