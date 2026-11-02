import math

import pytest

from shotpilot import geometry as g


def test_look_at_yaw_pitch():
    r = g.look_at((0, 0, 0), (100, 100, 0))
    assert r.yaw == pytest.approx(45)
    assert r.pitch == pytest.approx(0)
    r = g.look_at((0, 0, 0), (100, 0, 100))
    assert r.pitch == pytest.approx(45)


def test_basis_is_ue_convention():
    r = g.Rot(0, 0, 0)
    assert g.forward(r) == pytest.approx((1, 0, 0))
    assert g.right(r) == pytest.approx((0, 1, 0))
    assert g.up(r) == pytest.approx((0, 0, 1))


def test_project_center_and_sides():
    cam = g.Xform((0, 0, 0), g.Rot(0, 0, 0))
    cx, cy, depth = g.project((1000, 0, 0), cam, 35, 320, 180)
    assert (cx, cy, depth) == pytest.approx((160, 90, 1000))
    x_right, _, _ = g.project((1000, 100, 0), cam, 35, 320, 180)
    _, y_up, _ = g.project((1000, 0, 100), cam, 35, 320, 180)
    assert x_right > 160  # +Y is screen right
    assert y_up < 90      # +Z is screen up
    assert g.project((-10, 0, 0), cam, 35, 320, 180) is None


def test_longer_lens_magnifies():
    cam = g.Xform((0, 0, 0), g.Rot(0, 0, 0))
    wide = g.project((1000, 100, 0), cam, 24, 320, 180)[0] - 160
    tele = g.project((1000, 100, 0), cam, 85, 320, 180)[0] - 160
    assert tele / wide == pytest.approx(85 / 24, rel=1e-3)


def test_orbit_keeps_radius():
    e = g.orbit((100, 0, 50), (0, 0, 0), 90)
    assert e == pytest.approx((0, 100, 50))
    assert math.hypot(e[0], e[1]) == pytest.approx(100)


def test_smoothstep_ends():
    assert g.ease(0, "ease_in_out") == 0 and g.ease(1, "ease_in_out") == 1
    assert g.ease(0.5, "ease_in_out") == pytest.approx(0.5)
