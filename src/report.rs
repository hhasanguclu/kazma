//! `kz report`: one HTML file with hardware, health, startup, programs and drivers,
//! ready to send to whoever helps you with your PC. Nothing is uploaded anywhere.

use crate::fsutil::home;
use crate::{drivers, health, oplog, ps, startup, sys, tr, ui, uninstall};
use serde::Deserialize;
use std::path::PathBuf;

const HARDWARE: &str = r#"
$cs = Get-CimInstance Win32_ComputerSystem
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$bb = Get-CimInstance Win32_BaseBoard | Select-Object -First 1
$bios = Get-CimInstance Win32_BIOS | Select-Object -First 1
$gpu = (Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }) -join ', '
$disk = (Get-PhysicalDisk | ForEach-Object { "{0} ({1:N0} GB {2})" -f $_.FriendlyName, ($_.Size / 1GB), $_.MediaType }) -join ', '
@(
  [pscustomobject]@{ K = 'Computer'; V = "$($cs.Manufacturer) $($cs.Model)" },
  [pscustomobject]@{ K = 'Name'; V = [string]$env:COMPUTERNAME },
  [pscustomobject]@{ K = 'Windows'; V = "$($os.Caption) $($os.Version) (build $($os.BuildNumber))" },
  [pscustomobject]@{ K = 'Installed'; V = $os.InstallDate.ToString('yyyy-MM-dd') },
  [pscustomobject]@{ K = 'CPU'; V = "$($cpu.Name.Trim()) ($($cpu.NumberOfCores) cores / $($cpu.NumberOfLogicalProcessors) threads)" },
  [pscustomobject]@{ K = 'RAM'; V = "{0:N1} GB" -f ($cs.TotalPhysicalMemory / 1GB) },
  [pscustomobject]@{ K = 'GPU'; V = $gpu },
  [pscustomobject]@{ K = 'Disks'; V = $disk },
  [pscustomobject]@{ K = 'Motherboard'; V = "$($bb.Manufacturer) $($bb.Product)" },
  [pscustomobject]@{ K = 'BIOS'; V = "$($bios.SMBIOSBIOSVersion) ($($bios.ReleaseDate.ToString('yyyy-MM-dd')))" }
) | ConvertTo-Json -Compress
"#;

#[derive(Debug, Deserialize)]
struct Kv {
    #[serde(rename = "K")]
    k: String,
    #[serde(rename = "V", default)]
    v: String,
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn table(head: &[&str], rows: &[Vec<String>]) -> String {
    let mut h = String::from("<table><tr>");
    for c in head {
        h += &format!("<th>{}</th>", esc(c));
    }
    h += "</tr>";
    for r in rows {
        h += "<tr>";
        for c in r {
            h += &format!("<td>{}</td>", esc(c));
        }
        h += "</tr>";
    }
    h + "</table>"
}

fn section(title: &str, body: &str) -> String {
    format!("<h2>&gt;&gt; {}</h2>\n{body}\n", esc(title))
}

pub fn page(title: &str, sections: &[String]) -> String {
    format!(
        r#"<!doctype html>
<html lang="{lang}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{t}</title>
<style>
body{{background:#0c0c0c;color:#16c60c;font:14px/1.45 Consolas,"DejaVu Sans Mono",monospace;margin:0;padding:24px}}
pre.banner{{color:#16c60c;margin:0}} .sub{{color:#767676;margin:4px 0 24px}}
h2{{color:#f9f1a5;font-size:15px;margin:28px 0 8px;border-bottom:1px dashed #13a10e;padding-bottom:4px}}
table{{border-collapse:collapse;width:100%;max-width:1200px}} th{{text-align:left;color:#f9f1a5;border-bottom:1px dashed #13a10e}}
th,td{{padding:3px 12px 3px 0;vertical-align:top}} td{{color:#cccccc}} tr:hover td{{background:#1a1a1a}}
.ok{{color:#16c60c}} .warn{{color:#f9f1a5}} .bad{{color:#e74856}} .info{{color:#61d6d6}}
@media print{{body{{background:#fff;color:#000}} td{{color:#000}} h2,th{{color:#000}}}}
</style></head><body>
<pre class="banner">{banner}</pre>
<div class="sub">{t}</div>
{body}
<div class="sub">kazma v{ver} &middot; https://github.com/hhasanguclu/kazma</div>
</body></html>
"#,
        lang = if ui::is_tr() { "tr" } else { "en" },
        t = esc(title),
        banner = esc(ui::BANNER.trim_matches('\n')),
        body = sections.join("\n"),
        ver = env!("CARGO_PKG_VERSION"),
    )
}

fn default_path() -> PathBuf {
    let date = &oplog::now_utc()[..10];
    home()
        .map(|h| h.join("Desktop"))
        .filter(|d| d.is_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("kazma-report-{date}.html"))
}

pub fn run(output: Option<PathBuf>, open: bool) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "report :: system report as HTML",
        "report :: HTML sistem raporu",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz report works on Windows only.",
            "kz report yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    let lang = [("KAZMA_TR", if ui::is_tr() { "1" } else { "0" })];
    let mut sections = vec![];

    ps::working(ui::t("hardware...", "donanım..."));
    match ps::powershell_json::<Kv>(HARDWARE, &[]) {
        Ok(v) => sections.push(section(
            ui::t("Hardware", "Donanım"),
            &table(
                &["", ""],
                &v.into_iter().map(|kv| vec![kv.k, kv.v]).collect::<Vec<_>>(),
            ),
        )),
        Err(e) => ui::warn(&format!("hardware: {e}")),
    }
    ps::working(ui::t(
        "health checks (10-30 s)...",
        "sağlık kontrolleri (10-30 sn)...",
    ));
    if let Ok(v) = ps::powershell_json::<health::Check>(health::CHECKS, &lang) {
        let rows: String = v
            .iter()
            .map(|c| {
                let mark = match c.status.as_str() {
                    "ok" => "[ok]",
                    "warn" => "[!!]",
                    "bad" => "[xx]",
                    _ => "[..]",
                };
                format!(
                    "<tr><td class=\"{}\">{mark}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc(&c.status),
                    esc(&c.section),
                    esc(&c.item),
                    esc(&c.value)
                )
            })
            .collect();
        sections.push(section(
            ui::t("Health", "Sağlık"),
            &format!("<table>{rows}</table>"),
        ));
    }
    ps::working(ui::t("startup programs...", "başlangıç programları..."));
    if let Ok(v) = ps::powershell_json::<startup::Entry>(startup::LIST, &[]) {
        let rows: Vec<Vec<String>> = v
            .iter()
            .map(|e| {
                vec![
                    if e.enabled { "on".into() } else { "off".into() },
                    e.name.clone(),
                    e.kind.clone(),
                    e.command.clone(),
                ]
            })
            .collect();
        sections.push(section(
            &tr!("Startup ({})", "Başlangıç ({})", rows.len()),
            &table(&["", "name", "source", "command"], &rows),
        ));
    }
    ps::working(ui::t("installed programs...", "yüklü programlar..."));
    if let Ok(mut v) = ps::powershell_json::<uninstall::Program>(uninstall::LIST, &[]) {
        v.sort_by_key(|p| p.name.to_lowercase());
        v.dedup_by(|a, b| a.name == b.name && a.version == b.version);
        let rows: Vec<Vec<String>> = v
            .iter()
            .map(|p| {
                vec![
                    p.name.clone(),
                    p.version.clone(),
                    p.publisher.clone(),
                    if p.size_kb > 0 {
                        ui::human(p.size_kb as u64 * 1024)
                    } else {
                        String::new()
                    },
                ]
            })
            .collect();
        sections.push(section(
            &tr!("Programs ({})", "Programlar ({})", rows.len()),
            &table(&["name", "version", "publisher", "size"], &rows),
        ));
    }
    ps::working(ui::t("drivers...", "sürücüler..."));
    if let Ok(mut v) = ps::powershell_json::<drivers::InstalledDriver>(drivers::INSTALLED, &[]) {
        v.sort_by(|a, b| a.date.cmp(&b.date));
        let rows: Vec<Vec<String>> = v
            .iter()
            .map(|d| {
                vec![
                    d.date.clone(),
                    d.device.clone(),
                    d.version.clone(),
                    d.provider.clone(),
                    d.class.clone(),
                ]
            })
            .collect();
        sections.push(section(
            &tr!(
                "Third-party drivers ({})",
                "Üçüncü taraf sürücüler ({})",
                rows.len()
            ),
            &table(&["date", "device", "version", "provider", "class"], &rows),
        ));
    }
    println!("{:60}", "");

    let title = tr!(
        "System report, {}",
        "Sistem raporu, {}",
        &oplog::now_utc()[..16]
    );
    let path = output.unwrap_or_else(default_path);
    std::fs::write(&path, page(&title, &sections))?;
    oplog::record("report", &path.display().to_string());
    ui::ok(&tr!(
        "report saved: {}",
        "rapor kaydedildi: {}",
        path.display()
    ));
    println!(
        "  {}\n",
        ui::t(
            "it stays on this PC; share it only with people you trust.",
            "bu bilgisayarda kalır; yalnızca güvendiğiniz kişilerle paylaşın."
        )
    );
    if open {
        sys::open(&path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_is_escaped() {
        assert_eq!(
            esc("<a href=\"x\">&</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
        let t = table(&["n"], &[vec!["<script>".into()]]);
        assert!(t.contains("&lt;script&gt;") && !t.contains("<script>"));
        let p = page("t", &[section("s", "<p>b</p>")]);
        assert!(p.starts_with("<!doctype html>") && p.contains("&gt;&gt; s"));
    }
}
