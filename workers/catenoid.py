#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges

C = 0.35
U_MAX = C * math.acosh(1.0 / C)


def catenoid(u, v, c=C):
    r = c * math.cosh(u / c)
    return (r * math.cos(v), r * math.sin(v), u)


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(8):
        v = 2 * math.pi * k / 8
        pts = [catenoid(-U_MAX + 2.0 * U_MAX * i / (n - 1), v) for i in range(n)]
        segs += edges.polyline(pts)
        u = -U_MAX + 2.0 * U_MAX * k / 7
        pts = [catenoid(u, 2 * math.pi * i / (n - 1)) for i in range(n)]
        segs += edges.polyline(pts)
    edges.write(out, segs)


if __name__ == "__main__":
    main()
