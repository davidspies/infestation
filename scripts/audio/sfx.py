"""Sound-effect recipes. Each recipe takes a numpy Generator and returns a mono float array; the
generator then trims, fades and normalises it to the loudness tier listed in SFX."""

import numpy as np

import instruments as ins
from dsp import (SR, attack_ramp, bandpass, crackle, eq, fm, frac_delay, harmonics, highpass, lowpass, midi_hz,
                 mix, modal, n_of, noise, phase_of, place, release_ramp, reverb_ir, reverb_mono, soft_clip, svf,
                 t_of)


def env(n, attack, decay):
    """Raised-cosine attack followed by an exponential decay (time constant `decay`)."""
    t = t_of(n)
    return attack_ramp(n, attack) * np.exp(-np.maximum(t - attack, 0) / decay)


def bump_env(n, center, width):
    """Smooth Gaussian bump peaking at `center` seconds."""
    return np.exp(-(((t_of(n) - center) / width) ** 2))


def thump(dur, f_hi, f_lo, sweep, decay, rng, noise_amt=0.4, noise_lp=400.0, attack=0.002):
    """Pitch-dropping sine body plus a puff of low-passed noise: the basis of every thud."""
    n = n_of(dur)
    t = t_of(n)
    f = f_lo + (f_hi - f_lo) * np.exp(-t / sweep)
    x = np.sin(2 * np.pi * phase_of(f)) * env(n, attack, decay)
    x += noise_amt * lowpass(noise(n, rng), noise_lp, 2) * env(n, 0.001, decay * 0.5)
    return x


def sweep(dur, f_start, f_end, q, rng):
    """Noise through a band-pass whose centre glides exponentially."""
    n = n_of(dur)
    fc = f_start * (f_end / f_start) ** (t_of(n) / dur)
    return svf(noise(n, rng), fc, q, "bp")


def metal(freqs, t60s, amps, dur, rng, attack=0.0005):
    """Inharmonic metallic ring from absolute partial frequencies."""
    return modal(1.0, list(zip(freqs, amps, t60s)), n_of(dur), rng, attack=attack)


def grit(dur, rng, density, lo, hi, decay=0.0008, density_env=None):
    """Granular crackle (grit, debris, sparks), band-limited."""
    return bandpass(crackle(n_of(dur), rng, density, decay, density_env), lo, hi, 2)


def wobble(n, rate, depth, rng):
    """Slightly irregular vibrato factor (rate in Hz, depth as a fraction)."""
    t = t_of(n)
    return 1 + depth * np.sin(2 * np.pi * rate * t + rng.uniform(0, 6.28)) * (1 + 0.3 * np.sin(2 * np.pi * 3.1 * t))


_ROOM = reverb_ir(0.7, np.random.default_rng(7), predelay=0.008, hf_t60_ratio=0.4, low_cut=200.0, channels=1)
_HALL = reverb_ir(1.6, np.random.default_rng(8), predelay=0.014, hf_t60_ratio=0.35, low_cut=200.0, channels=1)


def note(m):
    return float(midi_hz(m))


# ----------------------------------------------------------------------------------------------
# Hero


def step(rng, variant):
    pitch = (150.0, 136.0, 164.0)[variant]
    knock_f = (440.0, 405.0, 470.0)[variant]
    scuff_at = (0.026, 0.032, 0.022)[variant]
    dur = 0.09
    n = n_of(dur)
    body = thump(dur, pitch * 1.5, pitch, 0.007, 0.018, rng, noise_amt=0.6, noise_lp=900)
    knock = modal(1.0, [(knock_f, 1.0, 0.05), (knock_f * 2.3, 0.3, 0.025)], n, rng, attack=0.001)
    scuff = bandpass(noise(n, rng), 800, 3200, 2) * bump_env(n, scuff_at, 0.014) * 0.12
    gr = grit(dur, rng, 1400, 1800, 6000) * bump_env(n, scuff_at + 0.006, 0.016) * 0.9
    return body + 0.35 * knock + scuff + gr


def bump(rng):
    dur = 0.16
    body = thump(dur, 140, 72, 0.012, 0.04, rng, noise_amt=0.7, noise_lp=600)
    knock = modal(1.0, [(260.0, 1.0, 0.06), (610.0, 0.35, 0.03)], n_of(dur), rng, attack=0.002)
    cloth = bandpass(noise(n_of(dur), rng), 180, 900, 2) * env(n_of(dur), 0.003, 0.02) * 0.35
    clink = metal([2350, 3710, 5120, 6630], [0.09, 0.07, 0.05, 0.04], [1.0, 0.7, 0.45, 0.25], 0.12, rng)
    clink2 = metal([2480, 3930, 5410], [0.06, 0.05, 0.04], [1.0, 0.6, 0.4], 0.08, rng)
    return mix((body + 0.3 * knock + cloth, 0.0), (0.11 * clink, 0.018), (0.05 * clink2, 0.047))


def slash(rng):
    dur = 0.2
    n = n_of(dur)
    whoosh = sweep(dur, 4200, 700, 2.6, rng) * bump_env(n, 0.06, 0.04) * 1.6
    whoosh *= release_ramp(n, n_of(0.14), 0.06)
    shing_f = np.array([3150, 4730, 6290, 7980])
    shing = sum(a * np.sin(2 * np.pi * phase_of(f * (1 + 0.03 * (1 - np.exp(-t_of(n_of(0.17)) / 0.03)))))
                * np.exp(-t_of(n_of(0.17)) / d) for f, a, d in zip(shing_f, [1.0, 0.6, 0.4, 0.2], [0.05, 0.04, 0.03, 0.02]))
    shing = shing * attack_ramp(n_of(0.17), 0.002)
    sh = bandpass(noise(n_of(0.05), rng), 5000, 9500, 2) * env(n_of(0.05), 0.001, 0.012)
    return mix((whoosh, 0.0), (0.22 * shing, 0.022), (0.08 * sh, 0.02))


def hit(rng):
    dur = 0.13
    body = thump(dur, 210, 68, 0.014, 0.045, rng, noise_amt=0.6, noise_lp=900)
    crunch = grit(0.05, rng, 3500, 700, 3600) * env(n_of(0.05), 0.001, 0.014) * 3.5
    snap = lowpass(noise(n_of(0.01), rng), 2500, 2) * env(n_of(0.01), 0.0005, 0.003) * 0.5
    return soft_clip(mix((body, 0.0), (crunch, 0.002), (snap, 0.0)), 1.6)


def hero_death(rng):
    hit_part = thump(0.35, 140, 40, 0.025, 0.11, rng, noise_amt=0.7, noise_lp=650)
    crunch = grit(0.09, rng, 3000, 500, 3200) * env(n_of(0.09), 0.001, 0.025) * 4.0
    # descending sad tone: a soft horn-like voice sliding down a minor sixth, vibrato blooming
    dur = 0.62
    n = n_of(dur)
    t = t_of(n)
    f0 = 196 + (392 - 196) * np.exp(-t / 0.25)
    vib = 1 + 0.012 * np.clip((t - 0.15) / 0.3, 0, 1) * np.sin(2 * np.pi * 5.3 * t)
    bright = np.exp(-t / 0.35)
    sad = harmonics(f0 * vib, [1.0, 0.45 * (0.4 + 0.6 * bright), 0.22 * bright, 0.1 * bright, 0.05 * bright], n)
    sad *= attack_ramp(n, 0.04) * release_ramp(n, n_of(0.38), 0.24)
    x = mix((soft_clip(hit_part, 1.5), 0.0), (crunch, 0.003), (0.45 * sad, 0.11))
    return reverb_mono(x, _ROOM, 0.12)[: n_of(0.8)]


# ----------------------------------------------------------------------------------------------
# Rats


def scurry(rng, variant):
    clicks = (3, 4, 2)[variant]
    gap = (0.031, 0.024, 0.042)[variant]
    dur = 0.12
    parts = [(bandpass(noise(n_of(dur), rng), 1500, 6000, 2) * np.hanning(n_of(dur)) * 0.05, 0.0)]
    for i in range(clicks):
        f = rng.uniform(3000, 4200)
        c = metal([f, f * 1.62, f * 2.31], [0.012, 0.008, 0.006], [1.0, 0.5, 0.3], 0.03, rng, attack=0.0003)
        c[: n_of(0.002)] += highpass(noise(n_of(0.002), rng), 2000, 2) * np.hanning(n_of(0.002)) * 0.4
        parts.append((c * rng.uniform(0.55, 1.0), 0.004 + i * gap + rng.normal(0, 0.003)))
    return lowpass(mix(*parts), 8000, 2)


def squeak_voice(f0, dur, rng, rise=5.0, fall=-7.0, peak_at=0.3, double=False):
    """Cartoon rat squeak: nasal harmonic tone with an up-then-down pitch bend and a little flutter."""
    n = n_of(dur)
    u = t_of(n) / dur
    semis = np.where(u < peak_at, rise * np.sin(0.5 * np.pi * u / peak_at),
                     rise + (fall - rise) * (np.maximum(u - peak_at, 0) / (1 - peak_at)) ** 1.3)
    f = f0 * 2 ** (semis / 12) * wobble(n, 28.0, 0.012, rng)
    x = harmonics(f, [1.0, 0.45, 0.18, 0.06, 0.02], n)
    x = eq(x, ("peak", 3600, 1.8, 6.0), ("peak", 1200, 1.5, -4.0))
    x += bandpass(noise(n, rng), 3000, 7000, 2) * 0.06
    shape = attack_ramp(n, 0.008) * release_ramp(n, n - n_of(dur * 0.35), dur * 0.35)
    if double:
        shape *= 0.55 + 0.45 * np.cos(2 * np.pi * np.clip((u - 0.25) / 0.3, 0, 1)) ** 2
    return lowpass(x * shape, 7000, 4)


def squeak(rng, variant):
    f0, dur, rise, fall, double = ((1900.0, 0.19, 5.0, -7.0, False), (2250.0, 0.23, 4.0, -9.0, True),
                                   (1700.0, 0.16, 6.0, -5.0, False))[variant]
    return squeak_voice(f0, dur, rng, rise, fall, double=double)


# ----------------------------------------------------------------------------------------------
# Cyborg rats


def servo(rng, variant):
    lo, hi, end, gear = ((185.0, 345.0, 250.0, 57.0), (150.0, 290.0, 205.0, 46.0))[variant]
    dur = 0.15
    n = n_of(dur)
    u = t_of(n) / dur
    f = np.where(u < 0.45, lo + (hi - lo) * np.sin(0.5 * np.pi * u / 0.45),
                 hi + (end - hi) * (np.maximum(u - 0.45, 0) / 0.55) ** 0.8)
    whirr = harmonics(f, [(-1) ** (k + 1) / k for k in range(1, 40)], n)
    whirr *= 1 + 0.45 * np.sin(2 * np.pi * phase_of(gear * f / hi))  # gear-tooth chatter
    whirr = lowpass(bandpass(whirr, 500, 3200, 2), 5000, 2) * attack_ramp(n, 0.012) * release_ramp(n, n_of(0.11), 0.04)
    tk1 = ins.tick(1.0, rng, freq=3100 + 300 * variant)
    tk2 = ins.tick(0.7, rng, freq=2700 + 200 * variant)
    return mix((0.5 * whirr, 0.0), (tk1, 0.0), (tk2, 0.135 - 0.01 * variant))


def cyborg_death(rng):
    dur = 0.46
    n = n_of(dur)
    t = t_of(n)
    clank = ins.clank(1.0, rng, freq=395.0)
    fizz = grit(dur, rng, 1.0, 1500, 7000, density_env=4500 * np.exp(-t / 0.12) + 200)
    gate = (lowpass(rng.standard_normal(n), 35, 2) > -0.2).astype(float)
    gate = lowpass(gate, 300, 1)
    sputter = bandpass(noise(n, rng), 2000, 6500, 2) * gate * np.exp(-t / 0.12) * 0.25
    f = 45 + 105 * np.exp(-t / 0.15)
    buzz = harmonics(f, [(-1) ** (k + 1) / k for k in range(1, 60)], n)
    buzz = lowpass(buzz, 2400, 2) * gate * attack_ramp(n, 0.005) * release_ramp(n, n_of(0.25), 0.2)
    x = mix((0.9 * clank, 0.0), (1.2 * fizz, 0.0), (sputter, 0.01), (0.32 * buzz, 0.015))
    return lowpass(x, 7000, 2)


def chomp(rng):
    open_click = ins.tick(0.8, rng, freq=2900)
    clack = metal([1620, 2610, 3940, 5270], [0.09, 0.07, 0.05, 0.035], [1.0, 0.8, 0.55, 0.3], 0.16, rng)
    body = thump(0.12, 130, 60, 0.012, 0.035, rng, noise_amt=0.6, noise_lp=900)
    crunch = lowpass(grit(0.05, rng, 3000, 500, 3000), 2500, 2) * env(n_of(0.05), 0.001, 0.015) * 3.0
    muffled = lowpass(squeak_voice(1450, 0.13, rng, rise=3.0, fall=-6.0), 1300, 4) * 0.45
    return mix((0.35 * open_click, 0.0), (0.55 * clack, 0.045), (body, 0.045), (crunch, 0.047),
               (muffled, 0.085))


# ----------------------------------------------------------------------------------------------
# Environment


def explosion(rng):
    dur = 1.4
    n = n_of(dur)
    t = t_of(n)
    f = 35 + 45 * np.exp(-t / 0.13)
    sub = np.sin(2 * np.pi * phase_of(f)) * env(n, 0.003, 0.33)
    sub = soft_clip(1.6 * sub, 1.4)  # a little harmonic content so small speakers hear the boom
    rumble = lowpass(np.cumsum(noise(n, rng)) * 0.02, 180, 2)
    rumble = highpass(rumble, 30, 2) * env(n, 0.01, 0.55)
    burst = svf(noise(n, rng), 150 + 5800 * np.exp(-t / 0.16), 0.7, "lp") * env(n, 0.002, 0.3)
    body = bandpass(noise(n, rng), 120, 900, 2) * env(n, 0.004, 0.22)
    debris_density = 900 * np.clip((t - 0.12) / 0.05, 0, 1) * np.exp(-np.maximum(t - 0.12, 0) / 0.32) + 15
    debris = grit(dur, rng, 1.0, 900, 4800, decay=0.0012, density_env=debris_density)
    pebbles = [(ins.rim(rng.uniform(0.2, 0.5), rng, freq=rng.uniform(600, 1400)), rng.uniform(0.2, 1.05))
               for _ in range(7)]
    x = mix((0.8 * sub, 0.0), (0.7 * rumble, 0.0), (0.55 * burst, 0.0), (0.9 * body, 0.0), (0.9 * debris, 0.0),
            *[(0.25 * p, s) for p, s in pebbles])
    x = lowpass(soft_clip(x, 1.3), 9000, 2)
    return reverb_mono(x, _ROOM, 0.08)


def zap(rng):
    dur = 0.42
    n = n_of(dur)
    t = t_of(n)
    f = 104 * (1 + 0.04 * lowpass(rng.standard_normal(n), 40, 2) * 8)
    buzz = harmonics(f, [(-1) ** (k + 1) / k for k in range(1, 50)], n)
    spikes = 0.35 + np.abs(lowpass(crackle(n, rng, 700, 0.003), 400, 1)) * 6
    buzz = bandpass(buzz, 350, 5000, 2) * np.minimum(spikes, 1.6) * env(n, 0.003, 0.12)
    crack = grit(dur, rng, 1.0, 2000, 8000, density_env=3000 * np.exp(-t / 0.08) + 100)
    zing = np.zeros(n)
    for m, a, d in ((81, 1.0, 0.0), (88, 0.75, 0.025), (93, 0.4, 0.05)):  # A5, E6, A6: open, arcane
        k = n - n_of(d)
        tt = t_of(k)
        fz = note(m) * 2 ** (-4 * np.exp(-tt / 0.018) / 12) * (1 + 0.004 * np.sin(2 * np.pi * 7 * tt))
        z = fm(fz, 2.0, 0.8 * np.exp(-tt / 0.05), k)
        zing[n_of(d):] += a * z * env(k, 0.004, 0.13)
    x = mix((0.45 * buzz, 0.0), (0.9 * crack, 0.0), (0.3 * zing, 0.0))
    return reverb_mono(x, _ROOM, 0.15)[: n_of(0.45)]


def wall_rise(rng):
    dur = 0.33
    n = n_of(dur)
    t = t_of(n)
    grind = svf(np.cumsum(noise(n, rng)) * 0.05, 160 + 200 * t / dur, 0.9, "lp")
    grind = highpass(grind, 35, 2)
    grind *= 1 + 0.6 * lowpass(rng.standard_normal(n), 25, 2) * 4
    shape = attack_ramp(n, 0.04) * release_ramp(n, n_of(0.25), 0.07)
    scrape = bandpass(noise(n, rng), 900, 3000, 2) * (0.3 + np.abs(crackle(n, rng, 500, 0.004)) * 3)
    settle = thump(0.12, 100, 62, 0.01, 0.035, rng, noise_amt=0.7, noise_lp=380)
    return mix((0.9 * grind * shape, 0.0), (0.2 * scrape * shape, 0.0), (0.9 * settle, 0.29))


def plank_break(rng):
    crack = grit(0.03, rng, 1.0, 1500, 7000, decay=0.0005,
                 density_env=np.where(t_of(n_of(0.03)) < 0.025, 2500.0, 0.0)) * 2.0
    snap = highpass(noise(n_of(0.003), rng), 800, 2) * np.hanning(n_of(0.003))
    knock = modal(1.0, [(235, 1.0, 0.07), (528, 0.7, 0.05), (985, 0.45, 0.035), (1660, 0.3, 0.02)],
                  n_of(0.12), rng)
    dur = 0.36
    t = t_of(n_of(dur))
    debris = grit(dur, rng, 1.0, 1000, 4500, decay=0.001,
                  density_env=np.where(t > 0.04, 160 * np.exp(-(t - 0.04) / 0.1), 0.0))
    bits = [(ins.rim(rng.uniform(0.25, 0.5), rng, freq=rng.uniform(750, 1500)), rng.uniform(0.09, 0.3))
            for _ in range(4)]
    return mix((crack, 0.0), (0.7 * snap, 0.0), (0.8 * knock, 0.003), (1.2 * debris, 0.0),
               *[(0.35 * b, s) for b, s in bits])


def web_tear(rng):
    swish_n = n_of(0.09)
    swish = sweep(0.09, 7000, 3200, 1.4, rng) * bump_env(swish_n, 0.028, 0.02)
    dur = 0.13
    n = n_of(dur)
    t = t_of(n)
    f = 260 * (1400 / 260) ** np.clip(t / 0.07, 0, 1)
    thwip = harmonics(f, [1.0, 0.3, 0.12], n) * env(n, 0.004, 0.035)
    tearing = grit(0.14, rng, 1700, 2000, 6500) * bump_env(n_of(0.14), 0.07, 0.04) * 1.5
    return mix((0.6 * swish, 0.0), (0.35 * thwip, 0.018), (tearing, 0.02))


def swallow(rng):
    dur = 0.6
    n = n_of(dur)
    t = t_of(n)
    f = 55 + 700 * np.exp(-t / 0.15)
    rate = 6 + 10 * t / dur
    f = f * (1 + 0.035 * np.sin(2 * np.pi * np.cumsum(rate) / SR))
    voice = harmonics(f, [1.0, 0.45, 0.2, 0.08], n) * attack_ramp(n, 0.015) * release_ramp(n, n_of(0.45), 0.13)
    air = svf(noise(n, rng), np.maximum(2.2 * f, 120), 3.0, "bp") * attack_ramp(n, 0.02) * np.exp(-t / 0.3)
    x = 0.5 * voice + 0.35 * air
    delay = 30 + 110 * (0.5 + 0.5 * np.sin(2 * np.pi * 2.6 * t))  # flanger: 0.7-3.2 ms sweep
    x = x + frac_delay(x, delay)
    thud = thump(0.12, 80, 40, 0.015, 0.04, rng, noise_amt=0.3, noise_lp=300)
    return mix((x, 0.0), (0.7 * thud, 0.55))


def trigger(rng):
    click = modal(1.0, [(1250, 1.0, 0.025), (2610, 0.6, 0.015), (4100, 0.3, 0.01)], n_of(0.05), rng)
    low = thump(0.06, 160, 110, 0.01, 0.018, rng, noise_amt=0.2)
    chimes = [(ins.glock(note(m), 0.2, 0.8, rng, t60=0.5), d) for m, d in ((81, 0.03), (88, 0.085), (95, 0.14))]
    x = mix((0.5 * click, 0.0), (0.5 * low, 0.0), *[(0.35 * c, d) for c, d in chimes])
    return reverb_mono(x, _ROOM, 0.18)[: n_of(0.4)]


def tape_rewind(dur, f_start, f_end, flutter, rng):
    """Rising chirp with tape flutter plus a reversed (swelling) whoosh, ending abruptly-but-smoothly."""
    n = n_of(dur)
    t = t_of(n)
    u = t / dur
    f = f_start * (f_end / f_start) ** (u**1.5)
    rate = flutter[0] + (flutter[1] - flutter[0]) * u
    f = f * (1 + 0.02 * np.sin(2 * np.pi * np.cumsum(rate) / SR))
    chirp = harmonics(f, [1.0, 0.35, 0.15, 0.06], n)
    swell = (u**2) * release_ramp(n, n - n_of(0.03), 0.03) * attack_ramp(n, 0.01)
    whoosh = sweep(dur, f_start * 1.5, f_end * 2.5, 1.2, rng)
    return (0.4 * chirp + 0.6 * whoosh) * (0.15 + 0.85 * swell)


def undo(rng):
    return tape_rewind(0.2, 380, 1500, (14, 22), rng)


def restart(rng):
    return tape_rewind(0.45, 130, 1700, (6, 30), rng)


# ----------------------------------------------------------------------------------------------
# UI and map


def ui_move(rng):
    x = modal(1.0, [(1180, 1.0, 0.03), (2790, 0.35, 0.014), (4300, 0.1, 0.008)], n_of(0.045), rng, attack=0.0008)
    return x + lowpass(noise(n_of(0.045), rng), 1500, 2) * env(n_of(0.045), 0.0005, 0.003) * 0.1


def two_notes(m1, m2, gap, vel, rng, t60=0.3):
    a = ins.marimba(note(m1), 0.05, vel, rng)
    b = ins.marimba(note(m2), 0.08, vel, rng)
    n = n_of(t60 + 0.1)
    return mix((a[:n] * release_ramp(min(n, len(a)), n_of(t60 * 0.6), t60 * 0.4), 0.0),
               (b[:n] * release_ramp(min(n, len(b)), n_of(t60 * 0.6), t60 * 0.4), gap))


def ui_confirm(rng):
    x = two_notes(79, 86, 0.06, 0.9, rng)  # G5 -> D6
    sparkle = ins.glock(note(98), 0.1, 0.3, rng, t60=0.3)  # D7 glint
    return reverb_mono(mix((x, 0.0), (0.12 * sparkle, 0.06)), _ROOM, 0.08)[: n_of(0.25)]


def ui_back(rng):
    return lowpass(two_notes(84, 79, 0.055, 0.6, rng, t60=0.25), 3000, 2)[: n_of(0.25)]  # C6 -> G5, darker


def ui_locked(rng):
    thunk = modal(1.0, [(220, 1.0, 0.05), (530, 0.4, 0.03), (1210, 0.15, 0.015)], n_of(0.08), rng)
    parts = [(0.6 * thunk, 0.0)]
    for start, f, d in ((0.0, 155.0, 0.07), (0.095, 116.0, 0.11)):
        k = n_of(d)
        buzz = harmonics(f * (1 - 0.04 * t_of(k) / d), [1 / j if j % 2 else 0.1 / j for j in range(1, 16)], k)
        parts.append((0.35 * lowpass(buzz, 1100, 2) * attack_ramp(k, 0.006) * release_ramp(k, k - n_of(0.03), 0.03),
                      start))
    return mix(*parts)


def map_step(rng):
    dur = 0.065
    n = n_of(dur)
    body = thump(dur, 240, 150, 0.006, 0.013, rng, noise_amt=0.5, noise_lp=900)
    crunch = grit(dur, rng, 2600, 1200, 5000) * bump_env(n, 0.016, 0.013) * 1.2
    brush = bandpass(noise(n, rng), 1500, 5000, 2) * bump_env(n, 0.02, 0.012) * 0.08
    return body + crunch + brush


def text_blip(rng, variant):
    f = (560.0, 600.0, 640.0)[variant]
    n = n_of(0.035)
    x = harmonics(np.full(n, f), [1.0, 0.18, 0.06], n)
    return x * attack_ramp(n, 0.003) * np.exp(-t_of(n) / 0.011)


def level_start(rng):
    whoosh_n = n_of(0.3)
    u = t_of(whoosh_n) / 0.3
    whoosh = sweep(0.3, 380, 2600, 1.3, rng) * u**2 * release_ramp(whoosh_n, n_of(0.25), 0.05)
    chord = [(ins.harp(note(m), 0.4, 0.75, rng, ring=0.5), 0.24 + 0.018 * i) for i, m in enumerate((50, 57, 62, 66))]
    bells = [(ins.glock(note(m), 0.2, 0.55, rng, t60=0.9), 0.26 + 0.03 * i) for i, m in enumerate((78, 81))]
    x = mix((0.45 * whoosh, 0.0), *[(0.35 * c, s) for c, s in chord], *[(0.22 * b, s) for b, s in bells])
    return reverb_mono(x, _HALL, 0.22)[: n_of(0.8)]


def pause(rng):
    body = thump(0.22, 270, 115, 0.035, 0.055, rng, noise_amt=0.45, noise_lp=700, attack=0.012)
    return lowpass(body, 1200, 2)


# ----------------------------------------------------------------------------------------------
# Jingles (D major, the world-map key)


def win(rng):
    parts = []
    run = ((74, 0.0), (78, 0.11), (81, 0.22))  # D5 F#5 A5
    for m, s in run:
        parts.append((0.5 * ins.harp(note(m), 0.1, 0.85, rng, ring=0.3), s))
        parts.append((0.25 * ins.lute(note(m), 0.1, 0.8, rng), s))
    parts.append((0.5 * ins.harp(note(86), 0.25, 0.95, rng, ring=0.4), 0.33))  # D6, accented
    parts.append((0.35 * ins.glock(note(86), 0.2, 0.9, rng, t60=1.0), 0.33))
    for i, (chord, bell, s) in enumerate((((67, 71, 74), 83, 0.62), ((69, 73, 76), 85, 0.86))):  # G, A
        for k, m in enumerate(chord):
            parts.append((0.33 * ins.harp(note(m), 0.2, 0.75, rng, ring=0.3), s + 0.012 * k))
        parts.append((0.3 * ins.glock(note(bell), 0.2, 0.75, rng, t60=0.8), s))
        parts.append((0.4 * ins.bass(note(43 + 2 * i), 0.2, 0.8, rng), s))
    for k, m in enumerate((62, 66, 69, 74)):  # final D major, strummed
        parts.append((0.33 * ins.harp(note(m), 1.0, 0.85, rng, ring=1.2), 1.1 + 0.02 * k))
    for k, m in enumerate((86, 90, 93)):  # bells D6 F#6 A6
        parts.append((0.28 * ins.glock(note(m), 0.5, 0.8, rng, t60=1.5), 1.12 + 0.06 * k))
    parts.append((0.55 * ins.bass(note(38), 0.9, 0.9, rng), 1.1))
    x = mix(*parts)
    return reverb_mono(x, _HALL, 0.25)[: n_of(2.6)]


def lose(rng):
    parts = []
    notes = ((67, 0.0, 0.26), (66, 0.3, 0.26), (65, 0.6, 0.26), (64, 0.9, 0.75))  # G4 F#4 F4 ... E4
    for i, (m, s, d) in enumerate(notes):
        n = n_of(d + 0.12)
        t = t_of(n)
        f = np.full(n, note(m))
        wah = np.exp(-((t - 0.07) / 0.09) ** 2)  # each note opens and closes like a muted horn
        if i == len(notes) - 1:  # the last one sags, wobbles and stays a little open
            f *= 1 - 0.03 * np.clip((t - 0.45) / 0.4, 0, 1)
            f *= 1 + 0.012 * np.clip((t - 0.2) / 0.2, 0, 1) * np.sin(2 * np.pi * 5.5 * t)
            wah = 0.3 + 0.7 * np.exp(-((t - 0.1) / 0.14) ** 2)
        amps = [1.0, 0.6 * (0.4 + 0.6 * wah), 0.4 * wah, 0.25 * wah, 0.12 * wah, 0.06 * wah]
        voice = harmonics(f, amps, n) * attack_ramp(n, 0.03) * release_ramp(n, n_of(d), 0.12)
        parts.append((0.4 * voice, s))
        parts.append((0.25 * ins.pizz(note(m - 12), 0.2, 0.7, rng), s))
    parts.append((0.3 * ins.harp(note(52), 0.6, 0.6, rng, ring=0.6), 0.9))  # E3 under the last note
    x = mix(*parts)
    return reverb_mono(x, _HALL, 0.2)[: n_of(1.9)]


def twinkles(dur, rng, density_env, f_lo, f_hi):
    """Random tiny bell grains (sparkle) whose density follows density_env (grains/second)."""
    n = n_of(dur)
    out = np.zeros(n)
    hits = np.nonzero(rng.random(n) < density_env / SR)[0]
    for h in hits:
        f = f_lo * (f_hi / f_lo) ** rng.random()
        g = modal(f, [(1.0, 1.0, 0.12), (2.76, 0.2, 0.04)], n_of(0.14), rng, attack=0.002)
        place(out, g * rng.uniform(0.3, 1.0), h)
    return out


def unlock(rng):
    run = (81, 86, 88, 90, 93, 98)  # A5 D6 E6 F#6 A6 D7: ascending D major pentatonic
    parts = [(0.3 * ins.glock(note(m), 0.1, 0.8, rng, t60=1.0 if i < len(run) - 1 else 1.4), 0.065 * i)
             for i, m in enumerate(run)]
    t = t_of(n_of(1.0))
    parts.append((0.05 * twinkles(1.0, rng, 40 * np.exp(-t / 0.35), 3500, 8000), 0.08))
    parts.append((0.3 * ins.harp(note(62), 0.5, 0.6, rng, ring=0.8), 0.0))
    x = mix(*parts)
    return reverb_mono(x, _HALL, 0.3)[: n_of(1.3)]


def reveal(rng):
    swell_dur = 1.05
    n = n_of(swell_dur)
    u = t_of(n) / swell_dur
    padx = ins.pad([note(m) for m in (50, 57, 64, 66)], swell_dur, 0.8, rng, cutoff=1800, attack=0.9,
                   release=0.9).mean(axis=1)
    shimmer = sweep(swell_dur, 800, 6000, 2.0, rng) * u**2.5 * release_ramp(n, n - n_of(0.08), 0.08)
    sparkle = twinkles(swell_dur, rng, 5 + 70 * u**2, 1500, 6500)
    chord = [(0.3 * ins.glock(note(m), 0.6, 0.85, rng, t60=1.6), swell_dur + 0.015 * k)
             for k, m in enumerate((74, 81, 86, 90))]
    low = (0.35 * ins.harp(note(50), 1.0, 0.8, rng, ring=1.0), swell_dur)
    x = mix((0.5 * padx, 0.0), (0.12 * shimmer, 0.0), (0.05 * sparkle, 0.0), *chord, low)
    return reverb_mono(x, _HALL, 0.3)[: n_of(2.3)]


# ----------------------------------------------------------------------------------------------
# name -> (recipe, target loudness). Targets are K-weighted loudness over the active part, on the
# LUFS scale as heard on both speakers (dsp.active_loudness), so they compare directly with the
# music's -18 LUFS: frequent sounds sit at or under the music, a kill (hit + squeak) around -14,
# big moments (explosion, hero death, win) at -12.

SFX = {
    "step_1": (lambda r: step(r, 0), -20.5),
    "step_2": (lambda r: step(r, 1), -20.5),
    "step_3": (lambda r: step(r, 2), -20.5),
    "bump": (bump, -19.0),
    "scurry_1": (lambda r: scurry(r, 0), -22.0),
    "scurry_2": (lambda r: scurry(r, 1), -22.0),
    "scurry_3": (lambda r: scurry(r, 2), -22.0),
    "squeak_1": (lambda r: squeak(r, 0), -18.0),
    "squeak_2": (lambda r: squeak(r, 1), -18.0),
    "squeak_3": (lambda r: squeak(r, 2), -18.0),
    "slash": (slash, -17.5),
    "hit": (hit, -16.0),
    "hero_death": (hero_death, -12.0),
    "servo_1": (lambda r: servo(r, 0), -21.5),
    "servo_2": (lambda r: servo(r, 1), -21.5),
    "cyborg_death": (cyborg_death, -14.5),
    "chomp": (chomp, -15.0),
    "explosion": (explosion, -11.5),
    "zap": (zap, -15.5),
    "wall_rise": (wall_rise, -15.5),
    "plank_break": (plank_break, -15.0),
    "web_tear": (web_tear, -16.5),
    "swallow": (swallow, -14.5),
    "trigger": (trigger, -17.0),
    "undo": (undo, -19.0),
    "restart": (restart, -17.0),
    "ui_move": (ui_move, -21.5),
    "ui_confirm": (ui_confirm, -17.5),
    "ui_back": (ui_back, -19.0),
    "ui_locked": (ui_locked, -17.5),
    "map_step": (map_step, -22.5),
    "text_blip_1": (lambda r: text_blip(r, 0), -23.0),
    "text_blip_2": (lambda r: text_blip(r, 1), -23.0),
    "text_blip_3": (lambda r: text_blip(r, 2), -23.0),
    "level_start": (level_start, -15.5),
    "pause": (pause, -18.5),
    "win": (win, -12.0),
    "lose": (lose, -14.5),
    "unlock": (unlock, -15.0),
    "reveal": (reveal, -14.5),
}
