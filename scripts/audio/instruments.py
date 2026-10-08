"""Synthesised instruments shared by the music and the SFX.

Every instrument is a function (freq_hz, dur_s, vel, rng) -> mono float array. `dur_s` is the held
length of the note; plucked/struck instruments ring past it (damped by a release) and the returned
array includes that ring-out. Velocity 1.0 gives a peak of roughly 0.5-1.0.
"""

import numpy as np
from scipy import signal

from dsp import (SR, attack_ramp, bandpass, cents, dc_block, eq, fm, harmonics, lowpass, modal, n_of, phase_of,
                 place, release_ramp, saw, saw_amps, square_amps, t60_env, t_of)


# ----------------------------------------------------------------------------------------------
# Karplus-Strong plucked strings


def ks_string(freq, n, rng, t60=2.0, damping=0.5, pick=0.15, excite_lp=4000.0, excite_noise=0.5, warmth=0.0):
    """Extended Karplus-Strong string, vectorised one loop period at a time.

    Loop: integer delay N + damping filter (one-zero (1-S) + S z^-1, S = `damping` <= 0.5, optionally
    followed by a one-pole low-pass with pole `warmth`) + first-order allpass tuned so the total
    phase delay at `freq` is exactly SR/freq. Loop gain is set so the fundamental decays by 60 dB in
    `t60` seconds; higher harmonics decay faster. The excitation is a one-period burst: a smoothed
    pluck shape mixed with low-passed noise, comb-filtered by the pick position."""
    period = SR / freq
    w0 = 2 * np.pi * freq / SR
    z = np.exp(-1j * w0)
    budget = 10 ** (-3 * 0.7 / (freq * t60))  # the damping filter may use 70% of the decay budget
    s, p = damping, warmth
    while True:
        b_lp = np.convolve([1 - s, s], [1 - p])
        a_lp = np.array([1.0, -p])
        h = (b_lp[0] + b_lp[1] * z) / (1 + a_lp[1] * z)
        if abs(h) >= budget:
            break
        s, p = s * 0.85, p * 0.85  # high notes: relax the damping
    d_lp = -np.angle(h) / w0
    delay = int(np.floor(period - d_lp - 0.5))
    d_ap = period - d_lp - delay
    c = np.sin(w0 * (1 - d_ap) / 2) / np.sin(w0 * (1 + d_ap) / 2)
    g = min(10 ** (-3 / (freq * t60)) / abs(h), 0.99995)
    b = np.convolve(b_lp, [c, 1.0]) * g
    a = np.convolve(a_lp, [1.0, c])

    # excitation, one period long
    shape = np.sin(np.pi * np.arange(delay) / delay) ** 2
    burst = lowpass(rng.standard_normal(delay + 64), excite_lp, 2)[64:]
    burst /= np.max(np.abs(burst)) + 1e-12
    exc = (1 - excite_noise) * shape + excite_noise * burst
    exc -= np.mean(exc)
    m = max(int(round(pick * delay)), 1)
    exc = exc - np.concatenate([np.zeros(m), exc[:-m]])

    blocks = -(-n // delay) + 1
    v = np.zeros(blocks * delay)
    v[:delay] = exc
    zi = np.zeros(max(len(a), len(b)) - 1)
    for k in range(1, blocks):
        y, zi = signal.lfilter(b, a, v[(k - 1) * delay : k * delay], zi=zi)
        v[k * delay : (k + 1) * delay] += y
    out = dc_block(v[:n], 25) * attack_ramp(n, 0.001)
    return out / (np.max(np.abs(out)) + 1e-12)


def _ring(dur, ring):
    """Total length and damping envelope for a plucked note held `dur` seconds, then damped."""
    n = n_of(dur + ring)
    return n, release_ramp(n, n_of(dur), ring)


def lute(freq, dur, vel, rng, ring=0.35):
    """Double-course lute: two slightly detuned strings plucked near the bridge, nasal body EQ.
    `ring` is how long the string sounds on after the note's held length before it is damped."""
    n, damp = _ring(dur, ring)
    bright = 1500 + 2500 * vel
    t60 = float(np.clip(2.6 * (196 / freq) ** 0.5, 0.9, 3.0))
    a = ks_string(freq * cents(-1.5), n, rng, t60=t60, damping=0.42, pick=0.11, excite_lp=bright, warmth=0.15)
    b = ks_string(freq * cents(1.8), n, rng, t60=t60 * 0.9, damping=0.46, pick=0.13, excite_lp=bright,
                  warmth=0.2)
    x = 0.55 * a + 0.45 * b
    pluck = bandpass(rng.standard_normal(n_of(0.004)), 1500, 5000) * 0.05
    x[: len(pluck)] += pluck
    x = eq(x, ("peak", 210, 1.1, 3.0), ("peak", 520, 1.4, 1.5), ("peak", 2300, 1.6, 2.5), ("peak", 4200, 1.0, -4))
    x = lowpass(x, 6500, 2)
    return 0.8 * vel * x * damp / (np.max(np.abs(x)) + 1e-12)


def harp(freq, dur, vel, rng, ring=1.6):
    """Concert-harp-like string: plucked mid-string with the finger, round and long."""
    n, damp = _ring(dur, ring)
    t60 = float(np.clip(3.2 * (220 / freq) ** 0.45, 1.0, 4.5))
    x = ks_string(freq, n, rng, t60=t60, damping=0.5, pick=0.27, excite_lp=1200 + 2000 * vel,
                  excite_noise=0.25, warmth=0.3)
    x = eq(x, ("peak", 250, 0.9, 2.0), ("highshelf", 3500, 0.7, -3))
    return 0.8 * vel * x * damp / (np.max(np.abs(x)) + 1e-12)


def pizz(freq, dur, vel, rng):
    """Pizzicato string: quickly damped string through violin-family body resonances."""
    n = n_of(0.5 + 0.15 * (220 / freq))
    t60 = float(np.clip(0.55 * (220 / freq) ** 0.6, 0.18, 0.9))
    x = ks_string(freq, n, rng, t60=t60, damping=0.5, pick=0.18, excite_lp=1200 + 2200 * vel,
                  excite_noise=0.3)
    thump = lowpass(rng.standard_normal(n_of(0.012)), 600, 2) * t60_env(n_of(0.012), 0.01) * 0.25
    x[: len(thump)] += thump
    x = eq(x, ("peak", 290, 1.5, 4.0), ("peak", 460, 2.0, 2.0), ("peak", 1100, 1.5, 1.5),
           ("peak", 2600, 1.2, 2.0), ("highshelf", 4000, 0.7, -6))
    x *= release_ramp(n, n - n_of(0.04), 0.04)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


# ----------------------------------------------------------------------------------------------
# Struck / modal instruments


def _mallet(n_click, rng, lo, hi):
    return bandpass(rng.standard_normal(n_click), lo, hi) * t60_env(n_click, n_click / SR)


def marimba(freq, dur, vel, rng):
    """Tuned bar (partials ~1:4:10) with resonator; softer mallets for lower velocities."""
    t60 = float(np.clip(1.5 * (262 / freq) ** 0.7, 0.35, 2.2))
    n = n_of(min(t60, dur + 0.6) + 0.1)
    hard = 0.35 + 0.65 * vel
    x = modal(freq, [(1.0, 1.0, t60), (3.93, 0.22 * hard, t60 * 0.22), (9.3, 0.05 * hard, t60 * 0.07),
                     (2.0, 0.03, t60 * 0.5)], n, rng, attack=0.0015)
    click = _mallet(n_of(0.003), rng, 800, 3500) * 0.06 * hard
    x[: len(click)] += click
    x *= release_ramp(n, n - n_of(0.08), 0.08)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


def kalimba(freq, dur, vel, rng):
    """Thumb piano tine: strong fundamental, a bright 'ping' partial, slight pitch settle, box warmth."""
    t60 = float(np.clip(2.4 * (330 / freq) ** 0.5, 0.8, 3.2))
    n = n_of(min(t60, dur + 1.2) + 0.1)
    t = t_of(n)
    bend = 1 + 0.004 * np.exp(-t / 0.025)
    ph = 2 * np.pi * phase_of(freq * bend)
    x = np.sin(ph) * 10 ** (-3 * t / t60)
    x += 0.22 * vel * np.sin(5.95 * ph + 0.3) * 10 ** (-3 * t / (0.18 * t60))
    if freq * 14.2 < 18000:
        x += 0.05 * vel * np.sin(14.2 * ph + 0.7) * 10 ** (-3 * t / 0.06)
    x *= attack_ramp(n, 0.001)
    click = _mallet(n_of(0.004), rng, 1500, 5000) * 0.08 * vel
    x[: len(click)] += click
    x = eq(x, ("peak", 260, 1.0, 2.5))
    x *= release_ramp(n, n - n_of(0.1), 0.1)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


def music_box(freq, dur, vel, rng, detune=6.0):
    """Music-box comb tooth (clamped bar, partials 1 : 6.27 : 17.55), two slightly mistuned teeth."""
    t60 = float(np.clip(2.2 * (1047 / freq) ** 0.5, 0.9, 3.0))
    n = n_of(t60 + 0.05)
    x = np.zeros(n)
    worn = rng.normal(0, 2.0)  # each tooth pair is a little out of tune
    for c, amp in ((0.0, 0.7), (detune, 0.3)):
        f = freq * cents(worn + c + rng.normal(0, 1.0))
        x += amp * modal(f, [(1.0, 1.0, t60), (6.27, 0.18, t60 * 0.15), (17.55, 0.04, 0.05)], n, rng,
                         attack=0.0006)
    click = _mallet(n_of(0.0015), rng, 3000, 8000) * 0.05
    x[: len(click)] += click
    x = eq(x, ("highshelf", 6000, 0.7, -5))
    x *= release_ramp(n, n - n_of(0.1), 0.1)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


def glock(freq, dur, vel, rng, t60=None):
    """Soft glockenspiel/celesta bell (free bar partials 1 : 2.76 : 5.40 : 8.93), gentle mallet."""
    t60 = t60 or float(np.clip(2.4 * (880 / freq) ** 0.4, 1.0, 3.2))
    n = n_of(t60 + 0.05)
    x = modal(freq, [(1.0, 1.0, t60), (2.756, 0.28 * vel, t60 * 0.28), (5.404, 0.08 * vel, t60 * 0.09),
                     (8.933, 0.03 * vel, 0.06), (2.0, 0.04, t60 * 0.6)], n, rng, attack=0.0012)
    x = eq(x, ("highshelf", 7000, 0.7, -6))
    x *= release_ramp(n, n - n_of(0.1), 0.1)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


def fm_bell(freq, dur, vel, rng, ratio=3.5, index=2.2, t60=2.5):
    """Two-operator FM chime with a decaying modulation index (bright attack, purer tail). An
    inharmonic ratio such as 1.41 gives struck glass."""
    n = n_of(t60 + 0.05)
    t = t_of(n)
    x = fm(freq, ratio, index * vel * np.exp(-t / 0.18) + 0.25, n) * 10 ** (-3 * t / t60)
    x *= attack_ramp(n, 0.0015) * release_ramp(n, n - n_of(0.1), 0.1)
    x = lowpass(x, 9000, 2)
    return 0.8 * vel * x / (np.max(np.abs(x)) + 1e-12)


# ----------------------------------------------------------------------------------------------
# Subtractive / additive voices


def synth_pluck(freq, dur, vel, rng, wave="saw", cutoff=(3200.0, 500.0), decay=0.22, q=1.2):
    """Band-limited saw/square through a resonant low-pass whose cutoff glides from cutoff[0] to
    cutoff[1] (computed additively, so it is alias-free and needs no per-sample filter loop)."""
    n = n_of(dur + decay * 1.5)
    t = t_of(n)
    fc = cutoff[1] + (cutoff[0] * (0.5 + 0.5 * vel) - cutoff[1]) * np.exp(-t / (decay * 0.5))
    k_max = int(min(12000, cutoff[0] * 2.5) / freq) + 1
    base = saw_amps(k_max) if wave == "saw" else square_amps(k_max)
    amps = []
    for k, a in enumerate(base, start=1):
        r = k * freq / fc
        amps.append(a / np.sqrt((1 - r * r) ** 2 + (r / q) ** 2) if a else 0.0)
    x = harmonics(freq * cents(rng.normal(0, 1.5)), amps, n, phase0=rng.random())
    env = attack_ramp(n, 0.002) * np.exp(-t / decay) * release_ramp(n, n_of(dur), decay)
    return 0.8 * vel * x * env / (np.max(np.abs(x * env)) + 1e-12)


def pad(freqs, dur, vel, rng, cutoff=1400.0, attack=0.9, release=1.4, detune=9.0, voices=3,
        vibrato=0.0, width=0.8):
    """Chord pad: per note `voices` detuned PolyBLEP saws spread across the stereo field, low-passed.
    Returns stereo (n, 2)."""
    n = n_of(dur + release)
    t = t_of(n)
    out = np.zeros((n, 2))
    for f in freqs:
        for v in range(voices):
            pos = (v / (voices - 1) * 2 - 1) * width if voices > 1 else 0.0
            c = (v / (voices - 1) * 2 - 1) * detune if voices > 1 else 0.0
            lfo = 1 + vibrato * np.sin(2 * np.pi * rng.uniform(4.2, 5.4) * t + rng.uniform(0, 6.28))
            x = saw(f * cents(c + rng.normal(0, 1.0)) * lfo, n, phase0=rng.random())
            ang = (pos + 1) * np.pi / 4
            out[:, 0] += x * np.cos(ang)
            out[:, 1] += x * np.sin(ang)
    out = lowpass(out, cutoff, 2)
    out = lowpass(out, cutoff * 1.6, 1)
    env = attack_ramp(n, attack) * release_ramp(n, n_of(dur), release)
    out *= env[:, None]
    return vel * out / np.sqrt(len(freqs) * voices)


def bass(freq, dur, vel, rng, pluck=0.6, bright=1.0):
    """Round bass: sine plus a few soft harmonics; `pluck` sets how quickly it decays while held."""
    n = n_of(dur + 0.12)
    t = t_of(n)
    h2 = 0.32 * bright * (0.6 + 0.4 * vel)
    amps = [1.0, h2, 0.13 * bright, 0.05 * bright]
    x = harmonics(freq, [a * (1 + 0.6 * np.exp(-t / 0.05)) if k else a for k, a in enumerate(amps)], n)
    hold = np.exp(-t * pluck * 1.4)
    env = attack_ramp(n, 0.006) * (0.45 + 0.55 * hold) * release_ramp(n, n_of(dur), 0.1)
    return 0.8 * vel * x * env / 1.3


def synth_bass(freq, dur, vel, rng):
    """Lab bass: soft square through a closing low-pass, bouncy."""
    return synth_pluck(freq, dur, vel, rng, wave="square", cutoff=(1100.0, 220.0), decay=0.35, q=0.9) * 1.1


def theremin(freq, dur, vel, rng, glide_from=None):
    """Eerie sine-ish lead with delayed vibrato and an optional portamento into the note."""
    n = n_of(dur + 0.35)
    t = t_of(n)
    f = np.full(n, float(freq))
    if glide_from is not None:
        f = freq + (glide_from - freq) * np.exp(-t / 0.09)
    vib = 1 + 0.006 * np.clip((t - 0.25) / 0.6, 0, 1) * np.sin(2 * np.pi * 5.2 * t)
    x = harmonics(f * vib, [1.0, 0.12, 0.05], n)
    env = attack_ramp(n, 0.12) * release_ramp(n, n_of(dur), 0.3)
    return 0.6 * vel * x * env


# ----------------------------------------------------------------------------------------------
# Percussion (all mono, dur ignored except where noted)


def kick(vel, rng, f_hi=110.0, f_lo=47.0, decay=0.16, click=0.12):
    n = n_of(decay * 3)
    t = t_of(n)
    f = f_lo + (f_hi - f_lo) * np.exp(-t / 0.028)
    x = np.sin(2 * np.pi * phase_of(f)) * np.exp(-t / decay) * attack_ramp(n, 0.002)
    x += 0.15 * np.sin(4 * np.pi * phase_of(f)) * np.exp(-t / (decay * 0.4))
    c = lowpass(rng.standard_normal(n_of(0.004)), 2500, 2) * t60_env(n_of(0.004), 0.004) * click
    x[: len(c)] += c
    x *= release_ramp(n, n - n_of(0.05), 0.05)
    return 0.9 * vel * x


def frame_drum(vel, rng):
    """Soft low hand drum (dungeon heartbeat)."""
    return kick(vel, rng, f_hi=95.0, f_lo=62.0, decay=0.22, click=0.05)


def brush(vel, rng, decay=0.11):
    """Brushed snare: a short swish of band-limited noise plus a faint drum-head tone."""
    n = n_of(decay * 4)
    t = t_of(n)
    x = bandpass(rng.standard_normal(n), 900, 6500, 2)
    env = attack_ramp(n, 0.006) * np.exp(-t / decay)
    x = x * env / 3.0
    x += 0.25 * np.sin(2 * np.pi * phase_of(185 + 30 * np.exp(-t / 0.02))) * np.exp(-t / 0.05)
    x = lowpass(x, 7000, 2) * release_ramp(n, n - n_of(0.04), 0.04)
    return 0.7 * vel * x


def shaker(vel, rng, decay=0.035):
    n = n_of(decay * 4 + 0.02)
    t = t_of(n)
    x = bandpass(rng.standard_normal(n), 3500, 9000, 2)
    env = attack_ramp(n, 0.012) * np.exp(-np.maximum(t - 0.012, 0) / decay)
    x = lowpass(x * env, 8500, 2) * release_ramp(n, n - n_of(0.02), 0.02)
    return 0.35 * vel * x


def rim(vel, rng, freq=1150.0):
    """Wooden rim click / woodblock."""
    n = n_of(0.08)
    x = modal(freq, [(1.0, 1.0, 0.045), (2.37, 0.45, 0.025), (4.1, 0.2, 0.012)], n, rng, attack=0.0004)
    c = _mallet(n_of(0.002), rng, 1500, 6000) * 0.2
    x[: len(c)] += c
    x *= release_ramp(n, n - n_of(0.02), 0.02)
    return 0.5 * vel * x


def tick(vel, rng, freq=2300.0):
    """Clockwork tick: tiny metallic woodblock."""
    n = n_of(0.05)
    x = modal(freq, [(1.0, 1.0, 0.018), (1.73, 0.6, 0.012), (2.91, 0.3, 0.008)], n, rng, attack=0.0003)
    x *= release_ramp(n, n - n_of(0.012), 0.012)
    return 0.4 * vel * x


def clank(vel, rng, freq=420.0):
    """Muted metal clank (gears), used sparingly in the lab."""
    n = n_of(0.45)
    x = modal(freq, [(1.0, 1.0, 0.25), (2.76, 0.7, 0.18), (4.07, 0.5, 0.12), (5.93, 0.3, 0.08),
                     (8.21, 0.15, 0.05)], n, rng, attack=0.0008)
    x = lowpass(x, 5000, 2) * release_ramp(n, n - n_of(0.05), 0.05)
    return 0.45 * vel * x


def drip(vel, rng, freq=1100.0):
    """Water drop 'plip': a sine whose pitch shoots up as the bubble resonance collapses."""
    n = n_of(0.09)
    t = t_of(n)
    f = freq * (1 + 1.2 * np.clip(t / 0.03, 0, 1) ** 1.5)
    x = np.sin(2 * np.pi * phase_of(f)) * np.exp(-t / 0.018) * attack_ramp(n, 0.001)
    x *= release_ramp(n, n - n_of(0.02), 0.02)
    return 0.5 * vel * x


def ratchet(vel, rng, clicks=6, spacing=0.018):
    """Run of small gear clicks."""
    n = n_of(clicks * spacing + 0.05)
    x = np.zeros(n)
    for i in range(clicks):
        c = tick(rng.uniform(0.6, 1.0), rng, freq=rng.uniform(2600, 3200))
        place(x, c, n_of(i * spacing + abs(rng.normal(0, 0.0015))))
    return vel * x
