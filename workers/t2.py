#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges

R, r = 0.8, 0.28


def torus(u, v):
    return (
        (R + r * math.cos(v)) * math.cos(u),
        r * math.sin(v),
        (R + r * math.cos(v)) * math.sin(u),
    )


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(8):
        u = 2 * math.pi * k / 8
        pts = [torus(u, 2 * math.pi * i / (n - 1)) for i in range(n)]
        segs += edges.polyline(pts)
        v = 2 * math.pi * k / 8
        pts = [torus(2 * math.pi * i / (n - 1), v) for i in range(n)]
        segs += edges.polyline(pts)
    edges.write(out, segs)


if __name__ == "__main__":
    main()
