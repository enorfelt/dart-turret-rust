import os
import tempfile
import unittest

from manifold3d import Manifold

import stack
import tray


def solid_at(m, x, y, z):
    return (m ^ Manifold.cube((0.2, 0.2, 0.2), center=True).translate((x, y, z))).volume() > 1e-6


class Envelope(unittest.TestCase):
    def test_heights(self):
        e = tray.envelope()
        expected = {"tube_top": 42.0, "tie_top": 43.5, "power_board_bottom": 27.0, "logic_bottom": 46.0,
                    "esp_top": 65.0, "bms_top": 27.95, "deck_bottom": 30.0, "pdb_bottom": 47.0, "xt60_top": 67.6}
        for k, v in expected.items():
            self.assertAlmostEqual(e[k], v, places=6, msg=k)

    def test_default_passes(self):
        self.assertEqual(tray.check_envelope(), [])

    def assertFlags(self, word, **change):
        problems = tray.check_envelope(dict(tray.TRAY, **change))
        self.assertTrue(any(word in p for p in problems), problems)

    def test_esp_above_ceiling(self):
        self.assertFlags("ESP", stack2_height=50.0)

    def test_xt60_above_ceiling(self):
        self.assertFlags("XT60", pdb_bottom=56.0)

    def test_pdb_on_ties(self):
        self.assertFlags("zip ties", pdb_bottom=44.0)

    def test_thick_foam_hits_deck(self):
        self.assertFlags("MOSFETs", foam_t=2.0)

    def test_board_wider_than_gap(self):
        self.assertFlags("between the tubes", tube_gap=stack.W)

    def test_logic_board_in_tubes(self):
        self.assertFlags("logic board", floor_bottom=15.0)

    def test_bore_too_tight(self):
        self.assertFlags("bore", tube_d=12.15)


class Tray(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.tray()

    def test_one_watertight_piece(self):
        self.assertEqual(str(self.m.status()), "Error.NoError")
        self.assertEqual(len(self.m.decompose()), 1)

    def test_extent(self):
        for got, want in zip(self.m.bounding_box(), (-51, 0, 20, 51, 150, 36)):
            self.assertAlmostEqual(got, want, places=3)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": self.m}), [])

    def test_tight_bore_flagged(self):
        t = dict(tray.TRAY, bore_d=11.8)
        self.assertTrue(any("tubes" in p for p in tray.check_parts(t, {"tray": tray.tray(t)})))

    def test_cradles(self):
        for y0 in tray.TRAY["cradles_y"]:
            for x in (-42, 42):
                self.assertTrue(solid_at(self.m, x, y0 + 6, 29.5))
                self.assertFalse(solid_at(self.m, x, y0 + 6, 31))
                for ty in (3, 9):
                    self.assertFalse(solid_at(self.m, x, y0 + ty, 26))
                self.assertTrue(solid_at(self.m, x, y0 + 6, 26))

    def test_bosses_on_stack_holes(self):
        for hx, hy in stack.HOLES:
            x, y = hx - stack.W / 2, hy + 68
            self.assertTrue(solid_at(self.m, x + 2.6, y, 25))
            for z in (21.5, 25):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_posts(self):
        for x, y in tray.TRAY["posts"]:
            self.assertTrue(solid_at(self.m, x + 2.6, y, 26))
            for z in (21.5, 26):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_pocket_and_vents(self):
        self.assertFalse(solid_at(self.m, 0, 51, 22.5))
        self.assertTrue(solid_at(self.m, 0, 55, 22.5))
        for y in (21, 31, 41):
            self.assertFalse(solid_at(self.m, 0, y, 21.5))
        self.assertTrue(solid_at(self.m, 0, 26, 21.5))

    def test_tie_slots(self):
        for x, y, _ in tray.TRAY["tie_slots"]:
            self.assertFalse(solid_at(self.m, x, y, 21.5))


class Deck(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.deck()

    def test_one_piece_up_to_the_pdb(self):
        self.assertEqual(len(self.m.decompose()), 1)
        self.assertAlmostEqual(self.m.bounding_box()[5], 47.0, places=3)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": tray.tray(), "deck": self.m}), [])

    def test_post_screws_go_through(self):
        both = tray.tray() + self.m
        for x, y in tray.TRAY["posts"]:
            rod = Manifold.cylinder(14, 1.5, 1.5, 32).translate((x, y, 20))
            self.assertLess((both ^ rod).volume(), 1e-6)

    def test_pdb_screws_fit_30_and_30_5(self):
        for pattern in (30.0, 30.5):
            for sx in (-1, 1):
                for sy in (-1, 1):
                    rod = Manifold.cylinder(14, 1.5, 1.5, 32).translate((sx * pattern / 2, 31 + sy * pattern / 2, 33))
                    self.assertLess((self.m ^ rod).volume(), 1e-6, (pattern, sx, sy))

    def test_nut_slots_open_outwards(self):
        for sx in (-1, 1):
            x = sx * 30.25 / 2
            self.assertFalse(solid_at(self.m, x + sx * 3.0, 31 + 15.125, 42.7))
            self.assertTrue(solid_at(self.m, x + sx * 3.0, 31 + 15.125, 39.0))


class Output(unittest.TestCase):
    def test_writes_three_stls(self):
        with tempfile.TemporaryDirectory() as d:
            tray.main(d)
            for name in ("tray.stl", "deck.stl", "cradle-test.stl"):
                self.assertGreater(os.path.getsize(os.path.join(d, name)), 1000)

    def test_failed_check_writes_nothing(self):
        with tempfile.TemporaryDirectory() as d:
            out = os.path.join(d, "tray")
            with self.assertRaises(SystemExit):
                tray.main(out, dict(tray.TRAY, foam_t=2.0))
            self.assertFalse(os.path.exists(out))

    def test_cradle_test_is_one_piece(self):
        self.assertEqual(len(tray.cradle_test().decompose()), 1)


if __name__ == "__main__":
    unittest.main()
