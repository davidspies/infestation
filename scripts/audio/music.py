"""Loop sequencer: a tiny note notation, rendering of parts into a seamless loop, and the mix bus.

Seamlessness: every note is rendered in full and anything that rings past the loop end is folded
back onto the loop start (and early-humanised notes before bar 1 onto the loop end), exactly as if
the loop had been played forever. Reverb and delay are circular convolutions of the folded send
buses, so the file's start already contains the tails of its end.
"""

import re
from dataclasses import dataclass
from fractions import Fraction

import numpy as np

from dsp import SR, highpass, limit, lowpass, lufs, midi_hz, n_of, pan, peak_db, undb

TARGET_LUFS = -18.0
CEILING_DB = -1.5  # sample-peak ceiling before encoding (the codec overshoots a little)
EIGHTHS = (0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5)
OFFBEATS = (0.5, 1.5, 2.5, 3.5)
NOTE_RE = re.compile(r"^([A-G])(#|b)?(-?\d)$")
SEMITONES = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}


def midi(name):
    m = NOTE_RE.match(name)
    if m is None:
        raise ValueError(f"bad note name {name!r}")
    letter, accidental, octave = m.groups()
    return 12 * (int(octave) + 1) + SEMITONES[letter] + {"#": 1, "b": -1, None: 0}[accidental]


@dataclass(frozen=True)
class Note:
    beat: float  # onset, in beats from the loop start
    dur: float  # held length in beats
    pitch: object  # MIDI note number, or a tuple of them for a chord voiced by one instrument
    vel: float


def bars(text, start_bar=0, vel=0.8, beats_per_bar=4):
    """Parse bars separated by '|'. Tokens: 'F#5:1.5' (note:beats), 'r:1' (rest),
    'D4+F#4+A4:2' (chord), optional '@0.9' velocity suffix. Durations may be fractions ('1/3').
    Each bar must add up to exactly `beats_per_bar`."""
    notes = []
    for i, bar in enumerate(text.split("|")):
        beat = Fraction(0)
        for token in bar.split():
            body, _, v = token.partition("@")
            pitches, _, dur = body.partition(":")
            d = Fraction(dur)
            if pitches != "r":
                for p in pitches.split("+"):
                    notes.append(Note(float((start_bar + i) * beats_per_bar + beat), float(d), midi(p),
                                      float(v) if v else vel))
            beat += d
        if beat != beats_per_bar:
            raise ValueError(f"bar {start_bar + i + 1} has {beat} beats: {bar!r}")
    return notes


def hits(pattern, bars_range, vel=0.8, beats_per_bar=4, accents=None):
    """Drum hits at the given beat offsets (within each bar) for every bar in `bars_range`.
    `accents` maps beat offset -> velocity multiplier."""
    accents = accents or {}
    return [Note(b * beats_per_bar + off, 0.25, 0, vel * accents.get(off, 1.0))
            for b in bars_range for off in pattern]


def shift(notes, beats=0.0, semitones=0, vel=1.0):
    return [Note(n.beat + beats, n.dur, n.pitch + semitones, n.vel * vel) for n in notes]


def phrased(notes, phrase_beats=16, depth=0.14, lift=0.012):
    """Musical dynamics for a melodic line: a gentle swell and ebb across each phrase, and higher
    notes slightly louder than lower ones."""
    centre = np.mean([n.pitch for n in notes])
    return [Note(n.beat, n.dur, n.pitch,
                 n.vel * (1 - depth / 2 + depth * np.sin(np.pi * (n.beat % phrase_beats) / phrase_beats))
                 * (1 + lift * (n.pitch - centre)))
            for n in notes]


class LoopBuffer:
    """Stereo accumulation buffer with head/tail room that folds into a seamless loop."""

    def __init__(self, length, pre=1.0, post=8.0):
        self.length = length
        self.pre = n_of(pre)
        self.buf = np.zeros((self.pre + length + n_of(post), 2))

    def add(self, x, t):
        start = self.pre + n_of(t)
        if start < 0 or start + len(x) > len(self.buf):
            raise ValueError(f"event at {t:.3f}s of length {len(x) / SR:.2f}s exceeds the loop buffer")
        self.buf[start : start + len(x)] += x

    def folded(self):
        n, pre = self.length, self.pre
        out = self.buf[pre : pre + n].copy()
        tail = self.buf[pre + n :]
        out[: len(tail)] += tail
        out[n - pre :] += self.buf[:pre]
        return out


def circular_convolve(x, ir):
    """Steady-state response of a periodic signal (one period x, shape (n, 2)) to a stereo IR."""
    n = len(x)
    assert len(ir) <= n
    spec_x = np.fft.rfft(x, axis=0)
    spec_ir = np.fft.rfft(ir, n=n, axis=0)
    return np.fft.irfft(spec_x * spec_ir, n=n, axis=0)


def circular_filter(x, fn, warmup=1.0):
    """Apply a causal filter `fn` to a periodic signal in its steady state."""
    m = n_of(warmup)
    return fn(np.concatenate([x[-m:], x]))[m:]


def pingpong_ir(delay, feedback, taps=6, lp=3500.0):
    """Stereo ping-pong echo IR (no dry path): taps alternate left/right and darken as they repeat."""
    n = n_of(delay * (taps + 1))
    ir = np.zeros((n, 2))
    for k in range(1, taps + 1):
        imp = np.zeros(n)
        imp[n_of(delay * k)] = feedback ** (k - 1)
        ir[:, (k - 1) % 2] += lowpass(imp, lp / k**0.5, 1)
    return ir


@dataclass
class Part:
    notes: list
    instrument: object  # fn(freq, dur_s, vel, rng) -> mono or stereo array
    gain_db: float = 0.0
    pan: float = 0.0
    reverb: float = 0.25  # send level
    delay: float = 0.0  # send level
    humanize: float = 0.006  # timing jitter (s, standard deviation)
    vel_jitter: float = 0.06
    strum: float = 0.0  # spread of simultaneous notes (s), for chords
    pitch_hz: bool = True  # False for unpitched percussion (instrument gets vel, rng only)


class Song:
    def __init__(self, bpm, n_bars, seed, beats_per_bar=4):
        self.spb = 60.0 / bpm
        self.length = n_of(n_bars * beats_per_bar * self.spb)
        self.rng = np.random.default_rng(seed)
        self.dry = np.zeros((self.length, 2))
        self.reverb_send = np.zeros((self.length, 2))
        self.delay_send = np.zeros((self.length, 2))

    def add(self, part):
        buf = LoopBuffer(self.length)
        rng = self.rng
        onsets = {}
        for note in sorted(part.notes, key=lambda nt: (nt.beat, np.min(nt.pitch))):
            k = onsets.get(note.beat, 0)
            onsets[note.beat] = k + 1
            t = note.beat * self.spb + rng.normal(0, part.humanize) + k * part.strum
            vel = float(np.clip(note.vel * (1 + rng.normal(0, part.vel_jitter)), 0.05, 1.0))
            if part.pitch_hz:
                freq = midi_hz(note.pitch)
                x = part.instrument(float(freq) if freq.ndim == 0 else freq, note.dur * self.spb, vel, rng)
            else:
                x = part.instrument(vel, rng)
            buf.add(x if x.ndim == 2 else pan(x, part.pan), t)
        stem = buf.folded() * undb(part.gain_db)
        self.dry += stem
        self.reverb_send += stem * part.reverb
        self.delay_send += stem * part.delay

    def mixdown(self, reverb_ir, delay_ir):
        """Echoes (40% of which also feed the reverb) + reverb + dry, high-passed, normalised to
        TARGET_LUFS and safety-limited, all in the loop's steady state."""
        echoes = circular_convolve(self.delay_send, delay_ir)
        wet = circular_convolve(self.reverb_send + 0.4 * echoes, reverb_ir)
        out = circular_filter(self.dry + echoes + wet, lambda x: highpass(x, 32.0, 2))
        out *= undb(TARGET_LUFS - lufs(out))
        self.limiter_db = max(peak_db(out) - CEILING_DB, 0.0)  # how hard the safety limiter works
        self.audio = limit(out, CEILING_DB, window=0.003, circular=True)
        return self
