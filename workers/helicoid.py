#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges

C = 0.35


def helicoid(u, v, c=C):
    return (u * math.cos(v), u * math.sin(v), c * v)


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(8):
        v = -math.pi + 2 * math.pi * k / 8
        pts = [helicoid(-1.0 + 2.0 * i / (n - 1), v) for i in range(n)]
        segs += edges.polyline(pts)
    for u in (-1.0, -1.0 / 3.0, 1.0 / 3.0, 1.0):
        pts = [helicoid(u, -math.pi + 2 * math.pi * i / (n - 1)) for i in range(n)]
        segs += edges.polyline(pts)
    edges.write(out, segs)


if __name__ == "__main__":
    main()
