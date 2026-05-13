//! HTML report generator — print-ready A4 portrait.
//!
//! Columns are auto-sized to content (narrow + tall, not wide + squat).
//! Spot ink headers match Pantone catalogue colours when the name contains
//! a recognisable PMS number.

use crate::core::models::{InkKind, JobConfig, ShapeData};
use crate::core::targets::interpolate_target;

// ── Public entry point ────────────────────────────────────────────────────────

pub fn generate_report(job: &JobConfig) -> String {
    let title = {
        let h = job.heading();
        if h.is_empty() { "Ink Density Report".to_string() } else { h }
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>{css}</style>
</head>
<body>
<div class="no-print">
  <button onclick="window.print()">Print / Save PDF</button>
  <button onclick="window.close()">Close</button>
</div>
{header}
{body}
</body>
</html>"#,
        title  = esc(&title),
        css    = CSS,
        header = build_header(job),
        body   = build_body(job),
    )
}

// ── Header ────────────────────────────────────────────────────────────────────

fn build_header(job: &JobConfig) -> String {
    let customer = if !job.customer.is_empty() {
        format!(r#"<div class="customer">{}</div>"#, esc(&job.customer))
    } else { String::new() };

    let job_name = if !job.job_name.is_empty() {
        format!(r#"<div class="job-name">{}</div>"#, esc(&job.job_name))
    } else { String::new() };

    let spec_tags: String = [
        job.plate_tech.as_str(),
        job.press_system.as_str(),
        job.esxr_number.as_str(),
        job.print_type.as_str(),
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .map(|s| format!(r#"<span class="spec-tag">{}</span>"#, esc(s)))
    .collect::<Vec<_>>()
    .join(" ");

    let specs = if !spec_tags.is_empty() {
        format!(r#"<div class="spec-line">{spec_tags}</div>"#)
    } else { String::new() };

    let right_items: String = [
        ("JOB",  job.job_number.as_str()),
        ("DATE", job.date.as_str()),
        ("SET",  job.set_number.as_str()),
    ]
    .iter()
    .filter(|(_, v)| !v.is_empty())
    .map(|(lbl, val)| format!(
        r#"<div class="detail-item"><span class="detail-label">{lbl}</span><span class="detail-value">{val}</span></div>"#,
        val = esc(val),
    ))
    .collect();

    let right = if right_items.is_empty() { String::new() } else {
        format!(r#"<div class="header-right">{right_items}</div>"#)
    };

    format!(
        r#"<header class="report-header"><div class="accent-bar"></div><div class="header-inner"><div class="header-left">{customer}{job_name}{specs}</div>{right}</div></header>"#
    )
}

// ── Body ──────────────────────────────────────────────────────────────────────

fn build_body(job: &JobConfig) -> String {
    if job.shapes.is_empty() {
        return r#"<p class="empty-note">No data recorded.</p>"#.to_string();
    }
    job.shapes.iter()
        .map(|shape| build_shape_section(job, shape))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_shape_section(job: &JobConfig, shape: &ShapeData) -> String {
    let name  = shape.display_name();
    let table = build_shape_table(job, shape);
    format!(
        r#"<section class="shape-section">
<div class="shape-heading"><span class="shape-label">{name}</span></div>
{table}
</section>"#,
        name  = esc(&name),
        table = table,
    )
}

// ── Shape table ───────────────────────────────────────────────────────────────

fn build_shape_table(job: &JobConfig, shape: &ShapeData) -> String {
    if shape.weights.is_empty() {
        return r#"<p class="empty-note">No LPI data.</p>"#.to_string();
    }

    let num_inks     = job.num_inks();
    let dev_indices  = job.deviation_ink_indices();
    let show_avg_dev = !dev_indices.is_empty();
    let cols_per_lpi = num_inks + if show_avg_dev { 2 } else { 0 };
    let num_lpis     = shape.weights.len();

    let mut h = String::from(r#"<table class="data-table"><thead>"#);

    // ── Row 1: Step | LPI group labels… | Target ─────────────────────────────
    h.push_str("<tr>");
    h.push_str(r#"<th rowspan="2" class="th-corner">Step</th>"#);
    for (wi, weight) in shape.weights.iter().enumerate() {
        let last  = wi == num_lpis - 1;
        let extra = if last { "" } else { " lpi-last" };
        h.push_str(&format!(
            r#"<th colspan="{cols}" class="th-lpi-group{extra}">{lpi}</th>"#,
            cols  = cols_per_lpi,
            extra = extra,
            lpi   = esc(&weight.lpi),
        ));
    }
    h.push_str(r#"<th rowspan="2" class="th-target">Target</th></tr>"#);

    // ── Row 2: Ink names (with colours) ──────────────────────────────────────
    h.push_str("<tr>");
    for (wi, _) in shape.weights.iter().enumerate() {
        let last_lpi = wi == num_lpis - 1;
        for (ci, ink) in job.inks.iter().enumerate() {
            let last_ink = ci == num_inks - 1 && !show_avg_dev;
            let boundary = !last_lpi && last_ink;
            let base_cls = ink_class(&ink.kind);
            let extra    = if boundary { " lpi-last" } else { "" };
            let style    = ink_header_style(&ink.kind, &ink.name);
            h.push_str(&format!(
                r#"<th class="th-ink {base_cls}{extra}"{style}>{name}</th>"#,
                name  = esc(&ink.name),
                style = style,
            ));
        }
        if show_avg_dev {
            let b = if !last_lpi { " lpi-last" } else { "" };
            h.push_str(r#"<th class="th-avg">Avg</th>"#);
            h.push_str(&format!(r#"<th class="th-dev{b}">Dev</th>"#));
        }
    }
    h.push_str("</tr></thead><tbody>");

    // ── Density row ───────────────────────────────────────────────────────────
    h.push_str(r#"<tr class="row-density"><td class="td-step">D</td>"#);
    for (wi, weight) in shape.weights.iter().enumerate() {
        let last_lpi = wi == num_lpis - 1;
        for (ci, _) in job.inks.iter().enumerate() {
            let last_ink = ci == num_inks - 1 && !show_avg_dev;
            let b = if !last_lpi && last_ink { " lpi-last" } else { "" };
            let v = weight.density.get(ci).copied().unwrap_or(0.0);
            h.push_str(&format!(r#"<td class="td-data{b}">{}</td>"#, fmt_val(v, 2)));
        }
        if show_avg_dev {
            let b = if !last_lpi { " lpi-last" } else { "" };
            h.push_str(r#"<td class="td-avg"></td>"#);
            h.push_str(&format!(r#"<td class="td-dev{b}"></td>"#));
        }
    }
    h.push_str(r#"<td class="td-target"></td></tr>"#);

    // ── Step rows ─────────────────────────────────────────────────────────────
    for (si, label) in job.step_labels.iter().enumerate() {
        h.push_str("<tr>");
        h.push_str(&format!(r#"<td class="td-step">{label}%</td>"#, label = esc(label)));

        let is_hundred = label == "100";

        for (wi, weight) in shape.weights.iter().enumerate() {
            let last_lpi = wi == num_lpis - 1;
            let row_values: Vec<f64> = if is_hundred {
                vec![100.0; num_inks]
            } else {
                let mut v = weight.steps.get(si).cloned().unwrap_or_default();
                v.resize(num_inks, 0.0);
                v
            };

            for (ci, _) in job.inks.iter().enumerate() {
                let last_ink = ci == num_inks - 1 && !show_avg_dev;
                let b = if !last_lpi && last_ink { " lpi-last" } else { "" };
                let v = row_values.get(ci).copied().unwrap_or(0.0);
                h.push_str(&format!(r#"<td class="td-data{b}">{}</td>"#, fmt_val(v, 1)));
            }

            if show_avg_dev {
                let b = if !last_lpi { " lpi-last" } else { "" };
                let avg = {
                    let s: f64 = dev_indices.iter()
                        .map(|&i| row_values.get(i).copied().unwrap_or(0.0))
                        .sum();
                    s / dev_indices.len() as f64
                };
                h.push_str(&format!(r#"<td class="td-avg">{}</td>"#, fmt_val(avg, 1)));

                let dev_str = label.parse::<f64>().ok()
                    .and_then(|s| interpolate_target(s))
                    .map(|t| {
                        let d = avg - t;
                        if d.abs() < 0.05 { "0".to_string() }
                        else { format!("{:+.1}", d) }
                    })
                    .unwrap_or_default();
                h.push_str(&format!(r#"<td class="td-dev{b}">{dev_str}</td>"#));
            }
        }

        let target = label.parse::<f64>().ok()
            .and_then(|s| interpolate_target(s))
            .map(|t| format!("{:.1}", t))
            .unwrap_or_default();
        h.push_str(&format!(r#"<td class="td-target">{target}</td>"#));
        h.push_str("</tr>");
    }

    h.push_str("</tbody></table>");
    h
}

// ── Ink colour helpers ────────────────────────────────────────────────────────

fn ink_class(kind: &InkKind) -> &'static str {
    match kind {
        InkKind::Cyan    => "ink-c",
        InkKind::Magenta => "ink-m",
        InkKind::Yellow  => "ink-y",
        InkKind::Black   => "ink-k",
        InkKind::White   => "ink-w",
        InkKind::Spot    => "ink-spot",
    }
}

/// For spot inks, try to match the name against the Pantone catalogue and
/// return an inline style with the matching background colour.
fn ink_header_style(kind: &InkKind, name: &str) -> String {
    if *kind != InkKind::Spot {
        return String::new();
    }
    if let Some(hex) = pantone_hex(name) {
        let dark = is_light(hex);
        let fg = if dark { "#1a1a1a" } else { "#ffffff" };
        format!(r#" style="background-color:#{hex};color:{fg};""#)
    } else {
        String::new()
    }
}

/// Parse a PMS/Pantone number from an ink name and look it up in the table.
fn pantone_hex(name: &str) -> Option<&'static str> {
    let s = name.trim();
    // Strip common prefixes
    let s = s.strip_prefix("Pantone® ").unwrap_or(s);
    let s = s.strip_prefix("Pantone ").unwrap_or(s);
    let s = s.strip_prefix("PMS ").unwrap_or(s);
    let s = s.strip_prefix("pms ").unwrap_or(s);
    // Strip common suffixes
    let s = s.strip_suffix(" CP").unwrap_or(s);
    let s = s.strip_suffix(" UP").unwrap_or(s);
    let s = s.strip_suffix(" C").unwrap_or(s);
    let s = s.strip_suffix(" U").unwrap_or(s);
    let s = s.trim();

    PANTONE.iter().find(|(k, _)| *k == s).map(|(_, v)| *v)
}

/// Returns true if the hex colour is light enough to need dark text.
fn is_light(hex: &str) -> bool {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32;
    0.299 * r + 0.587 * g + 0.114 * b > 140.0
}

// ── Formatting ────────────────────────────────────────────────────────────────

fn fmt_val(v: f64, dp: usize) -> String {
    if v == 0.0 { String::new() } else { format!("{:.prec$}", v, prec = dp) }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Pantone Coated catalogue ──────────────────────────────────────────────────
// Approximate RGB values for Pantone Coated (C) colours.
// Source: Pantone Color Bridge Coated approximations.

static PANTONE: &[(&str, &str)] = &[
    // ── Yellows ──────────────────────────────────────────────────────────────
    ("100",  "F4ED7C"), ("101",  "F4ED47"), ("102",  "F9E526"), ("103",  "C6AD0F"),
    ("104",  "A99200"), ("105",  "897A0B"), ("106",  "F6E619"), ("107",  "F5E012"),
    ("108",  "F4D900"), ("109",  "F2CC00"), ("110",  "E0B800"), ("111",  "C8A400"),
    ("112",  "B59500"), ("113",  "F5E642"), ("114",  "F5E13C"), ("115",  "F5DC3C"),
    ("116",  "FFCD00"), ("117",  "D89A00"), ("118",  "B88200"), ("119",  "8C6400"),
    // ── Yellow-Orange ─────────────────────────────────────────────────────────
    ("120",  "FAE171"), ("121",  "FAD95A"), ("122",  "FAD048"), ("123",  "FFC72C"),
    ("124",  "EAA900"), ("125",  "C48C00"), ("126",  "A07400"), ("127",  "F5E27A"),
    ("128",  "F5D85C"), ("129",  "F5CB3C"), ("130",  "F5A800"), ("131",  "D48900"),
    ("132",  "A87000"), ("133",  "785200"), ("134",  "FADE8C"), ("135",  "FAD274"),
    ("136",  "FAC257"), ("137",  "F5A623"), ("138",  "D48800"), ("139",  "A07200"),
    // ── Gold / Warm-Orange ────────────────────────────────────────────────────
    ("140",  "7A5500"), ("141",  "F5DA8C"), ("142",  "F5CC70"), ("143",  "F5BC50"),
    ("144",  "F08300"), ("145",  "CF6F00"), ("146",  "A15B00"), ("147",  "704800"),
    ("148",  "FAD9A8"), ("149",  "FAC880"), ("150",  "FAB25A"), ("151",  "FA7A14"),
    ("152",  "D96D00"), ("153",  "B35A00"), ("154",  "8C4700"), ("155",  "FADE9E"),
    ("156",  "FAC87E"), ("157",  "FAB55C"), ("158",  "FA6816"), ("159",  "C64B00"),
    // ── Red-Orange ───────────────────────────────────────────────────────────
    ("160",  "7D3D00"), ("161",  "5A2D0C"), ("162",  "FABE96"), ("163",  "FA9E6C"),
    ("164",  "FA8040"), ("165",  "FF6900"), ("166",  "E05C00"), ("167",  "B04500"),
    ("168",  "7A2E0A"), ("169",  "F4B8A0"), ("170",  "F49678"), ("171",  "F46E45"),
    ("172",  "F25C28"), ("173",  "CF4520"), ("174",  "99391A"), ("175",  "7A2C15"),
    ("176",  "F2B8AE"), ("177",  "F28C82"), ("178",  "F26859"), ("179",  "EF3A20"),
    // ── Red ──────────────────────────────────────────────────────────────────
    ("180",  "CF3520"), ("181",  "A02018"), ("182",  "F5B8B8"), ("183",  "F59090"),
    ("184",  "F56878"), ("185",  "EF3340"), ("186",  "CF2A2A"), ("187",  "A82020"),
    ("188",  "7A1818"), ("189",  "F5A8B8"), ("190",  "F580A8"), ("191",  "F560A0"),
    ("192",  "F02057"), ("193",  "C51547"), ("194",  "A01040"), ("195",  "7A0C30"),
    ("196",  "F5B8C8"), ("197",  "F590B0"), ("198",  "F56895"), ("199",  "EF2060"),
    // ── Red-Pink ─────────────────────────────────────────────────────────────
    ("200",  "CC2244"), ("201",  "A01830"), ("202",  "7A1422"), ("203",  "F5A8C8"),
    ("204",  "F580B0"), ("205",  "F05895"), ("206",  "EE1163"), ("207",  "C4004B"),
    ("208",  "9A003A"), ("209",  "7A002B"), ("210",  "F5A0CC"), ("211",  "F078B4"),
    ("212",  "EA5094"), ("213",  "E8006B"), ("214",  "C4005A"), ("215",  "9A0048"),
    ("216",  "7A0038"), ("217",  "F0A8D8"), ("218",  "E87CC0"), ("219",  "E050A0"),
    // ── Magenta ──────────────────────────────────────────────────────────────
    ("220",  "CC3C8C"), ("221",  "A22874"), ("222",  "7A1858"), ("223",  "ECA0D0"),
    ("224",  "E878BC"), ("225",  "E050A8"), ("226",  "DE008C"), ("227",  "BA0076"),
    ("228",  "940060"), ("229",  "7A004E"), ("230",  "F0A8E0"), ("231",  "EC80CC"),
    ("232",  "E850B8"), ("233",  "D200A0"), ("234",  "B00086"), ("235",  "90006E"),
    ("236",  "F4B4E8"), ("237",  "F090D8"), ("238",  "EC68C8"), ("239",  "E848B8"),
    // ── Magenta-Purple ────────────────────────────────────────────────────────
    ("240",  "D428A0"), ("241",  "B01C8C"), ("242",  "8C1474"), ("243",  "EFB0E0"),
    ("244",  "EC90D0"), ("245",  "E868C0"), ("246",  "D500A0"), ("247",  "B40088"),
    ("248",  "950072"), ("249",  "780060"), ("250",  "F0B8E8"), ("251",  "EE98DC"),
    ("252",  "EC74CC"), ("253",  "CC44B0"), ("254",  "A42898"), ("255",  "841480"),
    ("256",  "DAAAE0"), ("257",  "D090D4"), ("258",  "B86AB8"), ("259",  "8C3A96"),
    // ── Purple-Violet ─────────────────────────────────────────────────────────
    ("260",  "743084"), ("261",  "622873"), ("262",  "4E2060"), ("263",  "D4B8E0"),
    ("264",  "C4A0D4"), ("265",  "AC7EC0"), ("266",  "9054A8"), ("267",  "7B3798"),
    ("268",  "642880"), ("269",  "4E2069"), ("270",  "C0B0D8"), ("271",  "B098CC"),
    ("272",  "9C80BC"), ("273",  "7050A0"), ("274",  "582878"), ("275",  "401860"),
    ("276",  "9898CC"), ("277",  "A0C0E0"), ("278",  "90B4DC"), ("279",  "6094CC"),
    // ── Blue ─────────────────────────────────────────────────────────────────
    ("280",  "1B3A8A"), ("281",  "1B327A"), ("282",  "152860"), ("283",  "8CB8E0"),
    ("284",  "70A8D8"), ("285",  "4898CE"), ("286",  "0033A0"), ("287",  "003087"),
    ("288",  "002D72"), ("289",  "001A4B"), ("290",  "B8D8EC"), ("291",  "9CCCEC"),
    ("292",  "80BCEC"), ("293",  "0065BD"), ("294",  "00529A"), ("295",  "003878"),
    ("296",  "002055"), ("297",  "A0D0EC"), ("298",  "80C8EC"), ("299",  "60B8E8"),
    // ── Blue-Cyan ─────────────────────────────────────────────────────────────
    ("300",  "0057A8"), ("301",  "004B87"), ("302",  "003865"), ("303",  "002848"),
    ("304",  "A0D8E8"), ("305",  "78CCEC"), ("306",  "50C0ED"), ("307",  "0081A8"),
    ("308",  "006C90"), ("309",  "004860"), ("310",  "88D8EC"), ("311",  "60D0EC"),
    ("312",  "00B0D8"), ("313",  "0096C8"), ("314",  "0082A8"), ("315",  "006889"),
    ("316",  "004B62"), ("317",  "C0E8EC"), ("318",  "A0E0EC"), ("319",  "70D4EC"),
    // ── Cyan-Teal ─────────────────────────────────────────────────────────────
    ("320",  "009B9E"), ("321",  "008690"), ("322",  "006D76"), ("323",  "005A62"),
    ("324",  "A8E0DC"), ("325",  "80D8D4"), ("326",  "50CCCA"), ("327",  "009490"),
    ("328",  "007B78"), ("329",  "006462"), ("330",  "C0E8E0"), ("331",  "A0E0D8"),
    ("332",  "80D8CC"), ("333",  "00B4A8"), ("334",  "008A80"), ("335",  "007068"),
    ("336",  "006058"), ("337",  "B8E8D8"), ("338",  "98E0CC"), ("339",  "70D4BC"),
    // ── Green ────────────────────────────────────────────────────────────────
    ("340",  "00A880"), ("341",  "008C68"), ("342",  "007258"), ("343",  "005E48"),
    ("344",  "B4E0C0"), ("345",  "98D8AC"), ("346",  "78CC98"), ("347",  "00843D"),
    ("348",  "006747"), ("349",  "005030"), ("350",  "003820"), ("351",  "C0E8C4"),
    ("352",  "A8E0B0"), ("353",  "88D89A"), ("354",  "00A85A"), ("355",  "009A44"),
    ("356",  "007A3D"), ("357",  "005830"), ("358",  "C8E8A8"), ("359",  "B8E494"),
    // ── Yellow-Green ─────────────────────────────────────────────────────────
    ("360",  "A0DC78"), ("361",  "78CA40"), ("362",  "64A830"), ("363",  "508A28"),
    ("364",  "406E20"), ("365",  "CEE890"), ("366",  "C0E070"), ("367",  "A8D450"),
    ("368",  "78BE20"), ("369",  "68A010"), ("370",  "528400"), ("371",  "406800"),
    ("372",  "D8EC9C"), ("373",  "CCE888"), ("374",  "BAE06E"), ("375",  "97D700"),
    ("376",  "84C200"), ("377",  "6CA400"), ("378",  "548600"), ("379",  "E0EC88"),
    ("380",  "D4E048"), ("381",  "C8DC20"), ("382",  "B4CC00"), ("383",  "A0B800"),
    ("384",  "8A9E00"), ("385",  "748400"), ("386",  "EAF080"), ("387",  "E0EC68"),
    ("388",  "D4E440"), ("389",  "CCD800"), ("390",  "BACA00"), ("391",  "A6B800"),
    ("392",  "909E00"), ("393",  "EFF286"), ("394",  "EAF068"), ("395",  "E4EC48"),
    ("396",  "DCE800"), ("397",  "C8D400"), ("398",  "B0B800"), ("399",  "909800"),
    // ── Red (485 series) + Brown tones ────────────────────────────────────────
    ("461",  "F0DBC0"), ("462",  "E0C890"), ("463",  "C8A878"), ("464",  "A88054"),
    ("465",  "8C6640"), ("466",  "F0DEC8"), ("467",  "E8CCA8"), ("468",  "D4B48C"),
    ("469",  "A0622C"), ("470",  "8C5020"), ("471",  "784018"), ("472",  "F8D8C0"),
    ("473",  "F4C8A8"), ("474",  "ECA888"), ("475",  "DC8050"), ("476",  "7A4830"),
    ("477",  "6C3820"), ("478",  "582C18"), ("479",  "EDD8C0"), ("480",  "E8C8A0"),
    ("481",  "D8A870"), ("482",  "9A6040"), ("483",  "7A4830"), ("484",  "6A3820"),
    ("485",  "DA291C"), ("486",  "EBA890"), ("487",  "E89070"), ("488",  "DC6840"),
    ("489",  "C84818"),
    // ── Warm greys ────────────────────────────────────────────────────────────
    ("416",  "C4BCAC"), ("417",  "ACA49C"), ("418",  "8C847A"), ("419",  "1A1614"),
    ("420",  "D4CCC4"), ("421",  "BCBCB4"), ("422",  "A4A49C"), ("423",  "8C8C84"),
    ("424",  "70706C"), ("425",  "4A4A46"), ("426",  "1A1A18"),
    ("427",  "D8D4D0"), ("428",  "C8C8C4"), ("429",  "B4B4B0"), ("430",  "909088"),
    ("431",  "686868"), ("432",  "404040"), ("433",  "282828"),
    ("434",  "E0D8D4"), ("435",  "D4CCC8"), ("436",  "C4BCBA"), ("437",  "9C9090"),
    ("438",  "706860"), ("439",  "504840"), ("440",  "382E2A"),
    // ── Cool greys ────────────────────────────────────────────────────────────
    ("Cool Gray 1",  "E0DDD8"), ("Cool Gray 2",  "D4D2CC"), ("Cool Gray 3",  "C4C2BC"),
    ("Cool Gray 4",  "B4B2AC"), ("Cool Gray 5",  "A8A6A0"), ("Cool Gray 6",  "989690"),
    ("Cool Gray 7",  "888884"), ("Cool Gray 8",  "747472"), ("Cool Gray 9",  "626060"),
    ("Cool Gray 10", "4A4848"), ("Cool Gray 11", "383636"),
    // Alternate "CG X" format
    ("CG 1",  "E0DDD8"), ("CG 2",  "D4D2CC"), ("CG 3",  "C4C2BC"),
    ("CG 4",  "B4B2AC"), ("CG 5",  "A8A6A0"), ("CG 6",  "989690"),
    ("CG 7",  "888884"), ("CG 8",  "747472"), ("CG 9",  "626060"),
    ("CG 10", "4A4848"), ("CG 11", "383636"),
    // ── Warm grey alternate ───────────────────────────────────────────────────
    ("Warm Gray 1",  "D8D0C8"), ("Warm Gray 2",  "CCC4BC"), ("Warm Gray 3",  "BCB4AC"),
    ("Warm Gray 4",  "ACA49C"), ("Warm Gray 5",  "9C9488"), ("Warm Gray 6",  "8C8478"),
    ("Warm Gray 7",  "7C7468"), ("Warm Gray 8",  "6C6458"), ("Warm Gray 9",  "5C5448"),
    ("Warm Gray 10", "4C4440"), ("Warm Gray 11", "3C3430"),
    ("WG 1",  "D8D0C8"), ("WG 2",  "CCC4BC"), ("WG 3",  "BCB4AC"),
    ("WG 4",  "ACA49C"), ("WG 5",  "9C9488"), ("WG 6",  "8C8478"),
    ("WG 7",  "7C7468"), ("WG 8",  "6C6458"), ("WG 9",  "5C5448"),
    ("WG 10", "4C4440"), ("WG 11", "3C3430"),
    // ── Metallic approximations ───────────────────────────────────────────────
    ("871",  "9C8732"), ("872",  "8C7828"), ("873",  "806E20"), ("874",  "9A8430"),
    ("875",  "8A7426"), ("876",  "7A641C"), ("877",  "8C8C8C"),
    ("878",  "7A7A7A"), ("879",  "6A6A6A"),
    // ── Formula / special ─────────────────────────────────────────────────────
    ("021",        "FE5000"), ("032",        "EF3340"), ("072",        "003DA5"),
    ("Yellow",     "FEDD00"), ("Warm Red",   "F3432C"), ("Red 032",    "EF3340"),
    ("Rhodamine Red", "E0457B"), ("Rubine Red", "CA0044"),
    ("Reflex Blue","001489"), ("Violet",     "440099"), ("Process Blue","0085CA"),
    ("Green",      "00A651"), ("Black",      "231F20"),
    // ── Bright / Fluorescent ──────────────────────────────────────────────────
    ("801",  "008EAA"), ("802",  "6CC24A"), ("803",  "FFEF00"), ("804",  "FF8200"),
    ("805",  "FF5E57"), ("806",  "FF59F8"), ("807",  "F52886"),
    // ── 5xxx coated (muted/earthy tones — partial set) ────────────────────────
    ("5005", "4E7B8C"), ("5015", "3E7496"), ("5025", "7AACB8"), ("5035", "6E9EA8"),
    ("5115", "C8A0A8"), ("5125", "BF8C92"), ("5135", "B07880"), ("5145", "9C6268"),
    ("5155", "C8A080"), ("5165", "BF9068"), ("5175", "B07850"), ("5185", "9C6440"),
    ("5215", "D8B4C0"), ("5225", "D0A0B0"), ("5235", "C48CA0"), ("5245", "B87890"),
    ("5255", "5A6E9A"), ("5265", "4A5E88"), ("5275", "3A4E76"), ("5285", "2C3E64"),
    ("5315", "A0B8C8"), ("5325", "8AAAB8"), ("5335", "7498A8"), ("5345", "608898"),
    ("5415", "6A8EA0"), ("5425", "5A7E8E"), ("5435", "8AAAB8"), ("5445", "7A9AA8"),
    ("5455", "A8BEC8"), ("5465", "98B0BC"), ("5475", "6A909E"), ("5485", "587E8C"),
    ("5495", "90B0BE"), ("5505", "80A4B0"), ("5515", "6894A0"), ("5525", "54808E"),
    ("5535", "A0C0C8"), ("5545", "8CB4BC"), ("5555", "70A0A8"), ("5565", "5A8C94"),
    ("5575", "A8C8C4"), ("5585", "98BCBA"), ("5595", "7CACA8"), ("5605", "6A9C98"),
    ("5615", "9AB898"), ("5625", "86A884"), ("5635", "6C9470"), ("5645", "587C5C"),
    ("5655", "A4C4A0"), ("5665", "8CB488"), ("5675", "70A070"), ("5685", "5A8860"),
    ("5695", "C0D4B4"), ("5705", "B0C8A4"), ("5715", "98B888"), ("5725", "84A870"),
    ("5735", "AAC498"), ("5745", "98B484"), ("5755", "84A46E"), ("5765", "74945C"),
    ("5775", "C4D4A0"), ("5785", "B4C890"), ("5795", "A0B878"), ("5805", "8CA864"),
    // ── 7xxx (additional darks — partial) ─────────────────────────────────────
    ("7541", "D4D8DC"), ("7542", "C0D0D4"), ("7543", "A8B4B8"), ("7544", "8C9CA0"),
    ("7545", "5C6C72"), ("7546", "2C3C44"), ("7547", "1A282E"),
];

// ── Stylesheet ────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

body {
  font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
  font-size: 10pt;
  color: #1a1a1a;
  background: #fff;
  -webkit-print-color-adjust: exact;
  print-color-adjust: exact;
}

@page { size: A4 portrait; margin: 0; }

/* ── Screen-only UI ── */
.no-print {
  position: fixed; top: 10px; right: 10px; z-index: 100;
  display: flex; gap: 6px;
}
.no-print button {
  padding: 6px 16px;
  background: #1e293b; color: #fff;
  border: none; border-radius: 4px;
  font-size: 12px; cursor: pointer; font-family: inherit;
}
.no-print button:hover { background: #334155; }
@media print { .no-print { display: none !important; } }

/* ── Header ── */
.report-header { margin-bottom: 8mm; }
.accent-bar { height: 6px; background: #1e293b; }
.header-inner {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 7mm 17mm 7mm;
  border-bottom: 1pt solid #1e293b;
}
.header-left { flex: 1; }
.header-right { flex-shrink: 0; padding-left: 12mm; text-align: right; }

.customer {
  font-size: 20pt; font-weight: 700; color: #1e293b;
  letter-spacing: -0.4px; line-height: 1.1; margin-bottom: 4pt;
}
.job-name { font-size: 11pt; color: #374151; font-weight: 500; margin-bottom: 5pt; }
.spec-line { display: flex; flex-wrap: wrap; gap: 3pt; }
.spec-tag {
  display: inline-block;
  background: #f1f5f9; border: 0.5pt solid #cbd5e1; border-radius: 3px;
  padding: 2pt 6pt; font-size: 8pt; font-weight: 600; color: #475569;
  letter-spacing: 0.3px;
}

.detail-item { margin-bottom: 5pt; line-height: 1.2; }
.detail-label {
  display: block; font-size: 6.5pt; text-transform: uppercase;
  letter-spacing: 0.6px; color: #9ca3af; margin-bottom: 1pt;
}
.detail-value { font-size: 9pt; font-weight: 600; color: #1e293b; }

/* ── Shape section ── */
.shape-section {
  padding: 0 17mm;
  margin-bottom: 8mm;
  page-break-inside: avoid;
}
.shape-heading {
  display: flex; align-items: center; gap: 6pt; margin-bottom: 5pt;
}
.shape-heading::after {
  content: ''; flex: 1; height: 0.5pt; background: #cbd5e1;
}
.shape-label {
  font-size: 8.5pt; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.8px; color: #1e293b;
  background: #f1f5f9; border: 0.5pt solid #cbd5e1; border-radius: 3px;
  padding: 2.5pt 8pt; white-space: nowrap;
}
.empty-note { font-size: 8.5pt; color: #9ca3af; font-style: italic; padding: 4pt 17mm; }

/* ── Data table ── */
.data-table {
  border-collapse: collapse;
  font-size: 8.5pt;
  font-variant-numeric: tabular-nums;
  /* auto table-layout: columns size to content */
}

/* Header cells */
th {
  padding: 3.5pt 5pt;
  text-align: center;
  font-size: 7.5pt;
  font-weight: 700;
  letter-spacing: 0.2px;
  background: #1e293b;
  color: #94a3b8;
  border: 0.5pt solid #334155;
  white-space: nowrap;
}
th.th-corner {
  text-align: left; padding-left: 5pt;
  background: #0f172a; color: #64748b;
  font-size: 7pt; text-transform: uppercase; letter-spacing: 0.5px;
}
th.th-target {
  background: #0f172a; color: #64748b;
  font-size: 7pt; text-transform: uppercase; letter-spacing: 0.5px;
  border-left: 1pt solid #475569;
}
th.th-lpi-group {
  background: #263347; color: #cbd5e1;
  font-size: 8pt; font-weight: 700; letter-spacing: 0.5px;
  border-bottom: 1.5pt solid #475569; padding: 4pt 5pt;
}

/* Ink name headers — coloured on dark background */
th.th-ink            { background: #1a2535; font-size: 8pt; font-weight: 700; }
th.ink-c             { color: #38bdf8; }
th.ink-m             { color: #f472b6; }
th.ink-y             { color: #fbbf24; }
th.ink-k             { color: #e2e8f0; }
th.ink-w             { color: #f1f5f9; }
th.ink-spot          { color: #c4b5fd; }  /* fallback if no Pantone match */

th.th-avg  { background: #1a2535; color: #86efac; font-size: 7pt; }
th.th-dev  { background: #1a2535; color: #a5b4fc; font-size: 7pt; }

/* LPI group separator */
th.lpi-last, td.lpi-last {
  border-right: 2pt solid #64748b !important;
}

/* Body cells */
td {
  padding: 2.5pt 5pt;
  text-align: right;
  border: 0.5pt solid #e5e7eb;
  white-space: nowrap;
}
td.td-step {
  text-align: left; padding-left: 5pt;
  background: #f8fafc !important;
  color: #374151; font-size: 8pt; font-weight: 500;
  border-right: 1pt solid #cbd5e1;
}
td.td-target {
  background: #f0f4ff !important;
  color: #4b5563; font-style: italic;
  border-left: 1pt solid #c7d2fe;
}
td.td-avg  { color: #15803d; }
td.td-dev  { color: #374151; }
td.td-data { color: #111827; }

/* Density row */
tr.row-density td           { background: #f1f5f9; }
tr.row-density td.td-step   { background: #e2e8f0 !important; font-weight: 700; }
tr.row-density td.td-target { background: #e8edfa !important; }

/* Alternating shading */
tr:nth-child(even):not(.row-density) td.td-data { background: #fafafa; }
tr:nth-child(even):not(.row-density) td.td-avg  { background: #f0fdf4; }

/* ── Combined report — compact overrides ── */
body.combined .report-header { margin-bottom: 2mm; }
body.combined .accent-bar    { height: 4px; }
body.combined .header-inner  { padding: 3mm 12mm 3mm; }
body.combined .customer      { font-size: 13pt; margin-bottom: 2pt; }
body.combined .job-name      { font-size: 9.5pt; margin-bottom: 2pt; }
body.combined .spec-tag      { font-size: 7pt; padding: 1pt 4pt; }
body.combined .detail-item   { margin-bottom: 2pt; }
body.combined .detail-value  { font-size: 8pt; }
body.combined .shape-section { padding: 0 12mm; margin-bottom: 3mm; }
body.combined .shape-label   { font-size: 7.5pt; padding: 2pt 6pt; }
body.combined .job-section   { margin-bottom: 0; }
body.combined .job-section + .job-section { margin-top: 4mm; }
body.combined .job-section .accent-bar { display: none; }
body.combined .job-section .header-inner { border-bottom: none; padding-top: 1mm; }
body.combined .shared-banner .header-inner { padding: 3mm 12mm; }
body.combined .shared-banner .customer     { font-size: 14pt; margin-bottom: 2pt; }
body.combined .shared-banner .detail-value { font-size: 9pt; }
"#;

// ── Combined multi-job report ─────────────────────────────────────────────────

/// Generate a single print-ready HTML document containing the full report for
/// every job — header + all step tables — identical in appearance to the
/// single-job report.  Fields that are identical across all jobs are shown
/// once in a shared banner at the top; unique fields appear in each job's
/// own header.  Jobs are separated by a CSS page-break so the browser's
/// print-to-PDF produces one tidy document.
pub fn generate_comparison_report(jobs: &[&JobConfig]) -> String {
    if jobs.is_empty() {
        return "<html><body><p>No jobs provided.</p></body></html>".to_string();
    }

    let first = jobs[0];

    // ── Identify shared vs unique fields ──────────────────────────────────────
    let s_customer   = all_same(jobs, |j| &j.customer);
    let s_job_name   = all_same(jobs, |j| &j.job_name);
    let s_plate_tech = all_same(jobs, |j| &j.plate_tech);
    let s_esxr       = all_same(jobs, |j| &j.esxr_number);
    let s_press      = all_same(jobs, |j| &j.press_system);
    let s_print_type = all_same(jobs, |j| &j.print_type);
    let s_date       = all_same(jobs, |j| &j.date);
    let s_set        = all_same(jobs, |j| &j.set_number);

    // Shared inks string (names only, for display)
    let inks_str = |j: &JobConfig| j.inks.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(",");
    let s_inks = {
        let first_inks = inks_str(first);
        !first_inks.is_empty() && jobs.iter().all(|j| inks_str(j) == first_inks)
    };

    // ── Shared banner ─────────────────────────────────────────────────────────
    let customer_html = if s_customer && !first.customer.is_empty() {
        format!(r#"<div class="customer">{}</div>"#, esc(&first.customer))
    } else { String::new() };

    let job_name_html = if s_job_name && !first.job_name.is_empty() {
        format!(r#"<div class="job-name">{}</div>"#, esc(&first.job_name))
    } else { String::new() };

    let shared_spec_tags: String = [
        if s_plate_tech { first.plate_tech.as_str() } else { "" },
        if s_esxr       { first.esxr_number.as_str() } else { "" },
        if s_press      { first.press_system.as_str() } else { "" },
        if s_print_type { first.print_type.as_str() } else { "" },
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .map(|s| format!(r#"<span class="spec-tag">{}</span>"#, esc(s)))
    .collect::<Vec<_>>()
    .join(" ");

    let shared_specs = if !shared_spec_tags.is_empty() {
        format!(r#"<div class="spec-line">{shared_spec_tags}</div>"#)
    } else { String::new() };

    // Shared right-side detail items (date / set if same across all jobs)
    let shared_right_items: String = {
        let mut items = String::new();
        if s_date && !first.date.is_empty() {
            items.push_str(&format!(
                r#"<div class="detail-item"><span class="detail-label">DATE</span><span class="detail-value">{}</span></div>"#,
                esc(&first.date)
            ));
        }
        if s_set && !first.set_number.is_empty() {
            items.push_str(&format!(
                r#"<div class="detail-item"><span class="detail-label">SET</span><span class="detail-value">{}</span></div>"#,
                esc(&first.set_number)
            ));
        }
        if s_inks {
            let ink_names = first.inks.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join(", ");
            items.push_str(&format!(
                r#"<div class="detail-item"><span class="detail-label">INKS</span><span class="detail-value">{}</span></div>"#,
                esc(&ink_names)
            ));
        }
        items.push_str(&format!(
            r#"<div class="detail-item"><span class="detail-label">JOBS</span><span class="detail-value">{}</span></div>"#,
            jobs.len()
        ));
        items
    };

    let shared_right = format!(r#"<div class="header-right">{shared_right_items}</div>"#);

    let shared_banner = if customer_html.is_empty() && job_name_html.is_empty()
        && shared_specs.is_empty() && shared_right_items.is_empty()
    {
        String::new()
    } else {
        format!(
            r#"<header class="report-header shared-banner">
<div class="accent-bar"></div>
<div class="header-inner">
  <div class="header-left">{customer_html}{job_name_html}{shared_specs}</div>
  {shared_right}
</div>
</header>"#
        )
    };

    // ── Per-job sections ──────────────────────────────────────────────────────
    let job_sections: String = jobs.iter().map(|job| {
        // Only show fields in the per-job header that differ across jobs.
        let customer_h = if !s_customer && !job.customer.is_empty() {
            format!(r#"<div class="customer">{}</div>"#, esc(&job.customer))
        } else { String::new() };

        let job_name_h = if !s_job_name && !job.job_name.is_empty() {
            format!(r#"<div class="job-name">{}</div>"#, esc(&job.job_name))
        } else { String::new() };

        let spec_tags: String = [
            if !s_plate_tech { job.plate_tech.as_str() } else { "" },
            if !s_esxr       { job.esxr_number.as_str() } else { "" },
            if !s_press      { job.press_system.as_str() } else { "" },
            if !s_print_type { job.print_type.as_str() } else { "" },
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| format!(r#"<span class="spec-tag">{}</span>"#, esc(s)))
        .collect::<Vec<_>>()
        .join(" ");

        let specs_h = if !spec_tags.is_empty() {
            format!(r#"<div class="spec-line">{spec_tags}</div>"#)
        } else { String::new() };

        // Right side: only show job-unique identity fields
        let right_items: String = {
            let mut items = String::new();
            if !job.job_number.is_empty() {
                items.push_str(&format!(
                    r#"<div class="detail-item"><span class="detail-label">JOB</span><span class="detail-value">{}</span></div>"#,
                    esc(&job.job_number)
                ));
            }
            if !s_date && !job.date.is_empty() {
                items.push_str(&format!(
                    r#"<div class="detail-item"><span class="detail-label">DATE</span><span class="detail-value">{}</span></div>"#,
                    esc(&job.date)
                ));
            }
            if !s_set && !job.set_number.is_empty() {
                items.push_str(&format!(
                    r#"<div class="detail-item"><span class="detail-label">SET</span><span class="detail-value">{}</span></div>"#,
                    esc(&job.set_number)
                ));
            }
            items
        };

        let right_h = if right_items.is_empty() { String::new() } else {
            format!(r#"<div class="header-right">{right_items}</div>"#)
        };

        // Skip the per-job header entirely if there's nothing unique to show
        let header_h = if customer_h.is_empty() && job_name_h.is_empty()
            && specs_h.is_empty() && right_h.is_empty()
        {
            String::new()
        } else {
            format!(
                r#"<header class="report-header">
<div class="accent-bar"></div>
<div class="header-inner">
  <div class="header-left">{customer_h}{job_name_h}{specs_h}</div>
  {right_h}
</div>
</header>"#
            )
        };

        format!(r#"<div class="job-section">{header_h}{body}</div>"#, body = build_body(job))
    }).collect();

    let title = format!("Combined Report ({} jobs)", jobs.len());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{title}</title>
<style>{css}</style>
</head>
<body class="combined">
<div class="no-print">
  <button onclick="window.print()">Print / Save PDF</button>
  <button onclick="window.close()">Close</button>
</div>
{shared_banner}
{job_sections}
</body>
</html>"#,
        title         = esc(&title),
        css           = CSS,
        shared_banner = shared_banner,
        job_sections  = job_sections,
    )
}

/// Returns true if the extracted string field is non-empty and identical
/// across every job.
fn all_same<'a, F>(jobs: &[&'a JobConfig], f: F) -> bool
where
    F: Fn(&'a JobConfig) -> &'a String,
{
    let mut iter = jobs.iter().map(|j| f(j).as_str());
    let first = match iter.next() {
        Some(v) if !v.is_empty() => v,
        _ => return false,
    };
    iter.all(|v| v == first)
}
