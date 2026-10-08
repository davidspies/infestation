"""Ogg Vorbis and MP3 encoding/decoding through ffmpeg, plus a check for a lewton quirk that
affects Ogg loops.

MP3s keep their Xing/LAME header, whose encoder delay and padding fields let gapless decoders
(ffmpeg, and through it Chrome's decodeAudioData) trim the codec's priming and padding exactly.

quad-snd (macroquad's native audio) decodes Ogg Vorbis with lewton 0.9. lewton returns decoded
samples per packet on a different schedule than the granule positions count them: a long block
followed by a short one yields 448 samples early, which the next (short) packet gives back. Its
end-of-stream trim is computed from the last page's starting granule plus its own per-packet counts,
so when the second-to-last Ogg page ends exactly between such a pair of packets the trim comes out
448 samples too lenient and the decoded file keeps some encoder padding - a click at a loop seam.
`lewton_length` replays lewton's bookkeeping so the encoder can pick an Ogg paging that avoids it.
"""

import struct
import subprocess

import numpy as np

from dsp import SR


def _ffmpeg_encode(path, x, codec_args):
    channels = 1 if x.ndim == 1 else x.shape[1]
    cmd = ["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-f", "f32le", "-ar", str(SR), "-ac", str(channels),
           "-i", "pipe:0", *codec_args, "-fflags", "+bitexact", "-flags:a", "+bitexact", "-map_metadata", "-1",
           str(path)]
    subprocess.run(cmd, input=np.ascontiguousarray(x, dtype="<f4").tobytes(), check=True)


def encode_vorbis(path, x, quality, page_duration_us=1_000_000):
    """Ogg Vorbis at the given -q:a quality."""
    _ffmpeg_encode(path, x, ["-c:a", "libvorbis", "-q:a", str(quality), "-page_duration", str(page_duration_us)])


def encode_mp3(path, x, rate_args):
    """MP3 (LAME) with its Xing/LAME gapless header and no ID3 tag; rate_args e.g. ["-b:a", "96k"]."""
    _ffmpeg_encode(path, x, ["-c:a", "libmp3lame", *rate_args, "-write_xing", "1", "-id3v2_version", "0"])
    _set_lame_padding(path, len(x))


def _crc16_arc(data):
    crc = 0
    for byte in data:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if crc & 1 else crc >> 1
    return crc


def _set_lame_padding(path, n_samples):
    """ffmpeg's libmp3lame wrapper under-reports the LAME tag's end padding by 47 - n % 1152 samples
    whenever n % 1152 is 1..46, so gapless decoders would keep that much encoder padding. Write the
    exact padding (frames * 1152 - delay - n) and re-sign the tag the way ffmpeg does: CRC-16/ARC
    of the Info frame's first 190 bytes with the CRC field zeroed."""
    data = bytearray(path.read_bytes())
    if data[0] != 0xFF or data[1] & 0xE0 != 0xE0:
        raise ValueError(f"{path}: expected the Xing/Info frame at the start of the file")
    tag = max(data.find(b"Info", 0, 64), data.find(b"Xing", 0, 64))
    flags = struct.unpack_from(">I", data, tag + 4)[0]
    if not flags & 1:
        raise ValueError(f"{path}: Xing/Info header without a frame count")
    frames = struct.unpack_from(">I", data, tag + 8)[0]
    lame = tag + 8 + 4 * bool(flags & 1) + 4 * bool(flags & 2) + 100 * bool(flags & 4) + 4 * bool(flags & 8)
    delay = (data[lame + 21] << 4) | (data[lame + 22] >> 4)
    padding = frames * 1152 - delay - n_samples
    if not 0 <= padding < 4096:
        raise ValueError(f"{path}: LAME padding {padding} out of range")
    data[lame + 21 : lame + 24] = ((delay << 12) | padding).to_bytes(3, "big")
    data[lame + 34 : lame + 36] = b"\0\0"
    data[lame + 34 : lame + 36] = _crc16_arc(data[:190]).to_bytes(2, "big")
    path.write_bytes(bytes(data))


def decode(path, channels):
    cmd = ["ffmpeg", "-nostdin", "-loglevel", "error", "-i", str(path), "-f", "f32le", "-ac", str(channels), "pipe:1"]
    raw = subprocess.run(cmd, capture_output=True, check=True).stdout
    data = np.frombuffer(raw, dtype="<f4").astype(float)
    return data if channels == 1 else data.reshape(-1, channels)


def ogg_packets(data):
    """[(packet bytes, granule of the page it ends on, is last packet ending on that page, page is EOS)]"""
    packets, partial, pos = [], b"", 0
    while pos < len(data):
        if data[pos : pos + 4] != b"OggS":
            raise ValueError(f"bad Ogg page at byte {pos}")
        flags, granule = data[pos + 5], struct.unpack_from("<q", data, pos + 6)[0]
        lacing = data[pos + 27 : pos + 27 + data[pos + 26]]
        off = pos + 27 + len(lacing)
        done = []
        for size in lacing:
            partial += data[off : off + size]
            off += size
            if size < 255:
                done.append(partial)
                partial = b""
        packets += [(p, granule, i == len(done) - 1, bool(flags & 4)) for i, p in enumerate(done)]
        pos = off
    return packets


def lewton_length(path):
    """(samples lewton 0.9 will decode, samples the stream's granule positions say it holds)."""
    packets = ogg_packets(path.read_bytes())
    ident = packets[0][0]
    bs0, bs1 = 1 << (ident[28] & 15), 1 << (ident[28] >> 4)
    audio = packets[3:]
    # libvorbis streams have two modes: 0 = short block, 1 = long block (validated below)
    first = [p[0][0] for p in audio]
    if any(b & 1 for b in first):
        raise ValueError("unexpected header packet among audio packets")
    size = [bs1 if (b >> 1) & 1 else bs0 for b in first]
    spec = [0] + [size[i - 1] // 4 + size[i] // 4 for i in range(1, len(size))]
    position = np.cumsum(spec)
    for i, (_p, granule, last_in_page, eos) in enumerate(audio):
        if last_in_page and not eos and position[i] != granule:
            raise ValueError("granule positions do not match the assumed short/long block modes")

    def returned(i):
        if i == 0:
            return 0
        n, b = size[i], first[i]
        if n == bs0:
            return n // 2
        left = 0 if (b >> 2) & 1 else (n - bs0) // 4
        right = n // 2 if (b >> 3) & 1 else (3 * n - bs0) // 4
        return right - left

    tracked, total = None, 0
    for i, (_p, granule, last_in_page, eos) in enumerate(audio):
        count = returned(i)
        if tracked is not None and last_in_page and eos:
            count = min(count, max(granule - tracked, 0))
        if last_in_page:
            tracked = granule
        elif tracked is not None:
            tracked += count
        total += count
    return total, audio[-1][1]
