#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges


def klein(u, v):
    # figure-8 immersion, scaled into a unit-ish box
    cu, su = math.cos(u), math.sin(u)
    cv, sv = math.cos(v), math.sin(v)
    x = (2 + cv * su - sv * su * cu) * cu * 0.22
    y = (2 + cv * su - sv * su * cu) * su * 0.22
    z = sv * su + cv * cu * 0.22
    return (x, z, y)


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    out = p.parse_args().out
    n = edges.samples()
    segs = []
    for k in range(8):
        u = 2 * math.pi * k / 8
        pts = [klein(u, 2 * math.pi * i / (n - 1)) for i in range(n)]
        segs += edges.polyline(pts)
        v = 2 * math.pi * k / 8
        pts = [klein(2 * math.pi * i / (n - 1), v) for i in range(n)]
        segs += edges.polyline(pts)
    edges.write(out, segs)


if __name__ == "__main__":
    main()
