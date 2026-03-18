# Illustrator Template Guide

Two template files are required — one per step-count variant. Set their paths in **Settings → Templates**.

---

## Building the templates from scratch

If you need to regenerate the `.ai` template files (e.g. first-time setup, layout change, new
Illustrator version), use the included builder script rather than hand-crafting the files.

### Prerequisites

- Adobe Illustrator CC 2020 or later (ExtendScript engine required)
- The Helvetica font family installed (or edit `FONT_FAMILY` in the script to use another)

### Steps

1. Open `assets/build_ai_template.jsx` in a text editor.
2. Edit the `OUTPUT_DIR` variable at the top to point to the folder where you want the `.ai`
   files saved — for example:
   ```
   var OUTPUT_DIR = "C:/Users/YourName/Documents/InkDensityTemplates/";
   ```
   The trailing slash is required. The folder will be created automatically if it does not exist.
3. Optionally tweak the layout constants (`ROW_H`, `DATA_COL_W`, font sizes, etc.) to match
   your house style.
4. In Illustrator: **File → Scripts → Other Script…** → select `build_ai_template.jsx`.
5. The script runs silently, then shows an alert confirming the two files have been written:
   - `template_standard.ai` — 14-step layout (placeholders `R01`…`R14`)
   - `template_extended.ai` — 16-step layout (placeholders `R01`…`R16`)
6. In the app: **Settings → Templates** → set the Standard and Extended paths to the newly
   created files.

### Layout produced by the builder

```
┌─────────────────────────────────────────────────────────────────────────┐
│ <<CUSTOMER>>              <<CRS>>         <<DATE>>  <<SET>>  <<JOB>>    │  ← header row 1
│ <<SHAPE>>                                                               │  ← header row 2
├─────────┬──────────────────────────┬──────────────────────────┬─────────┤
│         │  <<W1_LABEL>>            │  <<W2_LABEL>>            │ <<W3…>> │  ← weight labels
│         │   C     M     Y     K    │   C     M     Y     K    │ C M Y K │  ← col headers
│   D     │ <<W1_DC>> … <<W1_DK>>   │ <<W2_DC>> … <<W2_DK>>   │  …      │  ← max density
│  100    │ <<W1_R01_C>> … _R01_K>> │ <<W2_R01_C>> …          │  …      │
│   95    │ <<W1_R02_C>> …          │  …                       │  …      │
│   …     │  …                      │  …                       │  …      │
│    1    │ <<W1_R14_C>> … _R14_K>> │  …                       │  …      │  ← last row (14-step)
└─────────┴──────────────────────────┴──────────────────────────┴─────────┘
```

Each weight block has exactly four data columns (C / M / Y / K) — no average or calculated
columns. Unused W2/W3 slots receive empty strings from the export engine and appear blank.

---

| Setting | Template type |
|---|---|
| Standard (14 steps) | Used when the job has 14 step rows (100 → 1) |
| Extended (16 steps) | Used when the job has 16 step rows (100 → 0.4) |

Each template must contain W1/W2/W3 slots. Unused slots receive empty strings and will appear blank.

---

## Job-level placeholders

These are the same on every exported page.

| Placeholder | Value |
|---|---|
| `<<CUSTOMER>>` | Customer, Print Type, Stock Desc, and Finish joined with spaces |
| `<<CRS>>` | Dot Shape Type + Dot Shape Number (e.g. `CRS 01`) |
| `<<DATE>>` | Date field |
| `<<SET>>` | `Set {number}`, or blank if the field is empty |
| `<<JOB>>` | `Job {number}`, or blank if the field is empty |
| `<<SHAPE>>` | Shape name (e.g. `HD 16`) |
| `<<STOCK>>` | Always blank (reserved for future use) |

---

## Per-weight placeholders

Repeated for `W1`, `W2`, and `W3` — replace `n` with `1`, `2`, or `3`.

### Label and max density

| Placeholder | Value |
|---|---|
| `<<Wn_LABEL>>` | Weight label (e.g. `120#`) |
| `<<Wn_DC>>` | Max density — Cyan |
| `<<Wn_DM>>` | Max density — Magenta |
| `<<Wn_DY>>` | Max density — Yellow |
| `<<Wn_DK>>` | Max density — Black |

### Step rows

One placeholder per step row per colour channel. Row numbers are zero-padded.

| Placeholder | Value |
|---|---|
| `<<Wn_R01_C>>` … `<<Wn_R14_C>>` | Step rows 1–14, Cyan |
| `<<Wn_R01_M>>` … `<<Wn_R14_M>>` | Step rows 1–14, Magenta |
| `<<Wn_R01_Y>>` … `<<Wn_R14_Y>>` | Step rows 1–14, Yellow |
| `<<Wn_R01_K>>` … `<<Wn_R14_K>>` | Step rows 1–14, Black |

The extended template additionally uses rows `R15` and `R16` for all four channels.

---

## Behaviours to know

- **Unused weight slots** — if a page has only 1 or 2 weights, all `W2`/`W3` placeholders emit empty strings. The text frames will appear blank.
- **Zero values** — rendered as blank, not `0.00`.
- **Chunking** — weights are grouped in pages of up to 3. A job with 5 weights produces two pages (3 + 2), each using the same template.
- The template is never saved — it is always opened as a copy.
