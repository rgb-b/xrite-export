/**
 * runner.jsx — Ink Density Tool ExtendScript template
 *
 * Tokens replaced by the Rust binary:
 *   <<REPLACEMENTS_DICT>>  — injected as a complete JSX object literal
 *   <<TEMPLATE_PATH>>      — absolute path to the master .ai template
 *   <<OUTPUT_PDF>>         — absolute path where the PDF should be saved
 */

#target illustrator

(function () {

    var PDF_PRESETS = [
        "[PDF/X-4:2008]",
        "[PDF/X-1a:2001]",
        "[High Quality Print]",
        "[Press Quality]"
    ];

    function exportPDF(doc, pdfFile) {
        for (var i = 0; i < PDF_PRESETS.length; i++) {
            try {
                var opts = new PDFSaveOptions();
                opts.pDFPreset = PDF_PRESETS[i];
                opts.useArtboardFrame = true;
                doc.saveAs(pdfFile, opts);
                return;  // success
            } catch (e) {}
        }
        // Last resort: default PDF options
        var opts = new PDFSaveOptions();
        opts.useArtboardFrame = true;
        doc.saveAs(pdfFile, opts);
    }

    var doc;
    try {

        // 1. Open the master template as a new document (non-destructive)
        var templateFile = new File("<<TEMPLATE_PATH>>");
        if (!templateFile.exists) {
            throw new Error("Template file not found: <<TEMPLATE_PATH>>");
        }

        doc = app.open(templateFile);

        // 2. Replacement dictionary — generated and injected by Rust.
        var replacements = <<REPLACEMENTS_DICT>>;

        // 3. Walk all text frames and replace placeholder text
        var items = doc.textFrames;
        for (var i = 0; i < items.length; i++) {
            var tf = items[i];
            var content = tf.contents;
            for (var key in replacements) {
                if (replacements.hasOwnProperty(key)) {
                    if (content.indexOf(key) !== -1) {
                        tf.contents = content.split(key).join(replacements[key]);
                        content = tf.contents;
                    }
                }
            }
        }

        // 4. Export as PDF (with preset fallback)
        var pdfFile = new File("<<OUTPUT_PDF>>");
        exportPDF(doc, pdfFile);

        // 5. Close without saving the .ai document
        doc.close(SaveOptions.DONOTSAVECHANGES);

    } catch (e) {
        if (doc) {
            try { doc.close(SaveOptions.DONOTSAVECHANGES); } catch (closeErr) {}
        }
        throw new Error("runner.jsx failed: " + e.message);
    }

})();
