//! `kz context`: add / remove "Analyze with kazma" in Explorer's right-click menu.
//! Per-user (HKCU), so no administrator rights are needed. On Windows 11 it shows
//! under "Show more options".

use crate::{oplog, ps, tr, ui};

const KEYS: &str = r"'HKCU:\Software\Classes\Directory\shell\kazma','HKCU:\Software\Classes\Directory\Background\shell\kazma','HKCU:\Software\Classes\Drive\shell\kazma'";

fn install_script() -> String {
    format!(
        r#"
foreach ($base in @({KEYS})) {{
  New-Item -Path "$base\command" -Force | Out-Null
  Set-Item -LiteralPath $base -Value $env:KAZMA_LABEL
  New-ItemProperty -LiteralPath $base -Name 'Icon' -Value ('"' + $env:KAZMA_EXE + '",0') -PropertyType String -Force | Out-Null
  Set-Item -LiteralPath "$base\command" -Value ('"' + $env:KAZMA_EXE + '" analyze "%V"')
}}
"#
    )
}

fn remove_script() -> String {
    format!(
        "foreach ($base in @({KEYS})) {{ if (Test-Path -LiteralPath $base) {{ Remove-Item -LiteralPath $base -Recurse -Force }} }}"
    )
}

fn status_script() -> String {
    format!(
        "@({KEYS}) | Where-Object {{ Test-Path -LiteralPath $_ }} | Measure-Object | Select-Object -ExpandProperty Count"
    )
}

pub fn run(action: Option<String>) -> anyhow::Result<()> {
    ui::print_banner(ui::t(
        "context :: Explorer right-click menu",
        "context :: Gezgin sağ tık menüsü",
    ));
    if !cfg!(windows) {
        ui::warn(ui::t(
            "kz context works on Windows only.",
            "kz context yalnızca Windows'ta çalışır.",
        ));
        return Ok(());
    }
    match action.as_deref().unwrap_or("status") {
        "install" => {
            let exe = std::env::current_exe()?;
            let lower = exe.to_string_lossy().to_lowercase();
            if lower.contains("\\downloads\\") || lower.contains("\\temp\\") {
                ui::warn(&tr!(
                    "kz.exe is in a temporary place ({}); the menu entry breaks if it moves.",
                    "kz.exe geçici bir yerde ({}); taşınırsa menü girdisi bozulur.",
                    exe.display()
                ));
            }
            let label = ui::t("Analyze with kazma", "kazma ile analiz et");
            ps::text(
                &install_script(),
                &[
                    ("KAZMA_EXE", &exe.to_string_lossy()),
                    ("KAZMA_LABEL", label),
                ],
            )?;
            oplog::record("context", "installed");
            ui::ok(&tr!(
                "added \"{label}\" to folder, drive and background menus",
                "\"{label}\" klasör, sürücü ve arka plan menülerine eklendi"
            ));
            println!(
                "  {}\n",
                ui::t(
                    "Windows 11: right-click -> Show more options",
                    "Windows 11: sağ tık -> Diğer seçenekleri göster"
                )
            );
        }
        "uninstall" | "remove" => {
            ps::text(&remove_script(), &[])?;
            oplog::record("context", "removed");
            ui::ok(ui::t(
                "right-click entry removed",
                "sağ tık girdisi kaldırıldı",
            ));
        }
        "status" => {
            let n: u32 = ps::text(&status_script(), &[])?.trim().parse().unwrap_or(0);
            if n > 0 {
                ui::ok(&tr!(
                    "installed ({n}/3 menus).  remove: kz context uninstall",
                    "kurulu ({n}/3 menü).  kaldır: kz context uninstall"
                ));
            } else {
                ui::skip(ui::t(
                    "not installed.  add: kz context install",
                    "kurulu değil.  ekle: kz context install",
                ));
            }
            println!();
        }
        other => anyhow::bail!(
            "{}",
            tr!(
                "unknown: {other}. use install | uninstall | status",
                "bilinmeyen: {other}. install | uninstall | status kullanın"
            )
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_target_only_kazma_keys() {
        for s in [install_script(), remove_script(), status_script()] {
            assert!(s.contains(r"shell\kazma"));
            assert!(!s.contains(r"shell\open"));
        }
        assert!(install_script().contains("%V"));
    }
}
