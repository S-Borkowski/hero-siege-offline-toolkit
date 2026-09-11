"""Minisign key generation, signing and verification, in pure standard library.

The hub verifies `catalog.json` with the Rust `minisign-verify` crate, so the
signatures this module writes have to be byte-compatible with minisign proper --
not with a scheme of our own. The format is:

    untrusted comment: <text>
    base64( sig_alg[2] | key_id[8] | signature[64] )
    trusted comment: <text>
    base64( ed25519(sk, signature[64] || trusted_comment) )

`sig_alg` is `ED`, the prehashed variant: the signed message is
BLAKE2b-512(file) rather than the file itself. minisign has used that by default
since 0.6 and `minisign-verify` accepts it.

Why an Ed25519 implementation lives here rather than a dependency: every other
build tool in `tools/` is standard-library-only and runs under a bare `py -3`, and
CI signs the catalog on a runner that would otherwise need a pip step before it
could produce a release artifact. Ed25519 from RFC 8032 is about sixty lines and
signs one small file per run, so the cost of writing it is lower than the cost of
requiring `cryptography` everywhere the catalog is built. It is *not* constant
time; nothing here handles a key on a machine an attacker can measure.
"""

from __future__ import annotations

import base64
import hashlib
import os
import secrets
from pathlib import Path
from typing import Optional, Tuple

# --------------------------------------------------------------------------
# Ed25519 (RFC 8032), reference formulation over extended coordinates.
# --------------------------------------------------------------------------

_P = 2 ** 255 - 19
_L = 2 ** 252 + 27742317777372353535851937790883648493
_D = -121665 * pow(121666, _P - 2, _P) % _P
_SQRT_M1 = pow(2, (_P - 1) // 4, _P)

_Point = Tuple[int, int, int, int]  # X, Y, Z, T with x = X/Z, y = Y/Z


def _sha512(data: bytes) -> bytes:
    return hashlib.sha512(data).digest()


def _inv(x: int) -> int:
    return pow(x, _P - 2, _P)


def _point_add(p: _Point, q: _Point) -> _Point:
    x1, y1, z1, t1 = p
    x2, y2, z2, t2 = q
    a = (y1 - x1) * (y2 - x2) % _P
    b = (y1 + x1) * (y2 + x2) % _P
    c = 2 * t1 * t2 * _D % _P
    d = 2 * z1 * z2 % _P
    e, f, g, h = b - a, d - c, d + c, b + a
    return (e * f % _P, g * h % _P, f * g % _P, e * h % _P)


def _scalar_mult(p: _Point, e: int) -> _Point:
    q: _Point = (0, 1, 1, 0)  # neutral element
    while e > 0:
        if e & 1:
            q = _point_add(q, p)
        p = _point_add(p, p)
        e >>= 1
    return q


def _recover_x(y: int, sign: int) -> Optional[int]:
    if y >= _P:
        return None
    x2 = (y * y - 1) * _inv(_D * y * y + 1) % _P
    if x2 == 0:
        return None if sign else 0
    x = pow(x2, (_P + 3) // 8, _P)
    if (x * x - x2) % _P != 0:
        x = x * _SQRT_M1 % _P
    if (x * x - x2) % _P != 0:
        return None
    if (x & 1) != sign:
        x = _P - x
    return x


_BASE_Y = 4 * _inv(5) % _P
_BASE_X = _recover_x(_BASE_Y, 0)
assert _BASE_X is not None
_BASE: _Point = (_BASE_X, _BASE_Y, 1, _BASE_X * _BASE_Y % _P)


def _compress(p: _Point) -> bytes:
    x, y, z, _ = p
    zinv = _inv(z)
    x = x * zinv % _P
    y = y * zinv % _P
    return int.to_bytes(y | ((x & 1) << 255), 32, "little")


def _decompress(data: bytes) -> Optional[_Point]:
    if len(data) != 32:
        return None
    y = int.from_bytes(data, "little")
    sign = y >> 255
    y &= (1 << 255) - 1
    x = _recover_x(y, sign)
    if x is None:
        return None
    return (x, y, 1, x * y % _P)


def _points_equal(p: _Point, q: _Point) -> bool:
    if (p[0] * q[2] - q[0] * p[2]) % _P:
        return False
    return (p[1] * q[2] - q[1] * p[2]) % _P == 0


def _expand_seed(seed: bytes) -> Tuple[int, bytes]:
    h = _sha512(seed)
    a = int.from_bytes(h[:32], "little")
    a &= (1 << 254) - 8
    a |= 1 << 254
    return a, h[32:]


def ed25519_public_key(seed: bytes) -> bytes:
    """Derive the 32-byte public key from a 32-byte seed."""
    if len(seed) != 32:
        raise ValueError("ed25519 seed must be 32 bytes")
    a, _ = _expand_seed(seed)
    return _compress(_scalar_mult(_BASE, a))


def ed25519_sign(seed: bytes, message: bytes) -> bytes:
    if len(seed) != 32:
        raise ValueError("ed25519 seed must be 32 bytes")
    a, prefix = _expand_seed(seed)
    pub = _compress(_scalar_mult(_BASE, a))
    r = int.from_bytes(_sha512(prefix + message), "little") % _L
    big_r = _compress(_scalar_mult(_BASE, r))
    k = int.from_bytes(_sha512(big_r + pub + message), "little") % _L
    s = (r + k * a) % _L
    return big_r + int.to_bytes(s, 32, "little")


def ed25519_verify(public_key: bytes, message: bytes, signature: bytes) -> bool:
    if len(public_key) != 32 or len(signature) != 64:
        return False
    a_point = _decompress(public_key)
    r_point = _decompress(signature[:32])
    if a_point is None or r_point is None:
        return False
    s = int.from_bytes(signature[32:], "little")
    if s >= _L:
        return False
    k = int.from_bytes(_sha512(signature[:32] + public_key + message), "little") % _L
    return _points_equal(_scalar_mult(_BASE, s), _point_add(r_point, _scalar_mult(a_point, k)))


# --------------------------------------------------------------------------
# minisign file formats
# --------------------------------------------------------------------------

SIG_ALG_PREHASHED = b"ED"
SIG_ALG_LEGACY = b"Ed"
KDF_ALG_NONE = b"\x00\x00"
CKSUM_ALG_BLAKE2B = b"B2"

_COMMENT_UNTRUSTED = "untrusted comment: "
_COMMENT_TRUSTED = "trusted comment: "


class MinisignError(Exception):
    pass


class PublicKey:
    def __init__(self, key_id: bytes, key: bytes) -> None:
        self.key_id = key_id
        self.key = key

    @property
    def key_id_hex(self) -> str:
        # minisign prints the key id little-endian-reversed, the way `minisign -R`
        # shows it in the "untrusted comment" of a generated .pub.
        return self.key_id[::-1].hex().upper()

    @classmethod
    def parse(cls, text: str) -> "PublicKey":
        payload = _payload_line(text, 1)
        raw = base64.b64decode(payload)
        if len(raw) != 42:
            raise MinisignError(f"public key payload is {len(raw)} bytes, expected 42")
        if raw[:2] not in (SIG_ALG_LEGACY, SIG_ALG_PREHASHED):
            raise MinisignError(f"unsupported public key algorithm {raw[:2]!r}")
        return cls(raw[2:10], raw[10:42])

    def to_text(self) -> str:
        raw = SIG_ALG_LEGACY + self.key_id + self.key
        return (
            f"{_COMMENT_UNTRUSTED}minisign public key {self.key_id_hex}\n"
            f"{base64.b64encode(raw).decode('ascii')}\n"
        )


class SecretKey:
    def __init__(self, key_id: bytes, seed: bytes) -> None:
        self.key_id = key_id
        self.seed = seed
        self.public_key = ed25519_public_key(seed)

    @classmethod
    def generate(cls) -> "SecretKey":
        return cls(secrets.token_bytes(8), secrets.token_bytes(32))

    @classmethod
    def parse(cls, text: str) -> "SecretKey":
        raw = base64.b64decode(_payload_line(text, 1))
        # sig_alg[2] kdf_alg[2] cksum_alg[2] salt[32] ops[8] mem[8] then keynum_sk
        if len(raw) < 54 + 8 + 64:
            raise MinisignError("secret key payload is too short")
        if raw[2:4] != KDF_ALG_NONE:
            raise MinisignError(
                "this secret key is password-protected; minisign.py only reads "
                "unencrypted keys (generate one with `keygen`, or export the raw "
                "seed into HUB_MINISIGN_SECRET_KEY)"
            )
        key_id = raw[54:62]
        sk = raw[62:126]  # libsodium layout: seed[32] || public key[32]
        return cls(key_id, sk[:32])

    def to_text(self) -> str:
        sk = self.seed + self.public_key
        body = self.key_id + sk
        chk = hashlib.blake2b(SIG_ALG_LEGACY + body, digest_size=32).digest()
        raw = (
            SIG_ALG_LEGACY
            + KDF_ALG_NONE
            + CKSUM_ALG_BLAKE2B
            + b"\x00" * 32  # kdf salt, unused without a password
            + b"\x00" * 8  # opslimit
            + b"\x00" * 8  # memlimit
            + body
            + chk
        )
        return (
            f"{_COMMENT_UNTRUSTED}minisign encrypted secret key "
            f"{self.key_id[::-1].hex().upper()}\n"
            f"{base64.b64encode(raw).decode('ascii')}\n"
        )

    def to_public_key(self) -> PublicKey:
        return PublicKey(self.key_id, self.public_key)


def _payload_line(text: str, index: int) -> str:
    lines = [ln.strip() for ln in text.replace("\r\n", "\n").split("\n") if ln.strip()]
    if len(lines) <= index:
        raise MinisignError("truncated minisign file")
    return lines[index]


def _comment_line(text: str, index: int, prefix: str) -> str:
    lines = [ln.rstrip("\r\n") for ln in text.replace("\r\n", "\n").split("\n")]
    lines = [ln for ln in lines if ln.strip()]
    if len(lines) <= index:
        raise MinisignError("truncated minisign file")
    line = lines[index]
    if not line.startswith(prefix):
        raise MinisignError(f"expected a {prefix.strip()!r} line, got {line[:40]!r}")
    return line[len(prefix):]


def sign_bytes(
    secret_key: SecretKey,
    data: bytes,
    *,
    untrusted_comment: str = "signature from the Hero Siege Toolkit catalog signer",
    trusted_comment: str = "",
) -> str:
    """Produce the text of a `.minisig` covering `data`."""
    digest = hashlib.blake2b(data, digest_size=64).digest()
    signature = ed25519_sign(secret_key.seed, digest)
    global_sig = ed25519_sign(
        secret_key.seed, signature + trusted_comment.encode("utf-8")
    )
    header = SIG_ALG_PREHASHED + secret_key.key_id + signature
    return (
        f"{_COMMENT_UNTRUSTED}{untrusted_comment}\n"
        f"{base64.b64encode(header).decode('ascii')}\n"
        f"{_COMMENT_TRUSTED}{trusted_comment}\n"
        f"{base64.b64encode(global_sig).decode('ascii')}\n"
    )


def verify_bytes(public_key: PublicKey, data: bytes, signature_text: str) -> bool:
    """Check a `.minisig` against `data`. Mirrors what the hub's Rust side does."""
    raw = base64.b64decode(_payload_line(signature_text, 1))
    if len(raw) != 74:
        return False
    alg, key_id, signature = raw[:2], raw[2:10], raw[10:74]
    if key_id != public_key.key_id:
        return False
    if alg == SIG_ALG_PREHASHED:
        message = hashlib.blake2b(data, digest_size=64).digest()
    elif alg == SIG_ALG_LEGACY:
        message = data
    else:
        return False
    if not ed25519_verify(public_key.key, message, signature):
        return False

    # The trusted comment is only trustworthy because it is covered by a second
    # signature over signature||comment. Skipping this check is the classic
    # minisign misuse, so it is not optional here.
    trusted = _comment_line(signature_text, 2, _COMMENT_TRUSTED)
    global_sig = base64.b64decode(_payload_line(signature_text, 3))
    return ed25519_verify(
        public_key.key, signature + trusted.encode("utf-8"), global_sig
    )


def load_secret_key(key_path: Optional[Path], env_var: str = "HUB_MINISIGN_SECRET_KEY") -> SecretKey:
    """Read a signing key from a `.key` file, or from a base64 seed in the env.

    CI holds the seed as a repository secret; a maintainer signing locally points
    at the `.key` file `keygen` wrote.
    """
    if key_path is not None:
        return SecretKey.parse(key_path.read_text(encoding="utf-8"))
    raw = os.environ.get(env_var)
    if not raw:
        raise MinisignError(
            f"no --key given and {env_var} is unset; nothing to sign with"
        )
    decoded = base64.b64decode(raw.strip())
    if len(decoded) == 32:
        # A bare seed: derive a key id from the public key so the .pub the hub
        # embeds stays stable for a given seed.
        seed = decoded
        key_id = hashlib.blake2b(ed25519_public_key(seed), digest_size=8).digest()
        return SecretKey(key_id, seed)
    if len(decoded) == 40:
        return SecretKey(decoded[:8], decoded[8:40])
    raise MinisignError(
        f"{env_var} decoded to {len(decoded)} bytes; expected 32 (seed) or 40 (key id + seed)"
    )


def _main(argv: Optional[list] = None) -> int:
    import argparse

    parser = argparse.ArgumentParser(description="minisign keygen/sign/verify")
    sub = parser.add_subparsers(dest="command", required=True)

    gen = sub.add_parser("keygen", help="write a new unencrypted key pair")
    gen.add_argument("--secret-key", type=Path, default=Path("catalog-signing.key"))
    gen.add_argument("--public-key", type=Path, default=Path("catalog-signing.pub"))
    gen.add_argument(
        "--show-seed",
        action="store_true",
        help="also print the base64 seed to paste into a CI secret. Off by "
             "default: a key printed to a terminal is a key in that terminal's "
             "scrollback, its logs, and anything reading either.",
    )

    sgn = sub.add_parser("sign", help="sign a file, writing <file>.minisig")
    sgn.add_argument("file", type=Path)
    sgn.add_argument("--key", type=Path, default=None)
    sgn.add_argument("--out", type=Path, default=None)
    sgn.add_argument("--trusted-comment", default="")

    exp = sub.add_parser(
        "export-seed",
        help="print the base64 seed for HUB_MINISIGN_SECRET_KEY. Run it only "
             "where you are about to paste the result into a secret store.",
    )
    exp.add_argument("--key", type=Path, required=True)

    ver = sub.add_parser("verify", help="verify a file against a public key")
    ver.add_argument("file", type=Path)
    ver.add_argument("--public-key", type=Path, required=True)
    ver.add_argument("--signature", type=Path, default=None)

    args = parser.parse_args(argv)

    if args.command == "keygen":
        key = SecretKey.generate()
        args.secret_key.write_text(key.to_text(), encoding="utf-8")
        args.public_key.write_text(key.to_public_key().to_text(), encoding="utf-8")
        try:
            os.chmod(args.secret_key, 0o600)
        except OSError:
            pass  # Windows ACLs do not map onto this; the path is the protection
        print(f"secret key -> {args.secret_key}")
        print(f"public key -> {args.public_key}")
        if args.show_seed:
            print(f"seed (for CI secrets) -> {base64.b64encode(key.key_id + key.seed).decode()}")
        else:
            print(
                "to put this key in CI, run:\n"
                f'  py -3 tools/minisign.py export-seed --key "{args.secret_key}"'
            )
        return 0

    if args.command == "export-seed":
        key = SecretKey.parse(args.key.read_text(encoding="utf-8"))
        print(base64.b64encode(key.key_id + key.seed).decode())
        return 0

    if args.command == "sign":
        key = load_secret_key(args.key)
        data = args.file.read_bytes()
        out = args.out or args.file.with_name(args.file.name + ".minisig")
        comment = args.trusted_comment or f"file:{args.file.name}"
        out.write_text(sign_bytes(key, data, trusted_comment=comment), encoding="utf-8")
        print(f"signature -> {out}")
        return 0

    pub = PublicKey.parse(args.public_key.read_text(encoding="utf-8"))
    sig_path = args.signature or args.file.with_name(args.file.name + ".minisig")
    ok = verify_bytes(pub, args.file.read_bytes(), sig_path.read_text(encoding="utf-8"))
    print("OK" if ok else "BAD SIGNATURE")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(_main())
