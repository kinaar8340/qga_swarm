#!/usr/bin/env python3
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edges

C = 0.35
U_MAX = C * math.acosh(1.0 / C)


def associate(u, v, theta, c=C):
    ct, st = math.cos(theta), math.sin(theta)
    ch, sh = math.cosh(u / c), math.sinh(u / c)
    return (
        ct * c * ch * math.cos(v) + st * c * sh * math.cos(v),
        ct * c * ch * math.sin(v) + st * c * sh * math.sin(v),
        ct * u + st * c * v,
    )


def main():
    p = __import__("argparse").ArgumentParser()
    edges.add_arg(p)
    p.add_argument("--theta", type=float, required=True, help="phase in [0,1] → θ=phase*π/2")
    args = p.parse_args()
    if not 0.0 <= args.theta <= 1.0:
        raise SystemExit("theta phase must be in [0,1]")
    theta = args.theta * math.pi / 2.0
    n = edges.samples()
    segs = []
    for k in range(8):
        v = -math.pi + 2 * math.pi * k / 8
        pts = [associate(-U_MAX + 2.0 * U_MAX * i / (n - 1), v, theta) for i in range(n)]
        segs += edges.polyline(pts)
        u = -U_MAX + 2.0 * U_MAX * k / 7
        pts = [associate(u, -math.pi + 2 * math.pi * i / (n - 1), theta) for i in range(n)]
        segs += edges.polyline(pts)
    edges.write(args.out, segs)


if __name__ == "__main__":
    main()
