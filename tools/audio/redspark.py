"""RedSpark (MadWorld se/*.rsp members) header decryption, after vgmstream src/meta/redspark.c:
key = word0 ^ 'RedS'; word0 ^= key; key = rotl(key, 11); then per word: key = rotl(key, 3) + key; word ^= key."""
import struct

M = 0xFFFFFFFF


def rotl(x, n): return ((x << n) | (x >> (32 - n))) & M


def decrypt(buf, size=None):
    size = len(buf) if size is None else min(size, len(buf))
    size -= size % 4
    out = bytearray(buf)
    key = struct.unpack_from('>I', buf, 0)[0] ^ 0x52656453
    struct.pack_into('>I', out, 0, struct.unpack_from('>I', buf, 0)[0] ^ key)
    key = rotl(key, 11)
    for i in range(4, size, 4):
        key = (rotl(key, 3) + key) & M
        struct.pack_into('>I', out, i, struct.unpack_from('>I', buf, i)[0] ^ key)
    return bytes(out)


def pack_members(data):
    n = struct.unpack_from('>I', data, 4)[0]
    return [struct.unpack_from('>II', data, 8 + i * 8) for i in range(n)]
