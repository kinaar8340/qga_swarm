#!/usr/bin/env python3
"""One-sheet hyperboloid rulings. Model cage, not a third minimal surface."""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges

A = 0.70
C = 0.55
T_MAX = 1.20


def family1(theta, t, a=A, c=C):
    return (
        a * (math.cos(theta) - t * math.sin(theta)),
        a * (math.sin(theta) + t * math.cos(theta)),
        c * t,
    )


def family2(theta, t, a=A, c=C):
    return (
        a * (math.cos(theta) + t * math.sin(theta)),
        a * (math.sin(theta) - t * math.cos(theta)),
        c * t,
    )


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(8):
        th = 2 * math.pi * k / 8
        pts1 = [family1(th, -T_MAX + 2.0 * T_MAX * i / (n - 1)) for i in range(n)]
        pts2 = [family2(th, -T_MAX + 2.0 * T_MAX * i / (n - 1)) for i in range(n)]
        segs += edges.polyline(pts1)
        segs += edges.polyline(pts2)
    edges.write(out, segs)


if __name__ == "__main__":
    main()
