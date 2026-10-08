"""The four music loops. Each function returns a mixed-down Song whose `audio` (stereo) loops
seamlessly."""

import numpy as np

import instruments as ins
from dsp import reverb_ir
from music import EIGHTHS, OFFBEATS, Note, Part, Song, bars, hits, midi, phrased, pingpong_ir, shift


def voicings(table):
    return {name: [midi(p) for p in notes.split()] for name, notes in table.items()}


def progression(text, start_bar=0, beats_per_bar=4):
    """'D:4 | Em7:2 A:2' -> [(beat, beats, chord_name), ...]"""
    out = []
    for i, bar in enumerate(text.split("|")):
        beat = (start_bar + i) * beats_per_bar
        for token in bar.split():
            name, _, d = token.partition(":")
            out.append((beat, float(d), name))
            beat += float(d)
        assert beat == (start_bar + i + 1) * beats_per_bar, bar
    return out


def arpeggio(prog, voicing, pattern, step=0.5, hold=1.0, vel=0.7, accent=1.15):
    """Broken chords: walk `pattern` (indices into the voicing) in steps of `step` beats."""
    notes = []
    for beat, length, name in prog:
        v = voicing[name]
        for k in range(int(round(length / step))):
            idx = pattern[k % len(pattern)]
            on_beat = (k * step) % 1 == 0
            notes.append(Note(beat + k * step, step * hold, v[idx], vel * (accent if on_beat else 1.0)))
    return notes


def stabs(prog, voicing, offsets, dur, vel):
    """Short chords struck at the given beat offsets within each chord span."""
    return [Note(beat + off, dur, p, vel) for beat, length, name in prog for off in offsets if off < length
            for p in voicing[name]]


def pad_chords(prog, voicing, vel=0.6):
    """One chord-note per chord span, for instruments that voice whole chords (pads)."""
    return [Note(beat, length, tuple(voicing[name]), vel) for beat, length, name in prog]


# ----------------------------------------------------------------------------------------------


def title_map():
    """Title screen and world map. D mixolydian/major, 110 BPM, 32 bars (A1 A2 B A3).

    A: a rising 'adventure' call (A-D-F#) answered by the flat-seventh C chord for mischief,
    sequenced up a fourth; bells answer the lute in the even bars. B: a sneaky staccato tune over
    a walking bass, with a cheeky borrowed Bb chord before the turnaround."""
    song = Song(110, 32, seed=1101)

    prog_a1 = "D:4 | C:4 | G:4 | D:4 | Bm:4 | G:4 | Em7:2 A:2 | A:4"
    prog_a2 = "D:4 | C:4 | G:4 | D:4 | Bm:4 | G:4 | Em7:2 A7:2 | D:4"
    prog_b = "Bm:4 | G:4 | D:4 | A:4 | Bm:4 | G:4 | Bb:4 | A7:4"
    prog_a3 = "D:4 | C:4 | G:4 | D:4 | Bm:4 | G:4 | Em7:2 A:2 | Asus4:2 A7:2"
    prog = (progression(prog_a1, 0) + progression(prog_a2, 8) + progression(prog_b, 16)
            + progression(prog_a3, 24))

    head = ("A4:.5 D5:.5 F#5:2 E5:.5 D5:.5 | E5:1.5 D5:.5 C5:1 G4:1 | B4:.5 D5:.5 G5:2 F#5:.5 E5:.5 "
            "| F#5:1 E5:.5 D5:.5 A4:2 | B4:.5 D5:.5 F#5:1.5 G5:.5 F#5:.5 E5:.5 | D5:1.5 B4:.5 G4:1 B4:1 ")
    melody_open = head + "| A4:.5 B4:.5 C#5:.5 D5:.5 E5:1 C#5:1 | E5:3 r:1"
    melody_close = head + "| A4:.5 B4:.5 C#5:.5 D5:.5 E5:1 G5:1 | F#5:1.5 E5:.5 D5:2"
    ornamented = ("A4:.5 D5:.5 F#5:1.5 G5:.25 F#5:.25 E5:.5 D5:.5 | E5:1.5 D5:.5 C5:.5 B4:.5 G4:1 "
                  "| B4:.5 D5:.5 G5:1.5 A5:.25 G5:.25 F#5:.5 E5:.5 | F#5:1 E5:.5 D5:.5 A4:2 "
                  "| B4:.5 D5:.5 F#5:1.5 G5:.5 F#5:.5 E5:.5 | D5:1.5 B4:.5 G4:1 B4:1 "
                  "| A4:.5 B4:.5 C#5:.5 D5:.5 E5:1 C#5:1 | E5:3 r:1")
    melody_b = ("F#5:.5 r:.5 D5:.5 r:.5 B4:.5 C#5:.5 D5:1 | E5:.5 r:.5 D5:.5 r:.5 B4:.5 A4:.5 G4:1 "
                "| F#4:.5 A4:.5 D5:.5 F#5:.5 A5:1.5 G5:.5 | F#5:1 E5:2 r:1 "
                "| F#5:.5 r:.5 D5:.5 r:.5 B4:.5 C#5:.5 D5:1 | E5:.5 r:.5 G5:.5 r:.5 B5:1 A5:1 "
                "| F5:.5 r:.5 D5:.5 r:.5 Bb4:1 D5:1 | C#5:.5 E5:.5 G5:1.5 F#5:.5 E5:1")
    melody = phrased(bars(melody_open, 0) + bars(melody_close, 8) + bars(ornamented, 24))
    sneaky = phrased(bars(melody_b, 16, vel=0.75))
    # bells answer in the gaps of the even bars
    answers_mid = "r:4 | r:2 G5:.5 C6:.5 E6:1 | r:4 | r:2 F#6:.5 E6:.5 D6:.5 A5:.5 | r:4 | r:2 B5:.5 D6:.5 G6:1 | r:4 "
    answers_a2 = answers_mid + "| r:1 F#6:.5 E6:.5 D6:1 r:1"
    answers_a3 = answers_mid + "| r:2 A5:.5 C#6:.5 E6:.5 G6:.5"
    bells = (bars(answers_a2, 8, vel=0.6) + bars(answers_a3, 24, vel=0.6)
             + [n for n in shift(bars(melody_b, 16, vel=0.45), semitones=12) if n.beat >= 20 * 4])

    harp_v = voicings({"D": "D3 A3 D4 F#4 A4", "C": "C3 G3 C4 E4 G4", "G": "G2 D3 G3 B3 D4",
                       "Bm": "B2 F#3 B3 D4 F#4", "Em7": "E3 B3 D4 G4 B4", "A": "A2 E3 A3 C#4 E4",
                       "A7": "A2 E3 G3 C#4 E4", "Asus4": "A2 E3 A3 D4 E4", "Bb": "Bb2 F3 Bb3 D4 F4"})
    stab_v = {k: v[2:] for k, v in harp_v.items()}
    pad_v = voicings({"D": "F#3 A3 D4", "C": "G3 C4 E4", "G": "G3 B3 D4", "Bm": "F#3 B3 D4",
                      "Em7": "G3 B3 E4", "A": "A3 C#4 E4", "A7": "G3 C#4 E4", "Asus4": "A3 D4 E4",
                      "Bb": "Bb3 D4 F4"})
    a_sections = [c for c in prog if c[0] < 64 or c[0] >= 96]
    b_section = [c for c in prog if 64 <= c[0] < 96]
    harp_arp = arpeggio(a_sections, harp_v, [0, 1, 2, 3, 4, 3, 2, 1], vel=0.55)
    harp_stabs = stabs(b_section, stab_v, OFFBEATS, dur=0.2, vel=0.5)

    bass_a = ("D2:1.5 D2:.5 A2:1 D2:1 | C2:1.5 C2:.5 G2:1 C2:1 | B1:1.5 B1:.5 D2:1 G2:1 | A1:1.5 A1:.5 D2:1 F#2:1 "
              "| B1:1.5 B1:.5 F#2:1 B1:1 | G1:1.5 G1:.5 D2:1 G1:1 | E2:1.5 E2:.5 A1:1.5 A1:.5 ")
    bass_b = ("B1:1 D2:1 F#2:1 D2:1 | G1:1 B1:1 D2:1 B1:1 | D2:1 F#2:1 A2:1 F#2:1 | A1:1 C#2:1 E2:1 C#2:1 "
              "| B1:1 D2:1 F#2:1 D2:1 | G1:1 B1:1 D2:1 G2:1 | Bb1:1 D2:1 F2:1 D2:1 | A1:1 C#2:1 E2:1 C#2:1")
    bassline = (bars(bass_a + "| A1:1.5 A1:.5 E2:1 C#2:1", 0) + bars(bass_a + "| D2:1.5 D2:.5 A1:1 C#2:1", 8)
                + bars(bass_b, 16, vel=0.7) + bars(bass_a + "| A1:1.5 A1:.5 E2:1 C#2:1", 24))

    kicks = (hits([0, 2], range(0, 16), 0.75) + hits([0, 2], range(20, 24), 0.7)
             + hits([0, 2], range(24, 32), 0.8) + hits([3.5], range(25, 32, 2), 0.5))
    brushes = hits([1, 3], range(8, 16), 0.7) + hits([1, 3], range(24, 32), 0.75)
    rims = hits([1, 3], range(16, 24), 0.6)
    shakes = hits(EIGHTHS, range(0, 32), 0.45, accents={k: 1.6 for k in OFFBEATS})

    def pad(f, d, v, rng):
        return ins.pad(f, d, v, rng, cutoff=1500, attack=0.6, release=1.2, detune=8)

    def stab_harp(f, d, v, rng):
        return ins.harp(f, d, v, rng, ring=0.25)

    def staccato_lute(f, d, v, rng):
        return ins.lute(f, d * 0.6, v, rng, ring=0.12)

    song.add(Part(melody, ins.lute, gain_db=0, pan=-0.1, reverb=0.22))
    song.add(Part(sneaky, staccato_lute, gain_db=0, pan=-0.1, reverb=0.25))
    song.add(Part(bells, ins.glock, gain_db=-9, pan=-0.3, reverb=0.3, delay=0.35, humanize=0.004))
    song.add(Part(harp_arp, ins.harp, gain_db=-9, pan=0.35, reverb=0.3))
    song.add(Part(harp_stabs, stab_harp, gain_db=-9, pan=0.3, reverb=0.3, strum=0.012))
    song.add(Part(bassline, ins.bass, gain_db=-8.5, reverb=0.05, humanize=0.003))
    song.add(Part(pad_chords([c for c in prog if c[0] >= 32], pad_v), pad, gain_db=-17, reverb=0.4,
                         humanize=0, vel_jitter=0))
    song.add(Part(kicks, ins.kick, gain_db=-7, reverb=0.05, pitch_hz=False, humanize=0.002))
    song.add(Part(brushes, ins.brush, gain_db=-1, pan=-0.15, reverb=0.2, pitch_hz=False))
    song.add(Part(rims, ins.rim, gain_db=-5, pan=-0.2, reverb=0.25, pitch_hz=False))
    song.add(Part(shakes, ins.shaker, gain_db=-7, pan=0.45, reverb=0.15, pitch_hz=False,
                            humanize=0.003))
    rng = np.random.default_rng(1102)
    return song.mixdown(reverb_ir(1.7, rng), pingpong_ir(0.75 * song.spb, 0.35))


def dungeon():
    """Cellar and archive levels. E dorian, 88 BPM, 24 bars (A B A').

    A: a kalimba ostinato (rising B-E-F#-G, falling answer) over a dark pad on an E pedal, the
    dorian IV (A major, its C# the 'mysterious' colour) alternating with Em9. B: the harp sings a
    slow tune over Cmaj7-D-Em-A; the A sections end on B7 for a pull back home, the first one
    resolving deceptively to Cmaj7. Water drips and a soft heartbeat drum are the only percussion."""
    song = Song(88, 24, seed=2201)

    prog_a = "Em9:4 | Em9:4 | A/E:4 | A/E:4 | Em9:4 | Em9:4 | Cmaj7:4 | Bsus4:2 B7:2"
    prog_b = "Cmaj7:4 | D:4 | Em:4 | A:4 | Cmaj7:4 | D:4 | Bsus4:4 | B:4"
    prog = progression(prog_a, 0) + progression(prog_b, 8) + progression(prog_a, 16)

    ostinato = ("r:.5 B4:.5 E5:.5 F#5:.5 G5:1 F#5:.5 B4:.5 | r:.5 B4:.5 E5:.5 F#5:.5 D5:1.5 r:.5 "
                "| r:.5 C#5:.5 E5:.5 F#5:.5 A5:1 F#5:.5 C#5:.5 | r:.5 C#5:.5 E5:.5 F#5:.5 E5:1.5 r:.5 "
                "| r:.5 B4:.5 E5:.5 F#5:.5 G5:1 F#5:.5 B4:.5 | r:.5 B4:.5 E5:.5 F#5:.5 G5:.5 A5:.5 B5:1 "
                "| r:.5 G4:.5 B4:.5 E5:.5 F#5:1 E5:.5 B4:.5 | r:.5 F#4:.5 B4:.5 E5:.5 D#5:1 F#4:.5 A4:.5")
    answers_b = "r:4 | r:4 | r:4 | r:2 E5:.5 F#5:.5 A5:1 | r:4 | r:4 | r:4 | r:2 F#5:.5 A5:.5 B5:1"
    kalimba = phrased(bars(ostinato, 0, vel=0.7) + bars(answers_b, 8, vel=0.6) + bars(ostinato, 16, vel=0.75), depth=0.1)

    tune_b = ("E5:1.5 D5:.5 B4:2 | A4:1 B4:.5 D5:.5 F#5:2 | G5:1.5 F#5:.5 E5:1 B4:1 | C#5:3 r:1 "
              "| E5:1.5 G5:.5 F#5:1 E5:1 | D5:1 E5:.5 F#5:.5 A5:2 | F#5:1.5 E5:.5 B4:2 | D#5:3 r:1")
    counter = ("G4:2 F#4:2 | E4:4 | E4:2 C#4:2 | E4:4 | G4:2 F#4:2 | B4:4 | G4:2 E4:2 | F#4:2 D#4:2")
    harp_notes = phrased(bars(tune_b, 8, vel=0.72)) + bars(counter, 16, vel=0.45)

    pad_v = voicings({"Em9": "G3 B3 D4 F#4", "A/E": "A3 C#4 E4 F#4", "Cmaj7": "G3 C4 E4 B4",
                      "Bsus4": "F#3 B3 E4", "B7": "F#3 A3 D#4", "D": "F#3 A3 D4 E4", "Em": "G3 B3 E4",
                      "A": "A3 C#4 E4", "B": "F#3 B3 D#4"})
    bass_a = "E2:4 | E2:4 | E2:4 | E2:4 | E2:4 | E2:4 | C2:4 | B1:4"
    bass_b = "C2:4 | D2:4 | E2:4 | A1:4 | C2:4 | D2:4 | B1:4 | B1:4"
    bassline = bars(bass_a, 0, vel=0.7) + bars(bass_b, 8, vel=0.7) + bars(bass_a, 16, vel=0.7)

    heart = (hits([0], range(0, 8), 0.55) + hits([0, 2.5], range(8, 16), 0.5, accents={2.5: 0.7})
             + hits([0], range(16, 24), 0.55) + hits([2.5], range(17, 24, 2), 0.35))
    shakes = hits([1.5, 3.5], range(8, 24), 0.3)
    drip_beats = [1.75, 6.25, 11.5, 15.0, 19.25, 21.5, 26.75, 30.5, 37.25, 43.5, 50.75, 58.25, 63.5, 70.0,
                  77.25, 83.5, 89.75]
    drips = [Note(b, 0.25, 0, 0.6) for b in drip_beats]

    def drip(vel, rng):
        return ins.drip(vel, rng, freq=rng.uniform(850, 1500))

    def dark_pad(f, d, v, rng):
        return ins.pad(f, d, v, rng, cutoff=750, attack=1.5, release=2.0, detune=7, vibrato=0.0015)

    def bass(f, d, v, rng):
        return ins.bass(f, d, v, rng, pluck=0.12, bright=0.9)

    song.add(Part(kalimba, ins.kalimba, gain_db=-1, pan=0.15, reverb=0.4, delay=0.25))
    song.add(Part(harp_notes, ins.harp, gain_db=2, pan=-0.25, reverb=0.4))
    song.add(Part(pad_chords(prog, pad_v), dark_pad, gain_db=-12, reverb=0.5, humanize=0, vel_jitter=0))
    song.add(Part(bassline, bass, gain_db=-7.5, reverb=0.1, humanize=0.002))
    song.add(Part(heart, ins.frame_drum, gain_db=-7, reverb=0.25, pitch_hz=False, humanize=0.003))
    song.add(Part(shakes, ins.shaker, gain_db=-4, pan=0.4, reverb=0.3, pitch_hz=False))
    song.add(Part(drips, drip, gain_db=-10, pan=-0.4, reverb=0.9, pitch_hz=False, humanize=0,
                           vel_jitter=0.25))
    rng = np.random.default_rng(2203)
    return song.mixdown(reverb_ir(2.6, rng, predelay=0.02), pingpong_ir(0.75 * song.spb, 0.3))


def keep():
    """Powder store, great hall and tower levels. A minor (aeolian, with a dorian D major and a
    relative-major B section for brighter moments), 98 BPM, 28 bars (intro A B A').

    A steady cello-pizzicato ostinato and brushed kit give a focused pulse; the marimba tune
    answers itself in two-bar phrases, rises to the relative major in B, and in A' is joined by a
    softer marimba harmony line. The intro bars double as the breather when the loop restarts."""
    song = Song(98, 28, seed=3301)

    prog = (progression("Am:4 | Am:4 | F:4 | E:4", 0)
            + progression("Am:4 | F:4 | C:4 | G:4 | Am:4 | F:4 | Dm:4 | E:4", 4)
            + progression("C:4 | G:4 | Am:4 | F:4 | C:4 | G:4 | F:4 | E:4", 12)
            + progression("Am:4 | F:4 | C:4 | G:4 | Am:4 | D:4 | F:4 | E:4", 20))
    pizz_v = voicings({"Am": "A2 E3 A3 C4", "F": "F2 C3 F3 A3", "C": "C3 E3 G3 C4", "G": "G2 D3 G3 B3",
                       "E": "E2 B2 E3 G#3", "Dm": "D3 F3 A3 D4", "D": "D3 F#3 A3 D4"})
    ostinato = arpeggio(prog, pizz_v, [0, 2, 1, 2, 3, 2, 1, 2], vel=0.6, accent=1.2)

    head = ("A4:1 C5:.5 B4:.5 A4:.5 E4:.5 A4:1 | C5:1 D5:.5 C5:.5 A4:1 F4:1 | E5:1.5 D5:.5 C5:.5 D5:.5 E5:1 "
            "| D5:2 B4:1 G4:1 | A4:1 C5:.5 B4:.5 A4:.5 E4:.5 A4:1 ")
    tune_a1 = head + "| C5:1 D5:.5 E5:.5 F5:1 E5:.5 D5:.5 | D5:1 F5:.5 E5:.5 D5:1 C5:1 | B4:1.5 G#4:.5 E4:2"
    tune_a2 = head + "| D5:1 E5:.5 F#5:.5 A5:1 F#5:1 | F5:1 E5:.5 D5:.5 C5:1 A4:1 | B4:1.5 G#4:.5 E4:2"
    tune_b = ("E5:.5 G5:.5 E5:.5 D5:.5 C5:1 G4:1 | B4:.5 D5:.5 G5:1.5 F5:.5 D5:1 | E5:1 C5:.5 E5:.5 A5:1.5 G5:.5 "
              "| F5:1 E5:.5 D5:.5 C5:2 | E5:.5 G5:.5 E5:.5 D5:.5 C5:1 G4:1 | B4:.5 D5:.5 G5:1 A5:.5 B5:.5 G5:1 "
              "| A5:1.5 G5:.5 F5:1 E5:1 | E5:1.5 D5:.5 B4:1 G#4:1")
    marimba = phrased(bars(tune_a1, 4, vel=0.75) + bars(tune_b, 12, vel=0.8) + bars(tune_a2, 20, vel=0.78))
    harmony = bars("E4:2 C4:2 | A4:2 F4:2 | G4:2 E4:2 | G4:2 D4:2 | E4:2 C4:2 | F#4:2 A4:2 | A4:2 F4:2 "
                   "| D4:2 B3:2", 20, vel=0.5)
    bells = [n for n in shift(bars(tune_b, 12, vel=0.4), semitones=12) if n.beat >= 16 * 4]

    roots = {"Am": ("A1", "E2"), "F": ("F1", "C2"), "C": ("C2", "G2"), "G": ("G1", "D2"), "E": ("E1", "B1"),
             "Dm": ("D2", "A2"), "D": ("D2", "A2")}
    bassline = []
    for beat, _length, name in prog:
        root, fifth = roots[name]
        bassline += shift(bars(f"{root}:2 {fifth}:1.5 {root}:.5"), beats=beat)

    kicks = hits([0, 2], range(0, 12), 0.7) + hits([0, 2.5], range(12, 20), 0.65) + hits([0, 2], range(20, 28), 0.75)
    brushes = hits([1, 3], range(4, 28), 0.7) + hits([3.75], range(13, 20, 2), 0.3)
    rims = hits([1.5], range(12, 20), 0.5)
    shakes = hits(EIGHTHS, range(0, 28), 0.45, accents={k: 1.5 for k in OFFBEATS})

    def strings_pad(f, d, v, rng):
        return ins.pad(f, d, v, rng, cutoff=1300, attack=0.8, release=1.0, detune=6)

    pad_v = voicings({"Am": "A3 C4 E4", "F": "A3 C4 F4", "C": "G3 C4 E4", "G": "G3 B3 D4", "E": "G#3 B3 E4",
                      "Dm": "A3 D4 F4", "D": "A3 D4 F#4"})

    song.add(Part(ostinato, ins.pizz, gain_db=-2, pan=0.3, reverb=0.2, humanize=0.004))
    song.add(Part(marimba, ins.marimba, gain_db=0, pan=-0.12, reverb=0.25))
    song.add(Part(harmony, ins.marimba, gain_db=-4, pan=-0.3, reverb=0.25))
    song.add(Part(bells, ins.glock, gain_db=-10, pan=0.25, reverb=0.3, delay=0.3))
    song.add(Part(pad_chords(prog[4:], pad_v), strings_pad, gain_db=-17, reverb=0.35, humanize=0,
                         vel_jitter=0))
    song.add(Part(bassline, ins.bass, gain_db=-8, reverb=0.05, humanize=0.003))
    song.add(Part(kicks, ins.kick, gain_db=-7, reverb=0.05, pitch_hz=False, humanize=0.002))
    song.add(Part(brushes, ins.brush, gain_db=2, pan=-0.15, reverb=0.2, pitch_hz=False))
    song.add(Part(rims, ins.rim, gain_db=-1, pan=0.2, reverb=0.2, pitch_hz=False))
    song.add(Part(shakes, ins.shaker, gain_db=-5, pan=0.45, reverb=0.15, pitch_hz=False,
                            humanize=0.003))
    rng = np.random.default_rng(3302)
    return song.mixdown(reverb_ir(1.5, rng), pingpong_ir(0.75 * song.spb, 0.3))


def lab():
    """The alchemist's laboratory. C minor with chromatic touches (Neapolitan Db, B natural,
    a C-B-Bb-A line cliche), 100 BPM, 28 bars (A B A' coda).

    Clockwork: a tick-tock on every eighth and an oom-pah synth bass keep strict time while a
    three-note synth arpeggio cycles against the four-beat bar. A slightly mistuned music box
    carries the tune, FM 'glassware' chimes ring at phrase starts, and in A' an eerie
    theremin-like voice scoops in underneath."""
    song = Song(100, 28, seed=4401)

    prog_a = "Cm:4 | Cm:4 | Ab:4 | Ab:4 | Fm:4 | Db:4 | G7:4 | G7:4"
    prog_b = "Cm:4 | CmM7:4 | Cm7:4 | Cm6:4 | Abmaj7:4 | Fm7:4 | Dm7b5:4 | G7:4"
    prog = (progression(prog_a, 0) + progression(prog_b, 8) + progression(prog_a, 16)
            + progression("Cm:4 | Db:4 | Cm:4 | G7:4", 24))
    arp_v = voicings({"Cm": "C4 Eb4 G4", "Ab": "C4 Eb4 Ab4", "Fm": "C4 F4 Ab4", "Db": "Db4 F4 Ab4",
                      "G7": "B3 D4 F4", "CmM7": "B3 Eb4 G4", "Cm7": "Bb3 Eb4 G4", "Cm6": "A3 Eb4 G4",
                      "Abmaj7": "C4 Eb4 G4", "Fm7": "C4 Eb4 Ab4", "Dm7b5": "C4 F4 Ab4"})
    arp = (arpeggio(prog[:16], arp_v, [0, 1, 2], vel=0.55, accent=1.25)
           + arpeggio(prog[16:24], arp_v, [0, 1, 2, 1], step=0.25, vel=0.45, accent=1.3)
           + arpeggio(prog[24:], arp_v, [0, 1, 2], vel=0.55, accent=1.25))

    tune_a = ("G5:1 C6:1 Eb6:1 D6:1 | C6:.5 B5:.5 C6:1 G5:2 | Ab5:1 C6:1 Eb6:1 Db6:1 | C6:1 Bb5:1 Ab5:1 G5:1 "
              "| F5:1 Ab5:1 C6:1 Db6:1 | C6:1 Ab5:1 F5:1 Db5:1 | B4:1 D5:1 F5:1 Ab5:1 | Ab5:.5 G5:.5 F#5:.5 G5:.5 r:2")
    tune_b = ("Eb6:2 D6:1 C6:1 | B5:3 r:1 | Bb5:2 G5:1 Bb5:1 | A5:3 r:1 | G5:1 C6:1 Eb6:1 G6:1 | F6:2 Eb6:1 C6:1 "
              "| Ab5:1 F5:1 D5:1 F5:1 | D6:1 B5:1 Ab5:1 F5:1")
    tune_coda = "G5:1 C6:1 Eb6:1 D6:1 | Db6:2 C6:2 | r:4 | r:2 B5:1 D6:1"
    box = phrased(bars(tune_a, 0, vel=0.7) + bars(tune_b, 8, vel=0.65) + bars(tune_a, 16, vel=0.75)
                  + bars(tune_coda, 24, vel=0.6), depth=0.08)
    eerie = bars("Eb5:4 | D5:2 C5:2 | C5:4 | Bb4:2 C5:2 | Ab4:4 | F4:2 Ab4:2 | G4:4 | B4:2 D5:2", 16, vel=0.6)
    glass = bars("G6:4 | r:4 | Bb6:4 | r:4 | G6:4 | r:4 | Ab6:4 | r:4 | Eb6:4 | r:4 | C7:4 | r:4 | Ab6:4 | r:4 "
                 "| B6:4 | r:4", 8, vel=0.5)

    roots = {"Cm": ("C2", "G2"), "Ab": ("Ab1", "Eb2"), "Fm": ("F1", "C2"), "Db": ("Db2", "Ab2"),
             "G7": ("G1", "D2"), "CmM7": ("C2", "G2"), "Cm7": ("C2", "G2"), "Cm6": ("C2", "G2"),
             "Abmaj7": ("Ab1", "Eb2"), "Fm7": ("F1", "C2"), "Dm7b5": ("D2", "Ab2")}
    bassline = []
    for beat, _length, name in prog:
        root, fifth = roots[name]
        bassline += shift(bars(f"{root}:.5 r:.5 {fifth}:.5 r:.5 {root}:.5 r:.5 {fifth}:.5 r:.5", vel=0.7),
                          beats=beat)

    ticks = hits([0, 1, 2, 3], range(0, 28), 0.5, accents={0: 1.3, 2: 1.15})
    tocks = hits(OFFBEATS, range(0, 28), 0.5)
    kicks = hits([0, 2], range(0, 8), 0.6) + hits([0], range(8, 16), 0.55) + hits([0, 2], range(16, 24), 0.65)
    clanks = hits([0], range(0, 28, 4), 0.5)
    ratchets = hits([3], [7, 15, 23, 27], 0.6)

    def tick(vel, rng):
        return ins.tick(vel, rng, freq=2300.0)

    def tock(vel, rng):
        return ins.tick(vel, rng, freq=1650.0)

    def eerie_voice(f, d, v, rng):
        return ins.theremin(f, d, v, rng, glide_from=f * 2 ** (-1 / 12))

    def eerie_pad(f, d, v, rng):
        return ins.pad(f, d, v, rng, cutoff=900, attack=1.0, release=1.2, detune=10, vibrato=0.003)

    def glassware(f, d, v, rng):
        return ins.fm_bell(f, d, v, rng, ratio=1.41, index=1.4, t60=2.2)

    def arp_pluck(f, d, v, rng):
        return ins.synth_pluck(f, d, v, rng, wave="square", cutoff=(2600.0, 500.0), decay=0.16)

    pad_v = voicings({"Cm": "G3 C4 Eb4", "Ab": "Ab3 C4 Eb4", "Fm": "Ab3 C4 F4", "Db": "Ab3 Db4 F4",
                      "G7": "G3 B3 F4", "CmM7": "G3 B3 Eb4", "Cm7": "G3 Bb3 Eb4", "Cm6": "G3 A3 Eb4",
                      "Abmaj7": "G3 C4 Eb4", "Fm7": "Ab3 C4 Eb4", "Dm7b5": "Ab3 C4 F4"})

    song.add(Part(box, ins.music_box, gain_db=0, pan=0.1, reverb=0.35, delay=0.3, humanize=0.0,
                               vel_jitter=0.04))
    song.add(Part(arp, arp_pluck, gain_db=-8, pan=-0.35, reverb=0.25, delay=0.15, humanize=0.0))
    song.add(Part(eerie, eerie_voice, gain_db=-9, pan=-0.15, reverb=0.45, humanize=0.0))
    song.add(Part(glass, glassware, gain_db=-12, pan=0.4, reverb=0.55, delay=0.2, humanize=0.0))
    song.add(Part(pad_chords(prog, pad_v), eerie_pad, gain_db=-17, reverb=0.4, humanize=0, vel_jitter=0))
    song.add(Part(bassline, ins.synth_bass, gain_db=-8, reverb=0.05, humanize=0.0))
    song.add(Part(ticks, tick, gain_db=0, pan=0.35, reverb=0.15, pitch_hz=False, humanize=0.0))
    song.add(Part(tocks, tock, gain_db=-1, pan=0.25, reverb=0.15, pitch_hz=False, humanize=0.0))
    song.add(Part(kicks, ins.kick, gain_db=-7, reverb=0.05, pitch_hz=False, humanize=0.0))
    song.add(Part(clanks, ins.clank, gain_db=-10, pan=-0.3, reverb=0.4, pitch_hz=False, humanize=0.0))
    song.add(Part(ratchets, ins.ratchet, gain_db=-8, pan=0.3, reverb=0.3, pitch_hz=False, humanize=0.0))
    rng = np.random.default_rng(4402)
    return song.mixdown(reverb_ir(1.6, rng), pingpong_ir(0.75 * song.spb, 0.35))


SONGS = {"title_map": title_map, "dungeon": dungeon, "keep": keep, "lab": lab}
