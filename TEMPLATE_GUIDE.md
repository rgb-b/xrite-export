# Illustrator Template Guide

Two template files are required — one per step-count variant. Set their paths in **Settings → Templates**.

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
