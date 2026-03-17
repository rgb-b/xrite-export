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

    // 1. Open the master template as a new document (non-destructive)
    var templateFile = new File("<<TEMPLATE_PATH>>");
    if (!templateFile.exists) {
        throw new Error("Template file not found: <<TEMPLATE_PATH>>");
    }

    var doc = app.open(templateFile);

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

    // 4. Export as PDF
    var pdfFile = new File("<<OUTPUT_PDF>>");
    var pdfOptions = new PDFSaveOptions();
    pdfOptions.pDFPreset = "[PDF/X-4:2008]";
    pdfOptions.useArtboardFrame = true;

    doc.saveAs(pdfFile, pdfOptions);

    // 5. Close without saving the .ai document
    doc.close(SaveOptions.DONOTSAVECHANGES);

})();
