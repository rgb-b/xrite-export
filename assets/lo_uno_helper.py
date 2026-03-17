#!/usr/bin/env python3
"""Minimal UNO bridge helper — spawned by the Rust binary.

Usage:
    python3 lo_uno_helper.py <lo_exe> <template_path> <placeholders_json_path> <output_pdf>

Starts a headless LibreOffice, opens the template as a copy,
replaces all <<PLACEHOLDER>> text, and exports to PDF.
"""
import json
import os
import subprocess
import sys
import time

LO_PORT = 2002
LO_RETRIES = 12
LO_WAIT = 1.0


def main():
    if len(sys.argv) != 5:
        print(f"Usage: {sys.argv[0]} <lo_exe> <template> <placeholders_json> <output_pdf>",
              file=sys.stderr)
        sys.exit(1)

    lo_exe, template_path, json_path, out_pdf = sys.argv[1:5]

    with open(json_path, 'r') as f:
        placeholders = json.load(f)

    # Start headless LibreOffice
    proc = subprocess.Popen(
        [lo_exe, "--headless", "--norestore", "--nofirststartwizard",
         f"--accept=socket,host=localhost,port={LO_PORT};urp;StarOffice.ServiceManager"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )

    try:
        desktop = connect()
        process_template(desktop, template_path, placeholders, out_pdf)
    finally:
        proc.terminate()
        proc.wait(timeout=10)


def connect():
    import uno
    local_ctx = uno.getComponentContext()
    resolver = local_ctx.ServiceManager.createInstanceWithContext(
        "com.sun.star.bridge.UnoUrlResolver", local_ctx)
    url = f"uno:socket,host=localhost,port={LO_PORT};urp;StarOffice.ComponentContext"
    for _ in range(LO_RETRIES):
        try:
            ctx = resolver.resolve(url)
            smgr = ctx.ServiceManager
            return smgr.createInstanceWithContext("com.sun.star.frame.Desktop", ctx)
        except Exception:
            time.sleep(LO_WAIT)
    raise RuntimeError("Could not connect to LibreOffice")


def process_template(desktop, template_path, placeholders, out_pdf):
    import uno
    from com.sun.star.beans import PropertyValue

    def prop(name, value):
        p = PropertyValue()
        p.Name = name
        p.Value = value
        return p

    template_url = uno.systemPathToFileUrl(os.path.abspath(template_path))
    doc = desktop.loadComponentFromURL(
        template_url, "_blank", 0,
        (prop("Hidden", True), prop("AsTemplate", True)))

    try:
        # Find & Replace (preserves formatting)
        search = doc.createSearchDescriptor()
        search.SearchRegularExpression = False
        for key, value in placeholders.items():
            if not key:
                continue
            search.SearchString = key
            search.ReplaceString = value
            doc.replaceAll(search)

        # Belt-and-braces: walk draw shapes recursively
        pages = doc.DrawPages
        for pi in range(pages.Count):
            replace_in_container(pages.getByIndex(pi), placeholders)

        # Export PDF
        out_url = uno.systemPathToFileUrl(os.path.abspath(out_pdf))
        doc.storeToURL(out_url, (prop("FilterName", "draw_pdf_Export"),))
    finally:
        doc.close(False)


def replace_in_container(container, placeholders):
    for i in range(container.Count):
        shape = container.getByIndex(i)
        if hasattr(shape, "Count"):
            replace_in_container(shape, placeholders)
        if shape.supportsService("com.sun.star.drawing.Text"):
            text_obj = shape.getText()
            content = text_obj.getString()
            new_content = content
            for key, value in placeholders.items():
                if key:
                    new_content = new_content.replace(key, value)
            if new_content != content:
                text_obj.setString(new_content)


if __name__ == "__main__":
    main()
