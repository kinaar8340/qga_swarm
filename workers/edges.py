#!/usr/bin/env python3
import argparse
import struct
from pathlib import Path

MAGIC = b"QGAE"
VERSION = 1


def pack(edges):
    buf = bytearray(MAGIC)
    buf += struct.pack("<I", VERSION)
    buf += struct.pack("<I", len(edges))
    for a, b in edges:
        buf += struct.pack("<6f", a[0], a[1], a[2], b[0], b[1], b[2])
    return bytes(buf)


def write(path, edges):
    Path(path).write_bytes(pack(edges))


def polyline(points):
    return [(points[i], points[i + 1]) for i in range(len(points) - 1)]


def samples(n=64):
    return n


def add_arg(p):
    p.add_argument("--out", required=True)
