"""Tests for the catalog signer.

The point of these is interoperability, not "our verify accepts our sign" -- the
thing that actually checks these signatures is the Rust `minisign-verify` crate
inside the hub, so the bytes have to match minisign's format rather than a
private one. The RFC 8032 vectors pin the curve arithmetic; the payload-length
assertions pin the container.
"""

import base64
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import minisign  # noqa: E402


# RFC 8032 section 7.1, test vectors 1, 2 and 3.
RFC8032 = [
    (
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33"
        "bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
    ),
    (
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e159"
        "96e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
    ),
    (
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f"
        "290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
    ),
]


class TestEd25519(unittest.TestCase):
    def test_rfc8032_vectors(self):
        for seed_hex, pub_hex, msg_hex, sig_hex in RFC8032:
            with self.subTest(seed=seed_hex[:8]):
                seed = bytes.fromhex(seed_hex)
                message = bytes.fromhex(msg_hex)
                self.assertEqual(minisign.ed25519_public_key(seed).hex(), pub_hex)
                self.assertEqual(minisign.ed25519_sign(seed, message).hex(), sig_hex)
                self.assertTrue(
                    minisign.ed25519_verify(bytes.fromhex(pub_hex), message, bytes.fromhex(sig_hex))
                )

    def test_verify_rejects_a_flipped_message_bit(self):
        seed = bytes.fromhex(RFC8032[1][0])
        pub = minisign.ed25519_public_key(seed)
        sig = minisign.ed25519_sign(seed, b"\x72")
        self.assertFalse(minisign.ed25519_verify(pub, b"\x73", sig))

    def test_verify_rejects_a_flipped_signature_bit(self):
        seed = bytes.fromhex(RFC8032[1][0])
        pub = minisign.ed25519_public_key(seed)
        sig = bytearray(minisign.ed25519_sign(seed, b"\x72"))
        sig[0] ^= 0x01
        self.assertFalse(minisign.ed25519_verify(pub, b"\x72", bytes(sig)))

    def test_verify_rejects_a_non_canonical_scalar(self):
        # s must be reduced mod L; an unreduced s is the classic malleability bug.
        seed = bytes.fromhex(RFC8032[1][0])
        pub = minisign.ed25519_public_key(seed)
        sig = minisign.ed25519_sign(seed, b"\x72")
        big_s = int.from_bytes(sig[32:], "little") + (2 ** 252 + 27742317777372353535851937790883648493)
        mangled = sig[:32] + big_s.to_bytes(32, "little")
        self.assertFalse(minisign.ed25519_verify(pub, b"\x72", mangled))

    def test_rejects_a_seed_of_the_wrong_length(self):
        with self.assertRaises(ValueError):
            minisign.ed25519_sign(b"short", b"x")


class TestKeyFormat(unittest.TestCase):
    def test_public_key_payload_is_minisigns_42_bytes(self):
        key = minisign.SecretKey.generate()
        payload = key.to_public_key().to_text().strip().split("\n")[1]
        self.assertEqual(len(payload), 56)  # base64 of 42 bytes
        self.assertEqual(len(base64.b64decode(payload)), 42)

    def test_secret_key_payload_is_minisigns_158_bytes(self):
        key = minisign.SecretKey.generate()
        payload = key.to_text().strip().split("\n")[1]
        self.assertEqual(len(payload), 212)  # base64 of 158 bytes
        self.assertEqual(len(base64.b64decode(payload)), 158)

    def test_key_pair_round_trips_through_its_own_text(self):
        key = minisign.SecretKey.generate()
        reloaded = minisign.SecretKey.parse(key.to_text())
        self.assertEqual(reloaded.seed, key.seed)
        self.assertEqual(reloaded.key_id, key.key_id)
        self.assertEqual(reloaded.public_key, key.public_key)

        pub = minisign.PublicKey.parse(key.to_public_key().to_text())
        self.assertEqual(pub.key, key.public_key)
        self.assertEqual(pub.key_id, key.key_id)

    def test_password_protected_key_is_refused_with_a_useful_message(self):
        key = minisign.SecretKey.generate()
        raw = bytearray(base64.b64decode(key.to_text().strip().split("\n")[1]))
        raw[2:4] = b"Sc"  # scrypt: what `minisign -G` writes without -W
        text = "untrusted comment: x\n" + base64.b64encode(bytes(raw)).decode() + "\n"
        with self.assertRaises(minisign.MinisignError) as ctx:
            minisign.SecretKey.parse(text)
        self.assertIn("password-protected", str(ctx.exception))


class TestSignatureFormat(unittest.TestCase):
    def setUp(self):
        self.key = minisign.SecretKey.generate()
        self.pub = self.key.to_public_key()
        self.data = b'{"schema": 1, "tools": []}\n'

    def test_signature_payload_is_minisigns_74_bytes_and_prehashed(self):
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog")
        raw = base64.b64decode(text.strip().split("\n")[1])
        self.assertEqual(len(raw), 74)
        self.assertEqual(raw[:2], b"ED")  # prehashed; what minisign writes by default
        self.assertEqual(raw[2:10], self.key.key_id)

    def test_signature_has_all_four_lines_in_minisigns_order(self):
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog 1")
        lines = text.strip().split("\n")
        self.assertEqual(len(lines), 4)
        self.assertTrue(lines[0].startswith("untrusted comment: "))
        self.assertTrue(lines[2].startswith("trusted comment: "))

    def test_round_trip_verifies(self):
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog")
        self.assertTrue(minisign.verify_bytes(self.pub, self.data, text))

    def test_modified_payload_fails(self):
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog")
        self.assertFalse(minisign.verify_bytes(self.pub, self.data + b" ", text))

    def test_signature_from_another_key_fails(self):
        other = minisign.SecretKey.generate()
        text = minisign.sign_bytes(other, self.data, trusted_comment="catalog")
        self.assertFalse(minisign.verify_bytes(self.pub, self.data, text))

    def test_edited_trusted_comment_fails(self):
        # The trusted comment is only trustworthy because a second signature
        # covers signature||comment. Verifying the first and printing the second
        # is the standard minisign misuse.
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog 2026-09-11")
        tampered = text.replace("catalog 2026-09-11", "catalog 2030-01-01")
        self.assertFalse(minisign.verify_bytes(self.pub, self.data, tampered))

    def test_truncated_signature_file_fails_rather_than_raising(self):
        text = minisign.sign_bytes(self.key, self.data, trusted_comment="catalog")
        head = "\n".join(text.strip().split("\n")[:2]) + "\n"
        with self.assertRaises(minisign.MinisignError):
            minisign.verify_bytes(self.pub, self.data, head)


class TestSecretKeyLoading(unittest.TestCase):
    def test_loads_a_bare_seed_from_the_environment(self):
        import os
        key = minisign.SecretKey.generate()
        os.environ["HUB_TEST_KEY"] = base64.b64encode(key.seed).decode()
        try:
            loaded = minisign.load_secret_key(None, env_var="HUB_TEST_KEY")
            self.assertEqual(loaded.seed, key.seed)
            # A bare seed gets a derived key id, so it is stable across runs.
            again = minisign.load_secret_key(None, env_var="HUB_TEST_KEY")
            self.assertEqual(again.key_id, loaded.key_id)
        finally:
            del os.environ["HUB_TEST_KEY"]

    def test_loads_key_id_plus_seed_from_the_environment(self):
        import os
        key = minisign.SecretKey.generate()
        os.environ["HUB_TEST_KEY"] = base64.b64encode(key.key_id + key.seed).decode()
        try:
            loaded = minisign.load_secret_key(None, env_var="HUB_TEST_KEY")
            self.assertEqual(loaded.key_id, key.key_id)
            self.assertEqual(loaded.seed, key.seed)
        finally:
            del os.environ["HUB_TEST_KEY"]

    def test_missing_key_is_an_error_not_an_unsigned_catalog(self):
        import os
        os.environ.pop("HUB_TEST_MISSING", None)
        with self.assertRaises(minisign.MinisignError):
            minisign.load_secret_key(None, env_var="HUB_TEST_MISSING")


if __name__ == "__main__":
    unittest.main()
