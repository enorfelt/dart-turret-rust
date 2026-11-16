import traceback

import adsk.core
import adsk.fusion

W, H = 7.0, 6.8
CORNER = 0.4
HOLES = ((0.4, 0.4), (6.6, 0.4), (0.4, 6.4), (6.6, 6.4))
BOARD_HOLE_D = 0.32

PARAMS = (
    ("stack_gap", "17 mm", "Top of the power board to the bottom of the logic board (stack.GAP)"),
    ("board_t", "1.6 mm", "Board thickness: 1.6 for JLCPCB, 2.0 for a printed board"),
    ("sleeve_od", "5.8 mm", "Limited by U5 on the power board, 3.1 mm from the (66, 4) hole"),
    ("sleeve_bore", "3.5 mm", "M3 clearance, with a little extra for the printed hole shrinking"),
    ("end_chamfer", "0.3 mm", "Outer edge chamfer on both ends, hides elephant's foot"),
)


def vec(x, y, z):
    m = adsk.core.Matrix3D.create()
    m.translation = adsk.core.Vector3D.create(x, y, z)
    return m


def profile_with_loops(sketch, loops):
    for p in sketch.profiles:
        if p.profileLoops.count == loops:
            return p
    raise RuntimeError(f"no profile with {loops} loops in {sketch.name}")


def build_board(root, name, z):
    occ = root.occurrences.addNewComponent(vec(0, 0, z))
    comp = occ.component
    comp.name = name
    sk = comp.sketches.add(comp.xYConstructionPlane)
    sk.name = "Outline"
    sk.sketchCurves.sketchLines.addTwoPointRectangle(adsk.core.Point3D.create(0, 0, 0), adsk.core.Point3D.create(W, H, 0))
    for x, y in HOLES:
        sk.sketchCurves.sketchCircles.addByCenterRadius(adsk.core.Point3D.create(x, y, 0), BOARD_HOLE_D / 2)
    ext = comp.features.extrudeFeatures.addSimple(
        profile_with_loops(sk, 1 + len(HOLES)),
        adsk.core.ValueInput.createByString("board_t"),
        adsk.fusion.FeatureOperations.NewBodyFeatureOperation,
    )
    body = ext.bodies.item(0)
    body.name = name
    corners = adsk.core.ObjectCollection.create()
    for e in body.edges:
        if e.geometry.curveType == adsk.core.Curve3DTypes.Line3DCurveType:
            s, t = e.startVertex.geometry, e.endVertex.geometry
            if abs(s.x - t.x) < 1e-6 and abs(s.y - t.y) < 1e-6:
                corners.add(e)
    fi = comp.features.filletFeatures.createInput()
    fi.edgeSetInputs.addConstantRadiusEdgeSet(corners, adsk.core.ValueInput.createReal(CORNER), True)
    comp.features.filletFeatures.add(fi)
    return occ


def build_sleeve(comp):
    sk = comp.sketches.add(comp.xYConstructionPlane)
    sk.name = "Section"
    origin = adsk.core.Point3D.create(0, 0, 0)
    circles = sk.sketchCurves.sketchCircles
    dims = sk.sketchDimensions
    for r, expr, at in ((0.29, "sleeve_od", 0.5), (0.175, "sleeve_bore", 0.3)):
        c = circles.addByCenterRadius(origin, r)
        sk.geometricConstraints.addCoincident(c.centerSketchPoint, sk.originPoint)
        dims.addDiameterDimension(c, adsk.core.Point3D.create(at, at, 0)).parameter.expression = expr
    ext = comp.features.extrudeFeatures.addSimple(
        profile_with_loops(sk, 2),
        adsk.core.ValueInput.createByString("stack_gap"),
        adsk.fusion.FeatureOperations.NewBodyFeatureOperation,
    )
    body = ext.bodies.item(0)
    body.name = "Sleeve"
    outer = adsk.core.ObjectCollection.create()
    largest = max(e.geometry.radius for e in body.edges if e.geometry.curveType == adsk.core.Curve3DTypes.Circle3DCurveType)
    for e in body.edges:
        g = e.geometry
        if g.curveType == adsk.core.Curve3DTypes.Circle3DCurveType and abs(g.radius - largest) < 1e-6:
            outer.add(e)
    ci = comp.features.chamferFeatures.createInput2()
    ci.chamferEdgeSets.addEqualDistanceChamferEdgeSet(outer, adsk.core.ValueInput.createByString("end_chamfer"), False)
    comp.features.chamferFeatures.add(ci)


def run(context):
    app = adsk.core.Application.get()
    ui = app.userInterface
    try:
        app.documents.add(adsk.core.DocumentTypes.FusionDesignDocumentType)
        design = adsk.fusion.Design.cast(app.activeProduct)
        design.designType = adsk.fusion.DesignTypes.ParametricDesignType
        design.fusionUnitsManager.distanceDisplayUnits = adsk.fusion.DistanceUnits.MillimeterDistanceUnits
        for name, expr, comment in PARAMS:
            design.userParameters.add(name, adsk.core.ValueInput.createByString(expr), "mm", comment)
        root = design.rootComponent
        root.name = "Stack 2"

        board_t = design.userParameters.itemByName("board_t").value
        gap = design.userParameters.itemByName("stack_gap").value
        build_board(root, "Power board", 0)
        build_board(root, "Logic board", board_t + gap)

        first = root.occurrences.addNewComponent(vec(HOLES[0][0], HOLES[0][1], board_t))
        sleeve = first.component
        sleeve.name = "Sleeve"
        build_sleeve(sleeve)
        for x, y in HOLES[1:]:
            root.occurrences.addExistingComponent(sleeve, vec(x, y, board_t))

        app.activeViewport.fit()
        ui.messageBox(
            "Stack 2 built. Change the sizes under Modify > Change Parameters.\n\n"
            "Export: right-click the Sleeve component > Save As Mesh (STL), print 4 upright."
        )
    except Exception:
        ui.messageBox("Failed:\n{}".format(traceback.format_exc()))
