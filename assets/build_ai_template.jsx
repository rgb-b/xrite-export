// =============================================================================
// build_ai_template.jsx
// One-time ExtendScript builder for Ink Density Tool Illustrator templates.
//
// Run in Illustrator:  File → Scripts → Other Script… → select this file
//
// Produces:
//   template_standard.ai   (14-step layout: placeholders R01…R14)
//   template_extended.ai   (16-step layout: placeholders R01…R16)
// =============================================================================

// ── CONFIG (edit before running) ─────────────────────────────────────────────

var OUTPUT_DIR       = "C:/Users/YourName/Documents/templates/";  // trailing slash required

var PAGE_W           = 842;   // pt — A4 landscape width
var PAGE_H           = 595;   // pt — A4 landscape height
var MARGIN           = 20;    // pt — all four sides
var HEADER_H         = 44;    // pt — height of job-info header block
var LABEL_ROW_H      = 18;    // pt — weight-label row
var COL_HDR_ROW_H    = 14;    // pt — "C  M  Y  K" column-header row
var ROW_H            = 14;    // pt — each data row (density + step rows)
var DATA_COL_W       = 42;    // pt — width of each C/M/Y/K sub-column
var WEIGHT_GAP       = 6;     // pt — horizontal gap between W1/W2/W3 blocks
var FONT_SIZE        = 7;     // pt — data cells
var HEADER_FONT_SIZE = 9;     // pt — job-info header text
var LABEL_FONT_SIZE  = 8;     // pt — weight-label row + column headers

// ── STEP LABEL ARRAYS ────────────────────────────────────────────────────────
// Must match STEP_LABELS_14 / STEP_LABELS_16 in src/core/models.rs

var STEP_LABELS_14 = ["100","95","90","80","70","60","50","40","30","20","10","5","3","1"];
var STEP_LABELS_16 = ["100","95","90","80","70","60","50","40","30","20","10","5","3","1","0.8","0.4"];

// =============================================================================
// HELPERS
// =============================================================================

/** Convert "pt from artboard top" → Illustrator internal y (origin bottom-left, y-up). */
function toIllustratorY(yFromTop) {
    return PAGE_H - yFromTop;
}

/** Resolve font with fallback list. */
function getFont(bold) {
    var names = bold
        ? ["Helvetica-Bold", "Helvetica Neue Bold", "Arial-BoldMT", "Arial Bold"]
        : ["Helvetica", "Helvetica Neue", "ArialMT", "Arial"];
    for (var i = 0; i < names.length; i++) {
        try { return app.textFonts.getByName(names[i]); } catch (e) {}
    }
    return app.textFonts[0];  // system default
}

/** Add a text frame.
 *  x, yFromTop, w, h are all in "pt from artboard top-left" (y increases downward).
 *  Internally converts to Illustrator's bottom-left origin, y-up coordinates. */
function addText(layer, x, yFromTop, w, h, content, fontSize, bold) {
    var tf = layer.textFrames.add();
    var top    = toIllustratorY(yFromTop);
    var bottom = toIllustratorY(yFromTop + h);
    tf.geometricBounds = [top, x, bottom, x + w];  // [top, left, bottom, right] in internal coords
    tf.contents = content;
    var attr = tf.textRange.characterAttributes;
    attr.size = fontSize || FONT_SIZE;
    try { attr.textFont = getFont(bold); } catch (e) {}
    return tf;
}

/** Draw a hairline rectangle (no fill, 0.25pt black stroke).
 *  x, yFromTop, w, h are in "pt from artboard top-left". */
function addRect(layer, x, yFromTop, w, h) {
    var r = layer.pathItems.rectangle(
        toIllustratorY(yFromTop),  // top in Illustrator internal coords
        x, w, h
    );
    r.filled = false;
    r.stroked = true;
    r.strokeWidth = 0.25;
    r.strokeColor = makeGray(0);
    return r;
}

function makeGray(k) {
    var c = new GrayColor();
    c.gray = k;
    return c;
}

// =============================================================================
// CORE BUILDER
// =============================================================================

function buildTemplate(stepLabels, outFileName) {

    var numSteps = stepLabels.length;

    // ── 1. Create document ───────────────────────────────────────────────────
    var docPreset = new DocumentPreset();
    docPreset.width        = PAGE_W;
    docPreset.height       = PAGE_H;
    docPreset.colorMode    = DocumentColorSpace.CMYK;
    docPreset.units        = RulerUnits.Points;
    docPreset.numArtboards = 1;

    var doc = app.documents.addDocument("Print", docPreset);
    // artboardRect: [left, top, right, bottom] in Illustrator internal coords
    doc.artboards[0].artboardRect = [0, PAGE_H, PAGE_W, 0];

    var layer = doc.layers[0];
    layer.name = "Template";

    // ── 2. Layout constants ──────────────────────────────────────────────────
    var contentW = PAGE_W - 2 * MARGIN;   // usable width

    // Each weight block = 4 data columns
    var blockW = 4 * DATA_COL_W;
    // Total weight area = 3 blocks + 2 gaps
    var weightAreaW = 3 * blockW + 2 * WEIGHT_GAP;
    // Step gutter takes the remainder on the left
    var stepColW = contentW - weightAreaW;
    if (stepColW < 20) stepColW = 20;  // enforce minimum

    var x0 = MARGIN;   // left edge of content

    // Weight block X positions (left edge of first data column in each block)
    var wBlockX = [];
    for (var wi = 0; wi < 3; wi++) {
        wBlockX[wi] = x0 + stepColW + wi * (blockW + WEIGHT_GAP);
    }

    // Row Y positions from TOP of artboard (positive downward in our convention)
    var yHeader   = MARGIN;                   // job-info header starts here
    var yLabels   = yHeader + HEADER_H;       // Wn_LABEL row
    var yColHdr   = yLabels + LABEL_ROW_H;   // "C M Y K" row
    var yDensity  = yColHdr + COL_HDR_ROW_H; // density row ("D")
    // step rows: yDensity + ROW_H + i * ROW_H  (i = 0-based)

    // ── 3. Header block ──────────────────────────────────────────────────────
    var hRowH = HEADER_H / 2;

    var custW = Math.floor(contentW * 0.45);
    addText(layer, x0, yHeader, custW, hRowH, "<<CUSTOMER>>", HEADER_FONT_SIZE, true);

    var crsW = Math.floor(contentW * 0.18);
    addText(layer, x0 + custW, yHeader, crsW, hRowH, "<<CRS>>", HEADER_FONT_SIZE, false);

    var dateW = Math.floor(contentW * 0.15);
    addText(layer, x0 + custW + crsW, yHeader, dateW, hRowH, "<<DATE>>", HEADER_FONT_SIZE, false);

    var setW = Math.floor(contentW * 0.10);
    addText(layer, x0 + custW + crsW + dateW, yHeader, setW, hRowH, "<<SET>>", HEADER_FONT_SIZE, false);

    var jobW = contentW - custW - crsW - dateW - setW;
    addText(layer, x0 + custW + crsW + dateW + setW, yHeader, jobW, hRowH, "<<JOB>>", HEADER_FONT_SIZE, false);

    // Row 2: SHAPE
    addText(layer, x0, yHeader + hRowH, contentW, hRowH, "<<SHAPE>>", HEADER_FONT_SIZE, false);

    // ── 4. Per-weight blocks ─────────────────────────────────────────────────
    for (var wn = 1; wn <= 3; wn++) {
        var wx = wBlockX[wn - 1];
        var wTag = "W" + wn;

        // Weight label row
        addText(layer, wx, yLabels, blockW, LABEL_ROW_H, "<<" + wTag + "_LABEL>>", LABEL_FONT_SIZE, true);
        addRect(layer, wx, yLabels, blockW, LABEL_ROW_H);

        // Column header row: C  M  Y  K
        var cols = ["C", "M", "Y", "K"];
        for (var ci = 0; ci < 4; ci++) {
            addText(layer, wx + ci * DATA_COL_W, yColHdr, DATA_COL_W, COL_HDR_ROW_H,
                    cols[ci], LABEL_FONT_SIZE, true);
            addRect(layer, wx + ci * DATA_COL_W, yColHdr, DATA_COL_W, COL_HDR_ROW_H);
        }

        // Density row
        var dcTags = ["DC", "DM", "DY", "DK"];
        for (var ci = 0; ci < 4; ci++) {
            addText(layer, wx + ci * DATA_COL_W, yDensity, DATA_COL_W, ROW_H,
                    "<<" + wTag + "_" + dcTags[ci] + ">>", FONT_SIZE, false);
            addRect(layer, wx + ci * DATA_COL_W, yDensity, DATA_COL_W, ROW_H);
        }

        // Step rows
        for (var ri = 0; ri < numSteps; ri++) {
            var rowTag = ri + 1;
            var rowTagStr = (rowTag < 10 ? "0" : "") + rowTag;
            var yRow = yDensity + ROW_H + ri * ROW_H;

            var chTags = ["C", "M", "Y", "K"];
            for (var ci = 0; ci < 4; ci++) {
                addText(layer, wx + ci * DATA_COL_W, yRow, DATA_COL_W, ROW_H,
                        "<<" + wTag + "_R" + rowTagStr + "_" + chTags[ci] + ">>",
                        FONT_SIZE, false);
                addRect(layer, wx + ci * DATA_COL_W, yRow, DATA_COL_W, ROW_H);
            }
        }
    }

    // ── 5. Step-label gutter (left column) ───────────────────────────────────
    // Density row label
    addText(layer, x0, yDensity, stepColW, ROW_H, "D", FONT_SIZE, true);
    addRect(layer, x0, yDensity, stepColW, ROW_H);

    for (var ri = 0; ri < numSteps; ri++) {
        var yRow = yDensity + ROW_H + ri * ROW_H;
        addText(layer, x0, yRow, stepColW, ROW_H, stepLabels[ri], FONT_SIZE, false);
        addRect(layer, x0, yRow, stepColW, ROW_H);
    }

    // ── 6. Outer border for the data table ───────────────────────────────────
    var tableTop = yLabels;
    var tableH   = LABEL_ROW_H + COL_HDR_ROW_H + ROW_H * (1 + numSteps);
    addRect(layer, x0, tableTop, contentW, tableH);

    // ── 7. Save and close ────────────────────────────────────────────────────
    var outFile = new File(OUTPUT_DIR + outFileName);

    var saveOpts = new IllustratorSaveOptions();
    saveOpts.compatibility         = Compatibility.ILLUSTRATOR24;  // CC 2020+
    saveOpts.saveMultipleArtboards = false;
    saveOpts.compressed            = true;

    doc.saveAs(outFile, saveOpts);
    doc.close(SaveOptions.DONOTSAVECHANGES);

    $.writeln("Saved: " + outFile.fsName);
}

// =============================================================================
// ENTRY POINT
// =============================================================================

// Ensure OUTPUT_DIR exists (create if missing)
var outFolder = new Folder(OUTPUT_DIR);
if (!outFolder.exists) {
    outFolder.create();
    $.writeln("Created output folder: " + OUTPUT_DIR);
}

buildTemplate(STEP_LABELS_14, "template_standard.ai");
buildTemplate(STEP_LABELS_16, "template_extended.ai");

alert("Done!\n\nTwo templates written to:\n" + OUTPUT_DIR +
      "\n\n  template_standard.ai  (14 steps)\n  template_extended.ai  (16 steps)" +
      "\n\nPoint Settings → Templates to these files in the app.");
