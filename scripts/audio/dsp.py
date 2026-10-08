"""Core DSP helpers shared by the SFX and music generators.

Conventions: sample rate SR, mono signals are 1-D float64 arrays, stereo signals are (n, 2) arrays.
"""

import numpy as np
from scipy import ndimage, signal

SR = 44100
NYQUIST = SR / 2


def n_of(seconds):
    return int(round(seconds * SR))


def t_of(n):
    return np.arange(n) / SR


def db(gain):
    return 20 * np.log10(gain)


def undb(decibels):
    return 10 ** (decibels / 20)


def midi_hz(m):
    return 440.0 * 2 ** ((np.asarray(m, dtype=float) - 69) / 12)


def cents(c):
    return 2 ** (c / 1200)


def t60_env(n, t60):
    """Exponential decay reaching -60 dB after t60 seconds."""
    return 10 ** (-3 * t_of(n) / t60)


def attack_ramp(n, attack):
    """Raised-cosine ramp from 0 to 1 over `attack` seconds, then 1."""
    a = max(n_of(attack), 1)
    ramp = np.ones(n)
    k = min(a, n)
    ramp[:k] = 0.5 - 0.5 * np.cos(np.pi * np.arange(k) / a)
    return ramp


def release_ramp(n, start, release):
    """1 until sample `start`, then a raised-cosine fall to exactly 0 over `release` seconds."""
    r = max(n_of(release), 1)
    out = np.ones(n)
    seg = np.arange(max(n - start, 0))
    out[start:] = np.where(seg < r, 0.5 + 0.5 * np.cos(np.pi * np.minimum(seg, r) / r), 0.0)
    return out


def fade(x, fade_in=0.0, fade_out=0.0):
    x = x.copy()
    n = len(x)
    shape = (n,) + (1,) * (x.ndim - 1)
    if fade_in > 0:
        x *= attack_ramp(n, fade_in).reshape(shape)
    if fade_out > 0:
        x *= release_ramp(n, n - n_of(fade_out), fade_out).reshape(shape)
    return x


def place(dst, src, start):
    """Add src into dst starting at sample `start` (clipped to dst's bounds)."""
    s0 = max(start, 0)
    s1 = min(start + len(src), len(dst))
    if s1 > s0:
        dst[s0:s1] += src[s0 - start : s1 - start]
    return dst


def mix(*parts):
    """Sum signals of different lengths, each given as (signal, start_seconds)."""
    n = max(n_of(start) + len(x) for x, start in parts)
    out = np.zeros((n,) + parts[0][0].shape[1:])
    for x, start in parts:
        place(out, x, n_of(start))
    return out


def pan(x, position):
    """Constant-power pan of a mono signal; position in [-1 (left), 1 (right)]."""
    angle = (position + 1) * np.pi / 4
    return np.stack([x * np.cos(angle), x * np.sin(angle)], axis=1)


# ----------------------------------------------------------------------------------------------
# Noise


def noise(n, rng):
    return rng.standard_normal(n)


def crackle(n, rng, density, decay=0.0015, density_env=None):
    """Sparse random clicks (Poisson process, `density` clicks per second, optionally time-varying)."""
    rate = density * (np.ones(n) if density_env is None else density_env)
    hits = rng.random(n) < rate / SR
    impulses = np.where(hits, rng.uniform(0.2, 1.0, n) * rng.choice([-1, 1], n), 0.0)
    k = n_of(decay * 6)
    kernel = np.exp(-t_of(k) / decay)
    return signal.fftconvolve(impulses, kernel)[:n]


# ----------------------------------------------------------------------------------------------
# Filters


def _sos(kind, freq, order):
    return signal.butter(order, freq, btype=kind, fs=SR, output="sos")


def lowpass(x, freq, order=2):
    return signal.sosfilt(_sos("lowpass", freq, order), x, axis=0)


def highpass(x, freq, order=2):
    return signal.sosfilt(_sos("highpass", freq, order), x, axis=0)


def bandpass(x, lo, hi, order=2):
    return signal.sosfilt(_sos("bandpass", [lo, hi], order), x, axis=0)


def biquad(kind, f0, q=0.707, gain_db=0.0):
    """RBJ cookbook biquad as a single sos row."""
    a_lin = 10 ** (gain_db / 40)
    w0 = 2 * np.pi * f0 / SR
    alpha = np.sin(w0) / (2 * q)
    cw = np.cos(w0)
    if kind == "peak":
        b = [1 + alpha * a_lin, -2 * cw, 1 - alpha * a_lin]
        a = [1 + alpha / a_lin, -2 * cw, 1 - alpha / a_lin]
    elif kind in ("lowshelf", "highshelf"):
        sq = 2 * np.sqrt(a_lin) * alpha
        sign = 1 if kind == "lowshelf" else -1
        b = [
            a_lin * ((a_lin + 1) - sign * (a_lin - 1) * cw + sq),
            sign * 2 * a_lin * ((a_lin - 1) - sign * (a_lin + 1) * cw),
            a_lin * ((a_lin + 1) - sign * (a_lin - 1) * cw - sq),
        ]
        a = [
            (a_lin + 1) + sign * (a_lin - 1) * cw + sq,
            -sign * 2 * ((a_lin - 1) + sign * (a_lin + 1) * cw),
            (a_lin + 1) + sign * (a_lin - 1) * cw - sq,
        ]
    elif kind == "highpass":
        b = [(1 + cw) / 2, -(1 + cw), (1 + cw) / 2]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    elif kind == "bandpass":
        b = [alpha, 0, -alpha]
        a = [1 + alpha, -2 * cw, 1 - alpha]
    else:
        raise ValueError(kind)
    return np.concatenate([np.array(b) / a[0], np.array(a) / a[0]])[None, :]


def eq(x, *bands):
    """Apply a chain of RBJ biquads given as (kind, f0, q, gain_db) tuples."""
    sos = np.concatenate([biquad(*band) for band in bands])
    return signal.sosfilt(sos, x, axis=0)


def svf(x, cutoff, q=0.707, mode="lp"):
    """Topology-preserving-transform state-variable filter with per-sample cutoff/Q (mono)."""
    n = len(x)
    fc = np.clip(np.broadcast_to(np.asarray(cutoff, dtype=float), (n,)), 5.0, SR * 0.49)
    g = np.tan(np.pi * fc / SR)
    k = 1 / np.broadcast_to(np.asarray(q, dtype=float), (n,))
    a1 = 1 / (1 + g * (g + k))
    a2 = g * a1
    a3 = g * a2
    lp = np.empty(n)
    bp = np.empty(n)
    ic1 = ic2 = 0.0
    for i, (xi, b1, b2, b3) in enumerate(zip(x.tolist(), a1.tolist(), a2.tolist(), a3.tolist())):
        v3 = xi - ic2
        v1 = b1 * ic1 + b2 * v3
        v2 = ic2 + b2 * ic1 + b3 * v3
        ic1 = 2 * v1 - ic1
        ic2 = 2 * v2 - ic2
        lp[i] = v2
        bp[i] = v1
    if mode == "lp":
        return lp
    if mode == "bp":
        return bp * k  # unity gain at the centre frequency
    if mode == "hp":
        return x - k * bp - lp
    raise ValueError(mode)


def dc_block(x, freq=20.0):
    return highpass(x, freq, order=2)


# ----------------------------------------------------------------------------------------------
# Oscillators


def phase_of(freq, n=None, phase0=0.0):
    """Instantaneous phase in cycles for a (possibly time-varying) frequency in Hz."""
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,) if n is not None else np.shape(freq))
    return phase0 + np.concatenate([[0.0], np.cumsum(f[:-1])]) / SR


def harmonics(freq, amps, n=None, phase0=0.0, rolloff_hz=NYQUIST * 0.9):
    """Band-limited additive tone. amps[k] is the amplitude of harmonic k+1, either a scalar or a
    per-sample array (time-varying spectrum). Harmonics above `rolloff_hz` are faded out."""
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,) if n is not None else np.shape(freq))
    ph = 2 * np.pi * phase_of(f, None, phase0)
    out = np.zeros(len(f))
    fmax = np.max(f)
    for k, a in enumerate(amps, start=1):
        if k * np.min(f) >= rolloff_hz:
            break
        guard = np.clip((rolloff_hz - k * f) / (0.1 * rolloff_hz), 0, 1) if k * fmax > 0.9 * rolloff_hz else 1.0
        out += a * guard * np.sin(k * ph)
    return out


def saw_amps(k_count):
    return [(-1) ** (k + 1) / k for k in range(1, k_count + 1)]


def square_amps(k_count):
    return [1 / k if k % 2 else 0.0 for k in range(1, k_count + 1)]


def _polyblep(t, dt):
    out = np.zeros_like(t)
    m = t < dt
    x = t[m] / dt[m]
    out[m] = x + x - x * x - 1
    m = t > 1 - dt
    x = (t[m] - 1) / dt[m]
    out[m] = x * x + x + x + 1
    return out


def saw(freq, n=None, phase0=0.0):
    """PolyBLEP band-limited sawtooth in [-1, 1]."""
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,) if n is not None else np.shape(freq))
    t = np.mod(phase_of(f, None, phase0), 1.0)
    return 2 * t - 1 - _polyblep(t, f / SR)


def fm(carrier, ratio, index, n, phase0=0.0):
    """Two-operator FM: sin(carrier phase + index * sin(modulator phase)); index may vary per sample."""
    mod = np.sin(2 * np.pi * phase_of(np.asarray(carrier) * ratio, n))
    return np.sin(2 * np.pi * phase_of(carrier, n, phase0) + index * mod)


def modal(freq, partials, n, rng=None, attack=0.0005):
    """Sum of exponentially decaying sinusoids. partials = [(ratio, amp, t60), ...]."""
    t = t_of(n)
    out = np.zeros(n)
    for ratio, amp, t60 in partials:
        f = freq * ratio
        if f >= NYQUIST * 0.9:
            continue
        ph = rng.uniform(0, 0.1) if rng is not None else 0.0
        out += amp * np.exp(-6.9078 * t / t60) * np.sin(2 * np.pi * (f * t + ph))
    return out * attack_ramp(n, attack)


# ----------------------------------------------------------------------------------------------
# Delay-based effects


def frac_delay(x, delay_samples):
    """Read x at time n - delay(n) with linear interpolation (mono, delay may vary per sample)."""
    n = len(x)
    pos = np.arange(n) - np.broadcast_to(delay_samples, (n,))
    i0 = np.floor(pos).astype(int)
    frac = pos - i0
    valid0 = (i0 >= 0) & (i0 < n)
    valid1 = (i0 + 1 >= 0) & (i0 + 1 < n)
    a = np.where(valid0, x[np.clip(i0, 0, n - 1)], 0.0)
    b = np.where(valid1, x[np.clip(i0 + 1, 0, n - 1)], 0.0)
    return a * (1 - frac) + b * frac


# ----------------------------------------------------------------------------------------------
# Reverb


def reverb_ir(t60, rng, predelay=0.012, hf_t60_ratio=0.35, hf_corner=2500.0, low_cut=150.0,
              early=((0.011, 0.6), (0.019, 0.45), (0.027, 0.4), (0.041, 0.3), (0.053, 0.25), (0.067, 0.2)),
              channels=2):
    """Synthetic room impulse response: sparse early reflections + a dense, decorrelated noise tail
    whose decay time falls with frequency. Returns (n, channels), normalised to unit energy per channel."""
    n = n_of(predelay + t60 * 1.15)
    nper, hop = 1024, 256
    freqs = np.fft.rfftfreq(nper, 1 / SR)
    t60_f = t60 * (hf_t60_ratio + (1 - hf_t60_ratio) / (1 + (freqs / hf_corner) ** 2))
    frames = n // hop + 4
    times = np.arange(frames) * hop / SR
    env = 10 ** (-3 * times[None, :] / t60_f[:, None])
    out = np.zeros((n, channels))
    for ch in range(channels):
        spec = (rng.standard_normal(env.shape) + 1j * rng.standard_normal(env.shape)) * env
        _, tail = signal.istft(spec, fs=SR, nperseg=nper, noverlap=nper - hop)
        tail = tail[:n] * attack_ramp(n, 0.035)  # diffusion build-up
        ir = np.zeros(n)
        start = n_of(predelay)
        ir[start:] = tail[: n - start]
        er = np.zeros(n)
        for i, (delay, gain) in enumerate(early):
            jitter = rng.uniform(0.85, 1.15)
            er[n_of(predelay + delay * jitter) + ch * 7 * (i % 2)] += gain * rng.choice([-1, 1])
        er = lowpass(er, 6000, 1)
        ir = ir / np.sqrt(np.sum(ir**2)) + 0.5 * er
        ir = highpass(ir, low_cut, 2)
        out[:, ch] = ir / np.sqrt(np.sum(ir**2))
    return out


def reverb_mono(x, ir, wet, dry=1.0):
    """Linear (non-circular) convolution reverb for one-shot mono sounds; output includes the tail."""
    w = signal.fftconvolve(x, ir[:, 0])
    out = wet * w
    out[: len(x)] += dry * x
    return out


# ----------------------------------------------------------------------------------------------
# Dynamics


def soft_clip(x, drive=1.0):
    return np.tanh(drive * x) / np.tanh(drive)


def limit(x, ceiling_db, window=0.004, circular=False):
    """Look-ahead peak limiter with no recursion: needed gain reduction (dB) is max-held over
    2*window, then moving-averaged over `window`, which guarantees every peak lands under the
    ceiling while keeping the gain curve smooth. Works on mono or stereo (linked). `circular`
    treats the signal as a seamless loop."""
    a = np.abs(x) if x.ndim == 1 else np.max(np.abs(x), axis=1)
    reduction = np.maximum(0.0, db(np.maximum(a, 1e-12)) - ceiling_db)
    if not np.any(reduction > 0):
        return x
    w = max(n_of(window), 1)
    mode = "wrap" if circular else "nearest"
    held = ndimage.maximum_filter1d(reduction, size=2 * w + 1, mode=mode)
    smooth = ndimage.uniform_filter1d(held, size=w, mode=mode)
    gain = undb(-smooth)
    return x * (gain if x.ndim == 1 else gain[:, None])


# ----------------------------------------------------------------------------------------------
# Analysis


def peak_db(x):
    return db(max(np.max(np.abs(x)), 1e-12))


def rms_db(x):
    return db(max(np.sqrt(np.mean(np.square(x))), 1e-12))


def active_rms_db(x, rel_db=-20.0, window=0.010):
    """RMS over the 'active' part: samples whose short-term RMS is within rel_db of the loudest."""
    p = np.square(x) if x.ndim == 1 else np.mean(np.square(x), axis=1)
    short = ndimage.uniform_filter1d(p, size=max(n_of(window), 1), mode="constant")
    active = short >= np.max(short) * undb(rel_db) ** 2
    return db(max(np.sqrt(np.mean(p[active])), 1e-12))


def active_loudness(x, rel_db=-20.0, window=0.010):
    """K-weighted loudness (LUFS scale) over the active part, like active_rms_db. A mono signal is
    counted as played on both speakers, so effects compare directly with stereo music LUFS."""
    k = k_weight(x)
    p = 2 * np.square(k) if x.ndim == 1 else np.sum(np.square(k), axis=1)
    short = ndimage.uniform_filter1d(p, size=max(n_of(window), 1), mode="constant")
    active = short >= np.max(short) * undb(rel_db) ** 2
    return -0.691 + 10 * np.log10(max(np.mean(p[active]), 1e-12))


def k_weight(x):
    """ITU-R BS.1770 K-weighting (shelf + RLB high-pass), re-derived for SR as libebur128 does."""
    k = np.tan(np.pi * 1681.974450955533 / SR)
    q = 0.7071752369554196
    vh = 10 ** (3.999843853973347 / 20)
    vb = vh**0.4996667741545416
    a0 = 1 + k / q + k * k
    shelf = [(vh + vb * k / q + k * k) / a0, 2 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0,
             1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
    k = np.tan(np.pi * 38.13547087602444 / SR)
    q = 0.5003270373238773
    a0 = 1 + k / q + k * k
    rlb = [1, -2, 1, 1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
    return signal.sosfilt(np.array([shelf, rlb]), x, axis=0)


def lufs(x):
    """Integrated loudness (BS.1770-4, gated) of a mono or stereo signal at least 0.4 s long."""
    y = k_weight(x if x.ndim == 2 else x[:, None])
    block, hop = n_of(0.4), n_of(0.1)
    starts = np.arange(0, len(y) - block + 1, hop)
    csum = np.concatenate([np.zeros((1, y.shape[1])), np.cumsum(y**2, axis=0)])
    z = np.sum((csum[starts + block] - csum[starts]) / block, axis=1)
    loud = -0.691 + 10 * np.log10(np.maximum(z, 1e-12))
    z = z[loud > -70]
    rel = -0.691 + 10 * np.log10(np.mean(z)) - 10
    z = z[-0.691 + 10 * np.log10(np.maximum(z, 1e-12)) > rel]
    return -0.691 + 10 * np.log10(np.mean(z))
