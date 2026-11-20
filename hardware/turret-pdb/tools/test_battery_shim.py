import os
import tempfile
import unittest

from manifold3d import Manifold

import battery_shim as bs

S = bs.BATTERY_SHIM


def solid_at(m, x, y, z):
    return (m ^ Manifold.cube((0.2, 0.2, 0.2), center=True).translate((x, y, z))).volume() > 1e-6


class Layout(unittest.TestCase):
    def test_passes_checks(self):
        self.assertEqual(bs.check(), [])

    def test_wall_between_screw_holes_and_cells(self):
        self.assertAlmostEqual(bs.hole_walls(), 0.74, places=2)

    def test_holes_cells_and_window_are_open(self):
        m = bs.shim(1.0)
        open_points = list(S["holes"]) + [(y, z) for y in S["stacks_y"] for z in S["cells_z"]] + [(0.0, -66.5)]
        for y, z in open_points:
            self.assertFalse(solid_at(m, y, z, 0.5), (y, z))

    def test_clamped_areas_are_solid(self):
        m = bs.shim(1.0)
        for y, z in ((0.0, -50.0), (0.0, -86.0), (-43.0, -55.0), (43.0, -55.0), (36.0, -88.0), (-12.8, -66.5)):
            self.assertTrue(solid_at(m, y, z, 0.5), (y, z))

    def test_wire_slits_reach_the_side_edges(self):
        m = bs.shim(1.0)
        w, z = S["wire_slit"]
        for y in (-36.5, 36.5):
            self.assertFalse(solid_at(m, y, z, 0.5), y)
            self.assertTrue(solid_at(m, y, z + w / 2 + 0.3, 0.5), y)
            self.assertTrue(solid_at(m, y, z - w / 2 - 0.3, 0.5), y)
        self.assertTrue(solid_at(m, -12.8, z, 0.5))
        self.assertTrue(solid_at(m, 12.8, z, 0.5))

    def test_matches_cover_outline(self):
        x0, y0, _, x1, y1, _ = bs.shim(1.0).bounding_box()
        self.assertAlmostEqual(x1 - x0, 98.2, places=1)
        self.assertAlmostEqual(y1 - y0, 44.0, places=1)

    def test_original_screws_still_bite(self):
        for t in S["thicknesses"]:
            self.assertEqual(bs.screw_for(t), 12)
        self.assertEqual(bs.screw_for(3.5), 14)


class Output(unittest.TestCase):
    def test_writes_one_stl_per_thickness(self):
        with tempfile.TemporaryDirectory() as out:
            bs.main(out)
            self.assertEqual(sorted(os.listdir(out)), [f"{bs.name(t)}.stl" for t in S["thicknesses"]])


if __name__ == "__main__":
    unittest.main()
