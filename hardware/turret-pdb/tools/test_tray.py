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


if __name__ == "__main__":
    unittest.main()
