import os
import tempfile
import unittest

from manifold3d import Manifold

import esc_holder as eh

H = eh.ESC_HOLDER


def solid_at(m, x, y, z):
    return (m ^ Manifold.cube((0.2, 0.2, 0.2), center=True).translate((x, y, z))).volume() > 1e-6


class Layout(unittest.TestCase):
    def test_centre_between_holes(self):
        self.assertAlmostEqual(eh.centre(), 18.025)

    def test_floor_fits_between_tabs(self):
        self.assertAlmostEqual(eh.half_width(), 16.475)

    def test_esc_longer_than_tab_gap(self):
        left, right = H["tab_inner_x"]
        self.assertGreater(H["esc"][0], right - left)

    def test_esc_beside_tabs(self):
        self.assertLess(H["esc_inner_y"], H["tab_y"] + H["tab_chamfer"])
        self.assertEqual(eh.esc_y(), (-42.8, -25.8))

    def test_hooks_follow_esc_height(self):
        front, back = eh.hook_z()
        self.assertAlmostEqual(front, 6.3)
        self.assertAlmostEqual(back, 7.7)

    def test_straight_outer_edge(self):
        self.assertAlmostEqual(eh.outer_y(), -45.1)
        self.assertAlmostEqual(eh.overhang(), 7.1)

    def test_clip_strain(self):
        front, back = eh.clip_strain()
        self.assertAlmostEqual(front, 1.5 * 1.0 * 0.4 / 5.3 ** 2)
        self.assertLess(back, front)
        self.assertLess(front, H["max_strain"])


class Checks(unittest.TestCase):
    def test_default_passes(self):
        self.assertEqual(eh.check(), [])

    def assertFlags(self, word, **change):
        problems = eh.check(dict(H, **change))
        self.assertTrue(any(word in p for p in problems), problems)

    def test_floor_onto_tabs(self):
        self.assertFlags("hits the magazine", gap=-0.2)

    def test_esc_onto_tab_chamfers(self):
        self.assertFlags("ESC hits", esc_inner_y=-24.6)

    def test_long_esc_reaches_tabs(self):
        self.assertFlags("ESC hits", esc=(36.5, 17.0), esc_inner_y=-25.0)

    def test_hooks_into_esc(self):
        self.assertFlags("ESC hits", top_gap=-0.3)

    def test_low_clips_too_stiff(self):
        self.assertFlags("motor end clips", esc_h=(3.0, 5.9))

    def test_floor_onto_gusset(self):
        self.assertFlags("hits the magazine", gusset=(-12.95, 2.0, -40.0, 19.0))

    def test_clip_over_screw_head(self):
        self.assertFlags("screw heads", esc_inner_y=-21.0, tab_y=-20.0, tab_chamfer=0.0, gap=-5.0)

    def test_split_piece(self):
        self.assertFlags("watertight", tie_slot=(7.0, 15.0, -38.0), slot_wall=1.0)


class Shape(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = eh.holder()
        cls.c = eh.centre()
        cls.y0, cls.y1 = eh.esc_y()

    def test_screw_holes_open(self):
        for x, y in H["holes"]:
            self.assertFalse(solid_at(self.m, x, y, 0.8))
            self.assertTrue(solid_at(self.m, x + H["hole_d"] / 2 + 0.5, y, 0.8))

    def test_floor_under_esc(self):
        for y in (self.y0 + 1.0, self.y1 - 1.0, H["edge_y"] - 3.0):
            self.assertTrue(solid_at(self.m, self.c, y, H["floor_t"] - 0.2))
            self.assertFalse(solid_at(self.m, self.c, y, H["floor_t"] + 0.2))

    def test_hooks_reach_over_esc(self):
        for dx, zb in zip((-H["clip_dx"], H["clip_dx"]), eh.hook_z()):
            x, z = self.c + dx, zb + 0.1
            self.assertTrue(solid_at(self.m, x, self.y1 - H["grip"] + 0.15, z))
            self.assertTrue(solid_at(self.m, x, self.y0 + H["grip"] - 0.15, z))
            self.assertFalse(solid_at(self.m, x, self.y1 - H["grip"] - 0.15, z))
            self.assertFalse(solid_at(self.m, x, self.y0 + H["grip"] - 0.15, zb - 0.1))

    def test_motor_end_clips_lower(self):
        front, back = eh.hook_z()
        top = front + H["side_gap"] + H["grip"] + 0.3
        inner = self.y1 + H["side_gap"] + 0.5
        self.assertFalse(solid_at(self.m, self.c - H["clip_dx"], inner, top + 0.2))
        self.assertTrue(solid_at(self.m, self.c + H["clip_dx"], inner, top + 0.2))

    def test_root_fillets_behind_clips(self):
        w, t = H["clip"]
        r, f = H["root_fillet"], H["floor_t"]
        inner_back = self.y1 + H["side_gap"] + t
        outer_back = self.y0 - H["side_gap"] - t
        for dx in (-H["clip_dx"], H["clip_dx"]):
            x = self.c + dx
            self.assertTrue(solid_at(self.m, x, inner_back + 0.15, f + 0.15))
            self.assertFalse(solid_at(self.m, x, inner_back + r - 0.15, f + r - 0.15))
            self.assertTrue(solid_at(self.m, x, outer_back - 0.15, f + 0.15))
            self.assertFalse(solid_at(self.m, x, outer_back - r + 0.15, f + r - 0.15))

    def test_outer_edge_straight(self):
        for x in (self.c - 24.0, self.c - H["clip_dx"], self.c, self.c + 24.0):
            self.assertTrue(solid_at(self.m, x, eh.outer_y() + 0.15, 0.8))
            self.assertFalse(solid_at(self.m, x, eh.outer_y() - 0.15, 0.8))

    def test_relief_grooves_at_clip_fronts(self):
        g, f = H["root_groove"], H["floor_t"]
        inner_face = self.y1 + H["side_gap"]
        outer_face = self.y0 - H["side_gap"]
        for dx in (-H["clip_dx"], H["clip_dx"]):
            x = self.c + dx
            self.assertFalse(solid_at(self.m, x, inner_face - g, f - g + 0.15))
            self.assertFalse(solid_at(self.m, x, outer_face + g, f - g + 0.15))
            self.assertTrue(solid_at(self.m, x, inner_face + 0.15, f - 0.3))
            self.assertTrue(solid_at(self.m, x, inner_face - g, f - g - 0.2))

    def test_tie_slots_past_the_edge(self):
        length = H["esc"][0]
        sw, sd, slot_y = H["tie_slot"]
        end = length / 2 + H["tie_tab"][0] - H["slot_wall"]
        self.assertLess(slot_y + sd / 2, H["edge_y"])
        for sign in (-1, 1):
            x = self.c + sign * (end - sw / 2)
            self.assertFalse(solid_at(self.m, x, slot_y, 0.8))
            self.assertTrue(solid_at(self.m, x, slot_y + sd / 2 + 0.5, 0.8))

    def test_end_stops(self):
        x = H["esc"][0] / 2 + H["side_gap"] + H["stop"][0] / 2
        for sign in (-1, 1):
            self.assertTrue(solid_at(self.m, self.c + sign * x, self.y0 + 1.0, H["floor_t"] + 1.0))
            self.assertFalse(solid_at(self.m, self.c + sign * x, self.y1 - 1.0, H["floor_t"] + 1.0))


class Output(unittest.TestCase):
    def test_writes_stl(self):
        with tempfile.TemporaryDirectory() as d:
            eh.main(d)
            for side in ("left", "right"):
                self.assertTrue(os.path.getsize(os.path.join(d, f"esc-holder-{side}.stl")) > 1000)


if __name__ == "__main__":
    unittest.main()
