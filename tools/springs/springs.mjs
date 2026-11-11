// Spring parameters shared by web, SwiftUI and Compose (DESIGN.md §6.2).
// response/damping (Apple style) -> stiffness k = (2π / response)², mass 1, ratio ζ = damping.

const TAU = Math.PI * 2;

/** @param {{response:number, damping:number}} s */
export function toPhysics({ response, damping }) {
  const omega = TAU / response;
  return { stiffness: omega * omega, dampingRatio: damping, mass: 1, omega };
}

/** Position (0 → 1) of a unit step spring at time t seconds, starting at rest. */
export function position(s, t) {
  const { omega, dampingRatio: z } = toPhysics(s);
  if (z < 1) {
    const wd = omega * Math.sqrt(1 - z * z);
    return 1 - Math.exp(-z * omega * t) * (Math.cos(wd * t) + ((z * omega) / wd) * Math.sin(wd * t));
  }
  if (z === 1) return 1 - Math.exp(-omega * t) * (1 + omega * t);
  const r1 = -omega * (z - Math.sqrt(z * z - 1));
  const r2 = -omega * (z + Math.sqrt(z * z - 1));
  const c2 = r1 / (r1 - r2);
  const c1 = 1 - c2;
  return 1 - (c1 * Math.exp(r1 * t) + c2 * Math.exp(r2 * t));
}

/** Time (ms) after which the spring stays within `epsilon` of rest. */
export function settleTime(s, epsilon = 0.001) {
  const dt = 1 / 1000;
  let lastOutside = 0;
  for (let t = 0; t < 5; t += dt) {
    if (Math.abs(1 - position(s, t)) > epsilon) lastOutside = t;
  }
  return Math.ceil((lastOutside + dt) * 1000);
}

/**
 * CSS `linear()` easing approximating the spring, plus the duration to pair it with.
 * @returns {{ easing: string, duration: number }}
 */
export function toCssLinear(s, samples = 40) {
  const duration = settleTime(s);
  const pts = [];
  for (let i = 0; i <= samples; i++) {
    const t = (i / samples) * (duration / 1000);
    const v = i === samples ? 1 : position(s, t);
    pts.push(Number(v.toFixed(4)));
  }
  return { easing: `linear(${pts.join(', ')})`, duration };
}
