// Sound: tiny WebAudio synth — no assets, satisfying feedback.
let ctx: AudioContext | null = null;

function ac(): AudioContext | null {
  try {
    if (!ctx) ctx = new (window.AudioContext || (window as any).webkitAudioContext)();
    return ctx;
  } catch {
    return null;
  }
}

function blip(freq: number, dur: number, type: OscillatorType, gain: number, when = 0) {
  const c = ac();
  if (!c) return;
  const t = c.currentTime + when;
  const o = c.createOscillator();
  const g = c.createGain();
  o.type = type;
  o.frequency.setValueAtTime(freq, t);
  o.frequency.exponentialRampToValueAtTime(Math.max(40, freq * 0.6), t + dur);
  g.gain.setValueAtTime(gain, t);
  g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  o.connect(g).connect(c.destination);
  o.start(t);
  o.stop(t + dur + 0.02);
}

export const sfx = {
  select() {
    blip(520, 0.06, "sine", 0.12);
  },
  match() {
    blip(660, 0.09, "triangle", 0.16);
    blip(990, 0.12, "triangle", 0.12, 0.05);
  },
  win() {
    [523, 659, 784, 1047].forEach((f, i) => blip(f, 0.16, "triangle", 0.16, i * 0.11));
  },
  fail() {
    blip(220, 0.25, "sawtooth", 0.1);
  },
  shuffle() {
    for (let i = 0; i < 6; i++) blip(300 + Math.random() * 500, 0.05, "square", 0.06, i * 0.04);
  },
  power() {
    blip(440, 0.08, "square", 0.12);
    blip(880, 0.1, "square", 0.08, 0.06);
  },
  deny() {
    blip(180, 0.08, "square", 0.08);
  },
};
