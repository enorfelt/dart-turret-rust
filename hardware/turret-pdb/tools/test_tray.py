import os
import tempfile
import unittest
from unittest import mock

from manifold3d import Manifold

import power
import stack
import tray


def solid_at(m, x, y, z):
    return (m ^ Manifold.cube((0.2, 0.2, 0.2), center=True).translate((x, y, z))).volume() > 1e-6


class Envelope(unittest.TestCase):
    def test_heights(self):
        e = tray.envelope()
        expected = {"tube_top": 42.0, "tie_top": 43.5, "power_board_bottom": 34.0, "bucks_bottom": 27.0,
                    "logic_bottom": 52.6, "esp_top": 71.2, "bms_top": 27.95, "deck_bottom": 30.0, "pdb_bottom": 47.0, "pdb_top": 52.6,
                    "switch_top": 56.0, "switch_bottom": 12.0}
        for k, v in expected.items():
            self.assertAlmostEqual(e[k], v, places=6, msg=k)

    def test_default_passes(self):
        self.assertEqual(tray.check_envelope(), [])

    def assertFlags(self, word, **change):
        problems = tray.check_envelope(dict(tray.TRAY, **change))
        self.assertTrue(any(word in p for p in problems), problems)

    def test_esp_above_ceiling(self):
        self.assertFlags("ESP", esp_above_logic=29.0)

    def test_pdb_above_ceiling(self):
        self.assertFlags("PDB parts", pdb_bottom=72.0)

    def test_pdb_on_ties(self):
        self.assertFlags("zip ties", pdb_bottom=44.0)

    def test_thick_foam_hits_deck(self):
        self.assertFlags("MOSFETs", foam_t=2.0)

    def test_board_wider_than_gap(self):
        self.assertFlags("between the tubes", tube_gap=stack.W)

    def test_logic_board_in_tubes(self):
        self.assertFlags("logic board", floor_bottom=9.0)

    def test_bore_too_tight(self):
        self.assertFlags("bore", tube_d=12.15)

    def test_esp_follows_stack_gap(self):
        with mock.patch.object(stack, "GAP", 30.0):
            self.assertTrue(any("ESP" in p for p in tray.check_envelope()))

    def test_tunnel_roof_in_bore(self):
        self.assertFlags("tunnel", tunnel=(5.0, 2.0, 25.0))

    def test_front_cradle_ties_clear_bms(self):
        self.assertFlags("BMS", bms_y=31.0)

    def test_bucks_on_the_floor_without_risers(self):
        self.assertFlags("buck modules", riser_h=0.0)

    def test_switch_lever_above_ceiling(self):
        self.assertFlags("arm switch", switch_above=42.0)

    def test_switch_body_near_floor_flagged(self):
        self.assertFlags("floor", switch_body=(13.0, 13.0, 26.0))


class Tray(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.tray()

    def test_one_watertight_piece(self):
        self.assertEqual(str(self.m.status()), "Error.NoError")
        self.assertEqual(len(self.m.decompose()), 1)

    def test_extent(self):
        for got, want in zip(self.m.bounding_box(), (-53, 0, 20, 53, tray.TRAY["length"], 36)):
            self.assertAlmostEqual(got, want, places=3)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": self.m}), [])

    def test_tight_bore_flagged(self):
        t = dict(tray.TRAY, bore_d=11.8)
        self.assertTrue(any("tubes" in p for p in tray.check_parts(t, {"tray": tray.tray(t)})))

    def test_wide_boss_under_a_buck_flagged(self):
        t = dict(tray.TRAY, boss_d=8.0, riser_h=6.0)
        self.assertTrue(any("buck modules" in p for p in tray.check_parts(t, {"tray": tray.tray(t)})))

    def test_cradles(self):
        for y0 in tray.TRAY["cradles_y"]:
            for x in (-44, 44):
                self.assertTrue(solid_at(self.m, x, y0 + 6, 29.5))
                self.assertFalse(solid_at(self.m, x, y0 + 6, 31))
                for ty in (3, 9):
                    self.assertFalse(solid_at(self.m, x, y0 + ty, 24))
                    self.assertFalse(solid_at(self.m, x, y0 + ty, 27.8))
                    self.assertTrue(solid_at(self.m, x, y0 + ty, 28.2))
                    for dy in (-2.3, 2.3):
                        self.assertFalse(solid_at(self.m, x, y0 + ty + dy, 25.4))
                        self.assertTrue(solid_at(self.m, x, y0 + ty + dy, 25.9))
                self.assertTrue(solid_at(self.m, x, y0 + 6, 26))
                self.assertTrue(solid_at(self.m, x, y0 + 3, 23.3))

    def test_bosses_on_stack_holes(self):
        for hx, hy in stack.HOLES:
            x, y = hx - stack.W / 2, hy + tray.TRAY["stack2_y"]
            self.assertTrue(solid_at(self.m, x + 2.6, y, 25))
            for z in (21.5, 25):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_posts(self):
        for x, y in tray.TRAY["posts"]:
            self.assertTrue(solid_at(self.m, x + 2.6, y, 26))
            for z in (21.5, 26):
                self.assertFalse(solid_at(self.m, x, y, z))

    def test_pocket_and_vents(self):
        b = tray.TRAY["bms_y"]
        self.assertGreaterEqual(b - tray.TRAY["bms"][1] / 2, tray.TRAY["cradles_y"][0] + tray.TRAY["cradle_len"] + 1)
        self.assertFalse(solid_at(self.m, 0, b + 20, 22.5))
        self.assertTrue(solid_at(self.m, 0, b + 24, 22.5))
        for y in tray.TRAY["vents_y"]:
            self.assertFalse(solid_at(self.m, 0, y, 21.5))
        self.assertTrue(solid_at(self.m, 0, b - 5, 21.5))

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
                    y = tray.TRAY["bms_y"] + sy * pattern / 2
                    rod = Manifold.cylinder(14, 1.5, 1.5, 32).translate((sx * pattern / 2, y, 33))
                    self.assertLess((self.m ^ rod).volume(), 1e-6, (pattern, sx, sy))

    def test_nut_slots_open_outwards(self):
        for x, y in tray.pillars(tray.TRAY):
            sx = 1 if x > 0 else -1
            self.assertFalse(solid_at(self.m, x + sx * 3.0, y, 42.7))
            self.assertTrue(solid_at(self.m, x + sx * 3.0, y, 39.0))

    def test_nut_slot_is_3_mm_tall(self):
        for x, y in tray.pillars(tray.TRAY):
            sx = 1 if x > 0 else -1
            for z in (41.1, 43.9):
                self.assertFalse(solid_at(self.m, x + sx * 3.0, y, z))
            for z in (40.85, 44.15):
                self.assertTrue(solid_at(self.m, x + sx * 3.0, y, z))

    def test_nut_slot_centres_the_nut(self):
        for x, y in tray.pillars(tray.TRAY):
            sx = 1 if x > 0 else -1
            self.assertFalse(solid_at(self.m, x - sx * 3.2, y, 42.7))
            self.assertTrue(solid_at(self.m, x - sx * 3.6, y, 42.7))
            for dy in (-2.8, 2.8):
                self.assertFalse(solid_at(self.m, x + sx * 1.0, y + dy, 42.7))

    def test_pillar_walls_beside_nut_slot(self):
        for x, y in tray.pillars(tray.TRAY):
            for dy in (-4.4, 4.4):
                self.assertTrue(solid_at(self.m, x, y + dy, 42.7))


class Sleeve(unittest.TestCase):
    def test_spans_the_board_gap(self):
        b = tray.sleeve().bounding_box()
        self.assertAlmostEqual(b[5] - b[2], stack.GAP, places=6)

    def test_bore_is_open(self):
        s = tray.sleeve()
        r = tray.TRAY["sleeve_bore"] / 2 - 0.15
        for z in (0.1, stack.GAP / 2, stack.GAP - 0.1):
            self.assertFalse(solid_at(s, r, 0, z), z)

    def test_wall_is_solid_mid_height(self):
        r = (tray.TRAY["sleeve_od"] + tray.TRAY["sleeve_bore"]) / 4
        self.assertTrue(solid_at(tray.sleeve(), r, 0, stack.GAP / 2))

    def test_ends_are_chamfered(self):
        r = tray.TRAY["sleeve_od"] / 2
        s = tray.sleeve()
        self.assertFalse(solid_at(s, r, 0, 0))
        self.assertFalse(solid_at(s, r, 0, stack.GAP))
        self.assertTrue(solid_at(s, r - 0.15, 0, stack.GAP / 2))

    def test_clears_the_power_board_parts(self):
        self.assertLessEqual(tray.TRAY["sleeve_od"] / 2, 3.0)


class Riser(unittest.TestCase):
    def test_height(self):
        b = tray.riser().bounding_box()
        self.assertAlmostEqual(b[5] - b[2], tray.TRAY["riser_h"], places=6)

    def test_bore_is_open(self):
        r = tray.TRAY["sleeve_bore"] / 2 - 0.15
        self.assertFalse(solid_at(tray.riser(), r, 0, tray.TRAY["riser_h"] / 2))

    def test_clears_the_bucks(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"riser": tray.riser()}), [])

    def test_wide_riser_flagged(self):
        t = dict(tray.TRAY, sleeve_od=8.0)
        self.assertTrue(any("riser" in p for p in tray.check_parts(t, {"riser": tray.riser(t)})))

    def test_bucks_hang_under_the_power_board(self):
        b = tray.bucks_space().bounding_box()
        self.assertAlmostEqual(b[5], tray.envelope()["power_board_bottom"], places=6)
        self.assertAlmostEqual(b[2], tray.envelope()["bucks_bottom"], places=6)


class ArmBracket(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = tray.arm_bracket()
        cls.y0 = tray.TRAY["cradles_y"][-1]
        cls.sx, cls.sy = tray.switch_centre(tray.TRAY)

    def test_one_watertight_piece(self):
        self.assertEqual(str(self.m.status()), "Error.NoError")
        self.assertEqual(len(self.m.decompose()), 1)

    def test_parts_check_passes(self):
        self.assertEqual(tray.check_parts(tray.TRAY, {"tray": tray.tray(), "arm-bracket": self.m}), [])

    def test_spans_the_rear_cradles_and_reaches_behind_the_tray(self):
        b = self.m.bounding_box()
        for got, want in zip(b, (-53, self.y0, 30.0, 53, tray.TRAY["length"] + 16, 46.1)):
            self.assertAlmostEqual(got, want, places=3)

    def test_feet_cover_the_tubes_and_sit_above_the_cradles(self):
        for x in (-44, 44):
            self.assertTrue(solid_at(self.m, x, self.y0 + 6, 45))
            self.assertFalse(solid_at(self.m, x, self.y0 + 6, 41.9))
            for fx in (x - 7.5, x + 7.5):
                self.assertFalse(solid_at(self.m, fx, self.y0 + 6, 36.15))
                self.assertTrue(solid_at(self.m, fx, self.y0 + 6, 36.5))

    def test_cradle_ties_pass_the_inner_faces(self):
        for x in (-34.5, 34.5, -35.5, 35.5):
            for dy in tray.TRAY["tunnels_y"]:
                for dz in (-2.0, 2.0):
                    self.assertFalse(solid_at(self.m, x, self.y0 + dy + dz, 44))
            self.assertTrue(solid_at(self.m, x, tray.TRAY["length"] + 3, 44))

    def test_switch_hole_on_the_bar_left_of_the_jack(self):
        self.assertEqual(self.sx, -15.0)
        self.assertAlmostEqual(self.sy, 163.5, places=6)
        self.assertGreater(self.sy - tray.TRAY["switch_hole_d"] / 2, tray.TRAY["length"] + tray.TRAY["arm_bar_gap"])
        self.assertFalse(solid_at(self.m, self.sx + 3.0, self.sy, 32))
        self.assertTrue(solid_at(self.m, self.sx + 3.4, self.sy, 32))

    def test_nothing_above_the_bar_between_the_legs(self):
        for x in (-20, 0, 15, 25):
            for z in (36, 40, 45):
                self.assertFalse(solid_at(self.m, x, 163.5, z), (x, z))

    def test_legs_join_feet_to_bar(self):
        for x in (-33, 33):
            for z in (32, 40, 45):
                self.assertTrue(solid_at(self.m, x, 163.5, z), (x, z))

    def test_high_bar_flagged(self):
        t = dict(tray.TRAY, arm_bar_top=38.0)
        problems = tray.check_parts(t, {"tray": tray.tray(t), "arm-bracket": tray.arm_bracket(t)})
        self.assertTrue(any("plug" in p for p in problems), problems)

    def test_plug_space_follows_j2(self):
        jx, jz = tray.jack_xz(tray.TRAY)
        self.assertAlmostEqual(jx, 15.0, places=6)
        self.assertAlmostEqual(jz, 42.1, places=6)
        for got, want in zip(tray.plug_space().bounding_box(), (9, 141, 36.1, 21, 172, 75)):
            self.assertAlmostEqual(got, want, places=3)
        j2 = power.PARTS["J2"]
        with mock.patch.dict(power.PARTS, {"J2": j2[:3] + ((30.0,) + j2[3][1:],)}):
            self.assertAlmostEqual(tray.jack_xz(tray.TRAY)[0], -5.0, places=6)

    def test_plug_path_open_to_the_ceiling(self):
        plug = tray.plug_space()
        self.assertFalse(tray.overlaps(self.m, plug))
        self.assertFalse(tray.overlaps(tray.switch_space(), plug))

    def test_stack2_clash_flagged(self):
        t = dict(tray.TRAY, stack2_y=90.0)
        problems = tray.check_parts(t, {"arm-bracket": tray.arm_bracket(t)})
        self.assertTrue(any("arm bracket" in p and "Stack 2" in p for p in problems), problems)

    def test_tight_bore_flagged(self):
        t = dict(tray.TRAY, bore_d=11.8)
        problems = tray.check_parts(t, {"arm-bracket": tray.arm_bracket(t)})
        self.assertTrue(any("feet" in p for p in problems), problems)

    def test_prints_on_its_rear_face(self):
        p = tray.arm_bracket_print()
        b = p.bounding_box()
        self.assertAlmostEqual(b[5] - b[2], 28.0, places=3)
        for x, y in ((-50, 40), (50, 40), (0, 32)):
            self.assertTrue(solid_at(p, x, y, b[2] + 0.15), (x, y))
        for x in (-44, 44):
            for z in (b[2] + 0.15, b[5] - 0.15):
                self.assertFalse(solid_at(p, x, 36, z))


class Output(unittest.TestCase):
    def test_writes_every_stl(self):
        with tempfile.TemporaryDirectory() as d:
            tray.main(d)
            for name in ("tray.stl", "deck.stl", "cradle-test.stl", "sleeve.stl", "riser.stl", "arm-bracket.stl"):
                self.assertGreater(os.path.getsize(os.path.join(d, name)), 1000)

    def test_failed_check_removes_old_stls(self):
        with tempfile.TemporaryDirectory() as d:
            tray.main(d)
            with self.assertRaises(SystemExit):
                tray.main(d, dict(tray.TRAY, foam_t=2.0))
            self.assertEqual(os.listdir(d), [])

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
