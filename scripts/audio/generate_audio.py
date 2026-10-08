#!/usr/bin/env python3
"""Procedurally generate every sound effect and music loop for Infestation.

Run from the repository root:

    python3 scripts/audio/generate_audio.py            # everything
    python3 scripts/audio/generate_audio.py sfx        # only assets/sfx
    python3 scripts/audio/generate_audio.py music      # only assets/music

Requires Python 3 with numpy and scipy, and an ffmpeg build with libvorbis and libmp3lame on PATH.
All randomness is seeded, and ffmpeg runs in bit-exact mode, so reruns reproduce identical files.

Output contract (every file in two formats: OGG for the native build, MP3 for the web build):
  assets/sfx/<name>.ogg    mono 44.1 kHz Vorbis q4, silence-trimmed, faded, peak <= -1 dBFS
  assets/sfx/<name>.mp3    the same sound as MP3, 96 kbps CBR
  assets/music/<name>.ogg  stereo 44.1 kHz Vorbis q3, seamless loops (tails wrap to the start)
  assets/music/<name>.mp3  the same loop as MP3, LAME VBR -q:a 4.5, with the gapless (LAME) header

After writing, every file is decoded again and checked (peak, loudness, clean start/end, DC,
exact length and zero offset against the source, and for music the loop seam); a summary table is
printed.
"""

import argparse
import sys
import zlib
from pathlib import Path

import numpy as np
from scipy import signal

import sfx
import songs
from codec import decode, encode_mp3, encode_vorbis, lewton_length
from dsp import SR, active_loudness, active_rms_db, fade, highpass, limit, lufs, n_of, peak_db, rms_db, undb

ROOT = Path(__file__).resolve().parents[2]
SFX_DIR = ROOT / "assets" / "sfx"
MUSIC_DIR = ROOT / "assets" / "music"
PEAK_LIMIT_DB = -1.0
# Digital silence around every effect (one Vorbis short block, 5.8 ms). At the start it keeps the
# codec's pre-echo of sharp attacks out of the first samples, so playback starts at exactly zero.
# At the end it hides a decoder disagreement on short (single Ogg page) files: lewton (the native
# mixer's decoder) keeps the encoder's padding while ffmpeg-based decoders drop the last 128 samples.
SFX_GUARD = 256


def encode_checked(path, x, encode_fn):
    """Encode with encode_fn(path, x), backing off the gain while the codec's overshoot pushes the
    decoded peak above PEAK_LIMIT_DB. Returns the decoded audio."""
    channels = 1 if x.ndim == 1 else x.shape[1]
    for _ in range(6):
        encode_fn(path, x)
        decoded = decode(path, channels)
        over = peak_db(decoded) - (PEAK_LIMIT_DB - 0.05)
        if over <= 0:
            return decoded
        x = x * undb(-over - 0.05)
    raise RuntimeError(f"{path}: cannot bring the decoded peak under {PEAK_LIMIT_DB} dBFS")


def vorbis_sfx(path, x):
    encode_vorbis(path, x, 4)


def vorbis_loop(path, x):
    """Choose an Ogg paging under which lewton (the native decoder) keeps the loop's exact length."""
    for page_us in (1_000_000, 900_000, 1_100_000, 800_000, 1_200_000, 700_000):
        encode_vorbis(path, x, 3, page_us)
        native, expected = lewton_length(path)
        if native == expected == len(x):
            return
    raise RuntimeError(f"{path}: no Ogg paging decodes to exactly {len(x)} samples in lewton")


def mp3_sfx(path, x):
    encode_mp3(path, x, ["-b:a", "96k"])


def mp3_loop(path, x):
    encode_mp3(path, x, ["-q:a", "4.5"])  # ~130 kbps on these tracks; -q:a 4 averages ~150 kbps


def lag(source, decoded):
    """Offset (samples) of `decoded` relative to `source`, from their cross-correlation; 0 = aligned."""
    a, b = source[: n_of(5.0)], decoded[: n_of(5.0)]
    if a.ndim == 2:
        a, b = a.mean(axis=1), b.mean(axis=1)
    corr = signal.correlate(b, a, mode="full", method="fft")
    window = min(n_of(0.1), len(a) - 1)
    centre = len(a) - 1
    return int(np.argmax(corr[centre - window : centre + window + 1])) - window


def finish_sfx(x, target_loudness):
    """High-pass, trim silence, fade in/out to exact zero, normalise loudness, limit peaks."""
    x = highpass(x, 20.0, 2)
    level = np.abs(x)
    peak = np.max(level)
    start = max(int(np.nonzero(level > peak * undb(-50))[0][0]) - n_of(0.001), 0)
    envelope = np.sqrt(np.convolve(x**2, np.ones(n_of(0.005)) / n_of(0.005), mode="same"))
    end = int(np.nonzero(envelope > peak * undb(-60))[0][-1]) + 1
    x = x[start:end]
    x = fade(x, fade_in=0.003, fade_out=float(np.clip(0.15 * len(x) / SR, 0.01, 0.08)))
    x = x * undb(target_loudness - active_loudness(x))
    limited = limit(x, -1.5, window=0.002)
    reduction = rms_db(x) - rms_db(limited)
    return np.concatenate([np.zeros(SFX_GUARD), limited, np.zeros(SFX_GUARD)]), reduction


def render_sfx():
    SFX_DIR.mkdir(parents=True, exist_ok=True)
    print(f"{'sfx':14s} {'dur s':>6s} {'peak':>6s} {'actRMS':>7s} {'actLU':>6s} {'limGR':>6s} "
          f"{'first':>8s} {'last':>8s} {'DC':>8s} {'ogg B':>7s} {'mp3 pk':>6s} {'mp3 B':>7s}")
    totals = {"ogg": 0, "mp3": 0}
    for name, (recipe, target) in sfx.SFX.items():
        rng = np.random.default_rng(zlib.crc32(name.encode()))
        x, reduction = finish_sfx(recipe(rng), target)
        y = encode_checked(SFX_DIR / f"{name}.ogg", x, vorbis_sfx)
        z = encode_checked(SFX_DIR / f"{name}.mp3", x, mp3_sfx)
        sizes = {ext: (SFX_DIR / f"{name}.{ext}").stat().st_size for ext in totals}
        for ext in totals:
            totals[ext] += sizes[ext]
        dc = abs(np.mean(y)) / (np.sqrt(np.mean(y**2)) + 1e-12)
        print(f"{name:14s} {len(x) / SR:6.3f} {peak_db(y):6.1f} {active_rms_db(y):7.1f} {active_loudness(y):6.1f} "
              f"{reduction:6.2f} {y[0]:+8.5f} {y[-1]:+8.5f} {dc:8.5f} {sizes['ogg']:7d} {peak_db(z):6.1f} "
              f"{sizes['mp3']:7d}")
        assert peak_db(y) <= PEAK_LIMIT_DB and abs(y[0]) < 1e-3 and abs(y[-1]) < 3e-3 and dc < 0.05, name
        # MP3's coarser transform blocks smear a little pre-echo (<= -45 dBFS) into the lead-in
        assert len(z) == len(x) and lag(x, z) == 0 and abs(z[0]) < 3e-3 and abs(z[-1]) < 3e-3, f"{name}.mp3"
    print(f"total assets/sfx: ogg {totals['ogg'] / 1024:.1f} KiB, mp3 {totals['mp3'] / 1024:.1f} KiB")


def render_music():
    MUSIC_DIR.mkdir(parents=True, exist_ok=True)
    print(f"{'music':14s} {'dur s':>6s} {'peak':>6s} {'RMS':>6s} {'LUFS':>6s} {'limGR':>6s} {'seam':>7s} "
          f"{'step99':>7s} {'lag':>4s} {'corrLR':>7s} {'bytes':>8s}")
    totals = {"ogg": 0, "mp3": 0}
    for name, compose in songs.SONGS.items():
        song = compose()
        x = song.audio
        for ext, encode_fn in (("ogg", vorbis_loop), ("mp3", mp3_loop)):
            path = MUSIC_DIR / f"{name}.{ext}"
            y = encode_checked(path, x, encode_fn)
            size = path.stat().st_size
            totals[ext] += size
            assert len(y) == len(x), f"{path.name}: decoded length {len(y)} != {len(x)}"
            offset = lag(x, y)
            steps = np.max(np.abs(np.diff(y, axis=0)), axis=1)
            seam = np.max(np.abs(y[0] - y[-1]))  # the jump the listener hears when the loop wraps
            corr = np.corrcoef(y[:, 0], y[:, 1])[0, 1]
            print(f"{path.name:14s} {len(y) / SR:6.2f} {peak_db(y):6.1f} {rms_db(y):6.1f} {lufs(y):6.1f} "
                  f"{song.limiter_db:6.2f} {seam:7.4f} {np.percentile(steps, 99):7.4f} {offset:4d} {corr:7.3f} "
                  f"{size:8d}")
            assert offset == 0, f"{path.name}: decoded audio is offset by {offset} samples"
            assert seam <= np.percentile(steps, 99.9), f"{path.name}: discontinuity at the loop seam"
    print(f"total assets/music: ogg {totals['ogg'] / 1024:.1f} KiB, mp3 {totals['mp3'] / 1024:.1f} KiB")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("groups", nargs="*", choices=["sfx", "music"], default=["sfx", "music"])
    args = parser.parse_args()
    if "sfx" in args.groups:
        render_sfx()
    if "music" in args.groups:
        render_music()


if __name__ == "__main__":
    sys.exit(main())
