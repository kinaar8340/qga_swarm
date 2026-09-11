#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges


def hemi(theta, phi):
    return (
        math.sin(theta) * math.cos(phi),
        math.cos(theta),
        math.sin(theta) * math.sin(phi),
    )


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(6):
        phi = k * math.pi / 6
        pts = [hemi(math.pi * 0.5 * i / (n - 1), phi) for i in range(n)]
        segs += edges.polyline(pts)
    pts = [hemi(math.pi / 2, 2 * math.pi * i / (n - 1)) for i in range(n)]
    segs += edges.polyline(pts)
    # identification chords: equator point → opposite
    for k in range(6):
        phi = k * math.pi / 6
        a = hemi(math.pi / 2, phi)
        b = hemi(math.pi / 2, phi + math.pi)
        segs.append((a, (0.0, 0.0, 0.0)))
        segs.append(((0.0, 0.0, 0.0), b))
    edges.write(out, segs)


if __name__ == "__main__":
    main()
