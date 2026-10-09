import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from macos_host import InstallError, parse_ids


class DirectoryIdentityObservation(unittest.TestCase):
    def test_macos_reserved_users_and_groups(self):
        users = parse_ids("root 0\nnobody -2\noasis7-codex 504\n")
        groups = parse_ids("wheel 0\nnobody -2\nnogroup -1\nstaff 20\n")
        self.assertEqual(users, {"root": 0, "nobody": 4294967294, "oasis7-codex": 504})
        self.assertEqual(groups["nogroup"], 4294967295)
        available = [value for value in range(400, 500)
                     if value not in users.values() and value not in groups.values()]
        self.assertEqual(available[0], 400)

    def test_signed_and_unsigned_uid_t_equivalence(self):
        self.assertEqual(parse_ids("nobody -2"), parse_ids("nobody 4294967294"))
        self.assertEqual(parse_ids("nogroup -1"), parse_ids("nogroup 4294967295"))

    def test_uint32_boundaries(self):
        self.assertEqual(parse_ids("low -2147483648\nhigh 4294967295"),
                         {"low": 2147483648, "high": 4294967295})
        for value in ("-2147483649", "4294967296", "-4294966896"):
            with self.subTest(value=value), self.assertRaises(InstallError):
                parse_ids("account " + value)

    def test_malformed_and_noncanonical_values_rejected(self):
        for value in ("+400", "-0", "0400", "-02", "4.0", "４００", "--2", "NaN"):
            with self.subTest(value=value), self.assertRaises(InstallError):
                parse_ids("account " + value)

    def test_missing_extra_and_duplicate_fields_rejected(self):
        for output in ("", "account", "account 400 extra", "account 400\naccount 401", "account 400\n\n"):
            with self.subTest(output=output), self.assertRaises(InstallError):
                parse_ids(output)

    def test_allocation_keeps_observed_positive_ids_reserved(self):
        observed = parse_ids("reserved -2\nexisting 400\nother 401")
        self.assertEqual(next(value for value in range(400, 500)
                              if value not in observed.values()), 402)


if __name__ == "__main__":
    unittest.main()
