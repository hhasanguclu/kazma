//! "Why is this process running?" for `kz status`: what a known Windows process
//! is, who started it, and whether it is a service or a startup entry.

use crate::{ps, startup, tr, ui};
use sysinfo::{Pid, System};

/// Short descriptions of processes people often wonder about.
pub fn known(name: &str) -> Option<&'static str> {
    let n = name.to_lowercase();
    let n = n.trim_end_matches(".exe");
    let t = ui::t;
    Some(match n {
        "system" => t("Windows kernel.", "Windows çekirdeği."),
        "registry" => t(
            "Windows registry in memory.",
            "Bellekteki Windows kayıt defteri.",
        ),
        "smss" | "csrss" | "wininit" | "winlogon" | "services" | "lsaiso" => t(
            "Core Windows process. Do not end it.",
            "Windows'un temel süreci. Sonlandırmayın.",
        ),
        "lsass" => t(
            "Windows sign-in and security. Do not end it.",
            "Windows oturum ve güvenlik süreci. Sonlandırmayın.",
        ),
        "svchost" => t(
            "Host for Windows services; see the service name below.",
            "Windows servislerinin taşıyıcısı; servis adı aşağıda.",
        ),
        "dwm" => t(
            "Desktop Window Manager: draws windows and effects.",
            "Masaüstü Pencere Yöneticisi: pencereleri ve efektleri çizer.",
        ),
        "explorer" => t(
            "Windows Explorer: taskbar, desktop and folders.",
            "Windows Gezgini: görev çubuğu, masaüstü ve klasörler.",
        ),
        "sihost" | "shellexperiencehost" | "startmenuexperiencehost" | "shellhost" => t(
            "Part of the Windows shell (Start, notifications).",
            "Windows kabuğunun parçası (Başlat, bildirimler).",
        ),
        "searchhost" | "searchapp" | "searchui" | "searchindexer" | "searchprotocolhost"
        | "searchfilterhost" => t("Windows Search and its index.", "Windows Arama ve dizini."),
        "runtimebroker" => t(
            "Checks permissions for Store apps.",
            "Store uygulamalarının izinlerini denetler.",
        ),
        "ctfmon" => t(
            "Keyboard, language bar and input methods.",
            "Klavye, dil çubuğu ve giriş yöntemleri.",
        ),
        "textinputhost" => t(
            "Touch keyboard, emoji panel and input.",
            "Dokunmatik klavye, emoji paneli ve giriş.",
        ),
        "audiodg" => t("Windows audio engine.", "Windows ses motoru."),
        "spoolsv" => t("Print spooler (printing).", "Yazdırma biriktiricisi."),
        "fontdrvhost" => t("Font driver host.", "Yazı tipi sürücüsü."),
        "conhost" | "openconsole" => t(
            "Console window host for a terminal program.",
            "Bir terminal programının konsol penceresi.",
        ),
        "taskhostw" => t(
            "Runs Windows background tasks.",
            "Windows arka plan görevlerini çalıştırır.",
        ),
        "dllhost" => t(
            "Runs a COM component (e.g. thumbnails).",
            "Bir COM bileşenini çalıştırır (ör. küçük resimler).",
        ),
        "wmiprvse" => t(
            "WMI provider: answers system information queries.",
            "WMI sağlayıcısı: sistem bilgisi sorgularını yanıtlar.",
        ),
        "msmpeng" | "mpdefendercoreservice" | "nissrv" => t(
            "Microsoft Defender antivirus engine.",
            "Microsoft Defender antivirüs motoru.",
        ),
        "securityhealthservice" | "securityhealthsystray" => {
            t("Windows Security status.", "Windows Güvenliği durumu.")
        }
        "msedgewebview2" => t(
            "Web view used inside an app (Teams, Widgets, Outlook...).",
            "Bir uygulamanın içindeki web görünümü (Teams, Widget'lar, Outlook...).",
        ),
        "onedrive" => t("OneDrive sync.", "OneDrive eşitleme."),
        "phoneexperiencehost" => t("Phone Link.", "Telefon Bağlantısı."),
        "widgets" | "widgetservice" => t("Windows Widgets.", "Windows Widget'ları."),
        "memory compression" => t(
            "Compressed memory: keeps more in RAM. Normal.",
            "Sıkıştırılmış bellek: RAM'de daha fazlasını tutar. Normaldir.",
        ),
        "tiworker" | "trustedinstaller" => t(
            "Windows Update / component servicing. Busy after updates.",
            "Windows Update / bileşen bakımı. Güncellemelerden sonra yoğundur.",
        ),
        "compattelrunner" => t(
            "Windows compatibility telemetry.",
            "Windows uyumluluk telemetrisi.",
        ),
        _ => return None,
    })
}

fn parent_reason(parent: &str) -> Option<&'static str> {
    let p = parent.to_lowercase();
    Some(match p.trim_end_matches(".exe") {
        "explorer" => ui::t(
            "You started it (desktop, Start menu or a shortcut).",
            "Siz başlattınız (masaüstü, Başlat menüsü ya da kısayol).",
        ),
        "services" => ui::t(
            "Started by Windows as a service.",
            "Windows tarafından servis olarak başlatıldı.",
        ),
        "svchost" => ui::t(
            "Started by a Windows service (often a scheduled task).",
            "Bir Windows servisi başlattı (çoğu zaman zamanlanmış görev).",
        ),
        "wininit" | "winlogon" | "smss" => ui::t(
            "Started by Windows itself at boot or sign-in.",
            "Windows açılışta ya da oturum açılırken başlattı.",
        ),
        "userinit" => ui::t("Started at sign-in.", "Oturum açılırken başlatıldı."),
        _ => return None,
    })
}

fn services_of(pid: u32) -> Vec<String> {
    if !cfg!(windows) {
        return vec![];
    }
    let script = "Get-CimInstance Win32_Service -Filter \"ProcessId=$($env:KAZMA_PID)\" | ForEach-Object { \"$($_.Name) - $($_.DisplayName)\" }";
    ps::text(script, &[("KAZMA_PID", &pid.to_string())])
        .map(|o| {
            o.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn duration(secs: u64) -> String {
    match secs {
        s if s >= 86_400 => tr!("{}d {}h", "{} gün {} sa", s / 86_400, s % 86_400 / 3600),
        s if s >= 3600 => tr!("{}h {}m", "{} sa {} dk", s / 3600, s % 3600 / 60),
        s => tr!("{}m", "{} dk", s / 60),
    }
}

/// (label, text) lines describing why `pid` is running.
pub fn explain(
    sys: &System,
    pid: u32,
    startup_entries: &[startup::Entry],
) -> Vec<(String, String)> {
    let Some(p) = sys.process(Pid::from_u32(pid)) else {
        return vec![(
            ui::t("process", "süreç").into(),
            ui::t("has exited", "kapanmış").into(),
        )];
    };
    let name = p.name().to_string_lossy().into_owned();
    let exe = p.exe().map(|e| e.display().to_string()).unwrap_or_default();
    let cmd = p
        .cmd()
        .iter()
        .map(|c| c.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let parent = p
        .parent()
        .and_then(|pp| sys.process(pp))
        .map(|pp| (pp.name().to_string_lossy().into_owned(), pp.pid().as_u32()));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut v: Vec<(String, String)> = vec![];
    let l = |s: &str| s.to_string();
    if let Some(k) = known(&name) {
        v.push((l(ui::t("what", "nedir")), k.into()));
    }
    // why it is running
    let mut why: Vec<String> = vec![];
    let svcs = services_of(pid);
    if !svcs.is_empty() {
        why.push(tr!(
            "Windows service: {}",
            "Windows servisi: {}",
            svcs.join(", ")
        ));
    }
    let lower_exe = exe.to_lowercase();
    if !lower_exe.is_empty()
        && let Some(e) = startup_entries
            .iter()
            .find(|e| e.command.to_lowercase().contains(&lower_exe))
    {
        why.push(tr!(
            "Starts with Windows ({}: {}). Turn it off in kz startup.",
            "Windows ile başlıyor ({}: {}). kz startup ile kapatabilirsiniz.",
            e.kind,
            e.name
        ));
    }
    if let Some((pn, _)) = &parent
        && let Some(r) = parent_reason(pn)
    {
        why.push(r.to_string());
    }
    if why.is_empty() {
        why.push(
            ui::t(
                "Started by another program (see parent).",
                "Başka bir program başlattı (üst sürece bakın).",
            )
            .into(),
        );
    }
    for (i, w) in why.into_iter().enumerate() {
        v.push((
            if i == 0 {
                l(ui::t("why", "neden"))
            } else {
                String::new()
            },
            w,
        ));
    }
    if let Some((pn, pp)) = parent {
        v.push((l(ui::t("parent", "üst süreç")), format!("{pn} ({pp})")));
    }
    if !exe.is_empty() {
        v.push((l(ui::t("path", "yol")), exe));
    }
    if !cmd.is_empty() {
        v.push((l(ui::t("command", "komut")), cmd));
    }
    if p.start_time() > 0 {
        v.push((
            l(ui::t("running for", "çalışma süresi")),
            duration(now.saturating_sub(p.start_time())),
        ));
    }
    v.push((l(ui::t("memory", "bellek")), ui::human(p.memory())));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions() {
        assert!(known("svchost.exe").is_some());
        assert!(known("MsMpEng.exe").is_some());
        assert!(known("random-tool.exe").is_none());
        assert!(parent_reason("explorer.exe").is_some());
        assert!(duration(90_000).contains('1'));
    }

    #[test]
    fn explains_self() {
        let sys = System::new_all().unwrap();
        let me = std::process::id();
        let lines = explain(&sys, me, &[]);
        assert!(lines.iter().any(|(_, t)| !t.is_empty()));
    }
}
