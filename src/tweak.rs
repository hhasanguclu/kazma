//! `kz tweak`: Windows settings for privacy, AI, Edge, Explorer, the taskbar,
//! gaming and updates, shown as a list of switches.
//!
//! Every setting is a handful of registry values (services are switched through
//! their `Start` value, a few telemetry settings also turn scheduled tasks off).
//! Before kazma changes anything it saves the values that were there in
//! `%LOCALAPPDATA%\kazma\tweaks.json`, so `kz tweak undo` puts back exactly what
//! was there, not just the Windows default.

use crate::{oplog, ps, sys, tr, ui};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A registry value as a setting wants it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum V {
    /// REG_DWORD
    D(u32),
    /// REG_SZ
    S(&'static str),
    /// the value is not there
    Absent,
    /// the whole key is not there
    NoKey,
}

struct Reg {
    /// `HKCU\...` or `HKLM\...`
    key: &'static str,
    /// value name; "" is the key's default value
    name: &'static str,
    /// what the setting writes
    set: V,
    /// what Windows has out of the box (used by undo when there is no backup)
    default: V,
}

/// What has to happen before a change shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum After {
    Nothing,
    Explorer,
    SignOut,
    Reboot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Os {
    Any,
    Win10,
    Win11,
}

struct Tweak {
    id: &'static str,
    cat: &'static str,
    en: &'static str,
    tr: &'static str,
    detail_en: &'static str,
    detail_tr: &'static str,
    regs: &'static [Reg],
    /// scheduled tasks (`\Folder\Name`) to turn off as well
    tasks: &'static [&'static str],
    os: Os,
    after: After,
}

/// (key, English, Turkish)
const CATEGORIES: &[(&str, &str, &str)] = &[
    ("privacy", "Privacy", "Gizlilik"),
    ("ads", "Ads and suggestions", "Reklam ve öneriler"),
    ("ai", "AI features", "Yapay zekâ özellikleri"),
    ("edge", "Microsoft Edge", "Microsoft Edge"),
    ("explorer", "File Explorer", "Dosya Gezgini"),
    ("taskbar", "Taskbar", "Görev çubuğu"),
    ("look", "Look and feel", "Görünüm"),
    ("gaming", "Gaming and input", "Oyun ve giriş aygıtları"),
    ("system", "System", "Sistem"),
    ("updates", "Windows Update", "Windows Update"),
];

const fn r(key: &'static str, name: &'static str, set: V, default: V) -> Reg {
    Reg {
        key,
        name,
        set,
        default,
    }
}

use V::{Absent, D, NoKey, S};

const ADV: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const CDM: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const DATA: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DataCollection";
const WINAI_LM: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsAI";
const WINAI_CU: &str = r"HKCU\Software\Policies\Microsoft\Windows\WindowsAI";
const EDGE: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Edge";
const PAINT: &str = r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Paint";
const WU: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate";
const WU_UX: &str = r"HKLM\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings";
const PERSONALIZE: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

const TWEAKS: &[Tweak] = &[
    // ---- privacy ----------------------------------------------------------------
    Tweak {
        id: "telemetry",
        cat: "privacy",
        en: "Send the least diagnostic data",
        tr: "En az tanılama verisi gönder",
        detail_en: "telemetry policy, DiagTrack service and data collection tasks off",
        detail_tr: "telemetri ilkesi, DiagTrack hizmeti ve veri toplama görevleri kapalı",
        regs: &[
            r(DATA, "AllowTelemetry", D(0), Absent),
            r(
                r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection",
                "AllowTelemetry",
                D(0),
                Absent,
            ),
            r(DATA, "AllowDeviceNameInTelemetry", D(0), Absent),
            r(DATA, "DoNotShowFeedbackNotifications", D(1), Absent),
            r(
                r"HKLM\SYSTEM\CurrentControlSet\Services\DiagTrack",
                "Start",
                D(4),
                D(2),
            ),
            r(
                r"HKLM\SYSTEM\CurrentControlSet\Services\dmwappushservice",
                "Start",
                D(4),
                D(3),
            ),
        ],
        tasks: &[
            r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
            r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
            r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
            r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
            r"\Microsoft\Windows\Feedback\Siuf\DmClient",
            r"\Microsoft\Windows\Feedback\Siuf\DmClientOnScenarioDownload",
        ],
        os: Os::Any,
        after: After::Reboot,
    },
    Tweak {
        id: "feedback",
        cat: "privacy",
        en: "Never ask for feedback",
        tr: "Asla geri bildirim isteme",
        detail_en: "no \"how likely are you to recommend Windows\" pop-ups",
        detail_tr: "\"Windows'u tavsiye eder misiniz\" pencereleri çıkmaz",
        regs: &[r(
            r"HKCU\Software\Microsoft\Siuf\Rules",
            "NumberOfSIUFInPeriod",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "error-reporting",
        cat: "privacy",
        en: "Do not send crash reports",
        tr: "Çökme raporlarını gönderme",
        detail_en: "Windows Error Reporting stops uploading crash data to Microsoft",
        detail_tr: "Windows Hata Bildirimi çökme verisini Microsoft'a göndermez",
        regs: &[r(
            r"HKLM\SOFTWARE\Microsoft\Windows\Windows Error Reporting",
            "Disabled",
            D(1),
            Absent,
        )],
        tasks: &[r"\Microsoft\Windows\Windows Error Reporting\QueueReporting"],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "activity-history",
        cat: "privacy",
        en: "Do not keep an activity history",
        tr: "Etkinlik geçmişi tutma",
        detail_en: "Windows stops recording which apps and files you used",
        detail_tr: "Windows hangi uygulama ve dosyaları kullandığınızı kaydetmez",
        regs: &[
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System",
                "EnableActivityFeed",
                D(0),
                Absent,
            ),
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System",
                "PublishUserActivities",
                D(0),
                Absent,
            ),
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System",
                "UploadUserActivities",
                D(0),
                Absent,
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "app-tracking",
        cat: "privacy",
        en: "Do not track app launches",
        tr: "Uygulama açılışlarını izleme",
        detail_en: "Start and search stop ranking apps by how often you open them",
        detail_tr: "Başlat ve arama, uygulamaları açılma sıklığına göre sıralamaz",
        regs: &[r(ADV, "Start_TrackProgs", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "location",
        cat: "privacy",
        en: "Apps may not use your location",
        tr: "Uygulamalar konumu kullanamasın",
        detail_en: "the \"let apps access your location\" switch, for this user",
        detail_tr: "bu kullanıcı için \"uygulamalar konuma erişebilir\" anahtarı",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location",
            "Value",
            S("Deny"),
            S("Allow"),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "find-my-device",
        cat: "privacy",
        en: "Turn off Find My Device",
        tr: "Cihazımı Bul'u kapat",
        detail_en: "stops reporting where this PC is; you can no longer locate it if stolen",
        detail_tr: "bilgisayarın konumu bildirilmez; çalınırsa artık bulunamaz",
        regs: &[r(
            r"HKLM\SOFTWARE\Policies\Microsoft\FindMyDevice",
            "AllowFindMyDevice",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "online-speech",
        cat: "privacy",
        en: "Turn off online speech recognition",
        tr: "Çevrimiçi konuşma tanımayı kapat",
        detail_en: "your voice is no longer sent to Microsoft; dictation works offline only",
        detail_tr: "sesiniz Microsoft'a gönderilmez; dikte yalnızca çevrimdışı çalışır",
        regs: &[r(
            r"HKCU\Software\Microsoft\Speech_OneCore\Settings\OnlineSpeechPrivacy",
            "HasAccepted",
            D(0),
            D(1),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "typing-data",
        cat: "privacy",
        en: "Do not learn from typing and inking",
        tr: "Yazma ve kalem verisinden öğrenme",
        detail_en: "Windows stops building a dictionary from what you type, write and your contacts",
        detail_tr: "Windows yazdıklarınızdan ve kişilerinizden sözlük oluşturmaz",
        regs: &[
            r(
                r"HKCU\Software\Microsoft\InputPersonalization",
                "RestrictImplicitInkCollection",
                D(1),
                D(0),
            ),
            r(
                r"HKCU\Software\Microsoft\InputPersonalization",
                "RestrictImplicitTextCollection",
                D(1),
                D(0),
            ),
            r(
                r"HKCU\Software\Microsoft\InputPersonalization\TrainedDataStore",
                "HarvestContacts",
                D(0),
                D(1),
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    Tweak {
        id: "language-list",
        cat: "privacy",
        en: "Hide your language list from websites",
        tr: "Dil listesini web sitelerinden gizle",
        detail_en: "sites no longer get your installed languages to tailor content",
        detail_tr: "siteler içeriği uyarlamak için yüklü dillerinizi göremez",
        regs: &[r(
            r"HKCU\Control Panel\International\User Profile",
            "HttpAcceptLanguageOptOut",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "device-metadata",
        cat: "privacy",
        en: "Do not download device icons and info",
        tr: "Aygıt simgesi ve bilgisi indirme",
        detail_en: "new hardware shows a generic icon; drivers are not affected",
        detail_tr: "yeni donanım genel bir simgeyle görünür; sürücüler etkilenmez",
        regs: &[r(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Device Metadata",
            "PreventDeviceMetadataFromNetwork",
            D(1),
            D(0),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    // ---- ads ------------------------------------------------------------------------
    Tweak {
        id: "consumer-features",
        cat: "ads",
        en: "Do not install sponsored apps",
        tr: "Sponsorlu uygulamaları kurma",
        detail_en: "stops Windows adding promoted Store apps and games for new users",
        detail_tr: "Windows yeni kullanıcılara tanıtım uygulaması ve oyun eklemez",
        regs: &[r(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent",
            "DisableWindowsConsumerFeatures",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "finish-setup",
        cat: "ads",
        en: "No \"finish setting up your PC\" screen",
        tr: "\"Bilgisayarınızı kurmayı bitirin\" ekranı yok",
        detail_en: "the full-screen Microsoft 365 / OneDrive offer after updates",
        detail_tr: "güncellemelerden sonraki tam ekran Microsoft 365 / OneDrive teklifi",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement",
            "ScoobeSystemSettingEnabled",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "welcome-experience",
        cat: "ads",
        en: "No welcome screen after updates",
        tr: "Güncellemelerden sonra karşılama ekranı yok",
        detail_en: "skips the \"what's new\" pages Windows shows after a feature update",
        detail_tr: "özellik güncellemesinden sonraki \"yenilikler\" sayfaları atlanır",
        regs: &[r(CDM, "SubscribedContent-310093Enabled", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "spotlight",
        cat: "ads",
        en: "Turn off Windows Spotlight on the lock screen",
        tr: "Kilit ekranında Windows Spotlight'ı kapat",
        detail_en: "a fixed picture instead of Bing images with captions and links",
        detail_tr: "açıklamalı ve bağlantılı Bing resimleri yerine sabit bir resim",
        regs: &[r(CDM, "RotatingLockScreenEnabled", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    // ---- AI ---------------------------------------------------------------------------
    Tweak {
        id: "copilot",
        cat: "ai",
        en: "Turn off Windows Copilot",
        tr: "Windows Copilot'ı kapat",
        detail_en: "removes the Copilot button and blocks the Win+C shortcut",
        detail_tr: "Copilot düğmesi kalkar, Win+C kısayolu engellenir",
        regs: &[
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
                D(1),
                Absent,
            ),
            r(
                r"HKCU\Software\Policies\Microsoft\Windows\WindowsCopilot",
                "TurnOffWindowsCopilot",
                D(1),
                Absent,
            ),
            r(ADV, "ShowCopilotButton", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "recall",
        cat: "ai",
        en: "Turn off Recall",
        tr: "Recall'u kapat",
        detail_en: "no screen snapshots on Copilot+ PCs, and Recall cannot be turned on",
        detail_tr: "Copilot+ bilgisayarlarda ekran görüntüsü alınmaz, Recall açılamaz",
        regs: &[
            r(WINAI_LM, "AllowRecallEnablement", D(0), Absent),
            r(WINAI_LM, "DisableAIDataAnalysis", D(1), Absent),
            r(WINAI_LM, "TurnOffSavingSnapshots", D(1), Absent),
            r(WINAI_CU, "DisableAIDataAnalysis", D(1), Absent),
        ],
        tasks: &[],
        os: Os::Win11,
        after: After::Reboot,
    },
    Tweak {
        id: "click-to-do",
        cat: "ai",
        en: "Turn off Click to Do",
        tr: "Click to Do'yu kapat",
        detail_en: "no AI actions on whatever is on screen when you Win+click",
        detail_tr: "Win+tık ile ekrandaki içerik için yapay zekâ eylemleri çıkmaz",
        regs: &[
            r(WINAI_LM, "DisableClickToDo", D(1), Absent),
            r(WINAI_CU, "DisableClickToDo", D(1), Absent),
        ],
        tasks: &[],
        os: Os::Win11,
        after: After::Nothing,
    },
    Tweak {
        id: "notepad-ai",
        cat: "ai",
        en: "Turn off AI in Notepad",
        tr: "Not Defteri'nde yapay zekâyı kapat",
        detail_en: "removes Rewrite and Summarize, so no text is sent away",
        detail_tr: "Yeniden Yaz ve Özetle kalkar, hiçbir metin dışarı gönderilmez",
        regs: &[r(
            r"HKLM\SOFTWARE\Policies\WindowsNotepad",
            "DisableAIFeatures",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Nothing,
    },
    Tweak {
        id: "paint-ai",
        cat: "ai",
        en: "Turn off AI in Paint",
        tr: "Paint'te yapay zekâyı kapat",
        detail_en: "removes Cocreator, Image Creator, generative fill / erase, remove background",
        detail_tr: "Cocreator, Image Creator, üretken doldurma / silme, arka plan kaldırma kalkar",
        regs: &[
            r(PAINT, "DisableCocreator", D(1), Absent),
            r(PAINT, "DisableImageCreator", D(1), Absent),
            r(PAINT, "DisableGenerativeFill", D(1), Absent),
            r(PAINT, "DisableGenerativeErase", D(1), Absent),
            r(PAINT, "DisableRemoveBackground", D(1), Absent),
        ],
        tasks: &[],
        os: Os::Win11,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-ai",
        cat: "ai",
        en: "Turn off Copilot in Edge",
        tr: "Edge'de Copilot'ı kapat",
        detail_en: "no Copilot icon, no page content sent to it, no AI history search",
        detail_tr: "Copilot simgesi yok, sayfa içeriği gönderilmez, yapay zekâ geçmiş araması yok",
        regs: &[
            r(EDGE, "CopilotPageContext", D(0), Absent),
            r(EDGE, "CopilotCDPPageContext", D(0), Absent),
            r(EDGE, "Microsoft365CopilotChatIconEnabled", D(0), Absent),
            r(EDGE, "EdgeHistoryAISearchEnabled", D(0), Absent),
            r(EDGE, "ComposeInlineEnabled", D(0), Absent),
            r(EDGE, "NewTabPageBingChatEnabled", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "ai-service",
        cat: "ai",
        en: "Start the Windows AI service on demand",
        tr: "Windows yapay zekâ hizmetini gerektiğinde başlat",
        detail_en: "WSAIFabricSvc no longer starts with Windows; features that need it still work",
        detail_tr: "WSAIFabricSvc Windows ile başlamaz; ona ihtiyaç duyan özellikler çalışır",
        regs: &[r(
            r"HKLM\SYSTEM\CurrentControlSet\Services\WSAIFabricSvc",
            "Start",
            D(3),
            D(2),
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Reboot,
    },
    // ---- Edge -------------------------------------------------------------------------
    Tweak {
        id: "edge-background",
        cat: "edge",
        en: "Edge does not run in the background",
        tr: "Edge arka planda çalışmasın",
        detail_en: "turns off Startup Boost and background mode; Edge closes when you close it",
        detail_tr: "Başlangıç Hızlandırma ve arka plan modu kapanır; Edge kapatınca kapanır",
        regs: &[
            r(EDGE, "StartupBoostEnabled", D(0), Absent),
            r(EDGE, "BackgroundModeEnabled", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-shopping",
        cat: "edge",
        en: "Turn off Edge shopping",
        tr: "Edge alışveriş özelliğini kapat",
        detail_en: "no price comparison, coupons or cashback pop-ups",
        detail_tr: "fiyat karşılaştırma, kupon ve para iadesi pencereleri çıkmaz",
        regs: &[r(EDGE, "EdgeShoppingAssistantEnabled", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-sidebar",
        cat: "edge",
        en: "Hide the Edge sidebar",
        tr: "Edge kenar çubuğunu gizle",
        detail_en: "removes the app bar on the right side of Edge",
        detail_tr: "Edge'in sağındaki uygulama çubuğu kalkar",
        regs: &[r(EDGE, "HubsSidebarEnabled", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-first-run",
        cat: "edge",
        en: "Skip the Edge welcome pages",
        tr: "Edge karşılama sayfalarını atla",
        detail_en: "no first-run setup and import prompts after Edge updates",
        detail_tr: "Edge güncellemesinden sonra ilk kurulum ve içe aktarma soruları çıkmaz",
        regs: &[
            r(EDGE, "HideFirstRunExperience", D(1), Absent),
            r(EDGE, "ImportOnEachLaunch", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-default-prompt",
        cat: "edge",
        en: "Edge stops asking to be the default",
        tr: "Edge varsayılan olmak için sormasın",
        detail_en: "no \"make Edge your default browser\" bar",
        detail_tr: "\"Edge'i varsayılan tarayıcı yap\" çubuğu çıkmaz",
        regs: &[r(EDGE, "DefaultBrowserSettingEnabled", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "edge-new-tab",
        cat: "edge",
        en: "Plain new tab page in Edge",
        tr: "Edge'de sade yeni sekme sayfası",
        detail_en: "no MSN news feed, sponsored links or default tiles",
        detail_tr: "MSN haber akışı, sponsorlu bağlantılar ve varsayılan kutucuklar yok",
        regs: &[
            r(EDGE, "NewTabPageContentEnabled", D(0), Absent),
            r(EDGE, "NewTabPageHideDefaultTopSites", D(1), Absent),
            r(EDGE, "NewTabPageQuickLinksEnabled", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    // ---- Explorer ---------------------------------------------------------------------
    Tweak {
        id: "file-extensions",
        cat: "explorer",
        en: "Show file extensions",
        tr: "Dosya uzantılarını göster",
        detail_en: "so invoice.pdf.exe shows up as the program it is",
        detail_tr: "fatura.pdf.exe gerçekte olduğu gibi program olarak görünür",
        regs: &[r(ADV, "HideFileExt", D(0), D(1))],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "hidden-files",
        cat: "explorer",
        en: "Show hidden files",
        tr: "Gizli dosyaları göster",
        detail_en: "protected system files stay hidden",
        detail_tr: "korumalı sistem dosyaları gizli kalır",
        regs: &[r(ADV, "Hidden", D(1), D(2))],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "this-pc",
        cat: "explorer",
        en: "Open File Explorer on This PC",
        tr: "Dosya Gezgini Bu Bilgisayar'da açılsın",
        detail_en: "drives first, instead of Home / Quick access",
        detail_tr: "Giriş / Hızlı erişim yerine önce sürücüler",
        regs: &[r(ADV, "LaunchTo", D(1), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "classic-menu",
        cat: "explorer",
        en: "Classic right-click menu",
        tr: "Klasik sağ tık menüsü",
        detail_en: "the full menu right away, without \"show more options\"",
        detail_tr: "\"diğer seçenekleri göster\" olmadan doğrudan tam menü",
        regs: &[r(
            r"HKCU\Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32",
            "",
            S(""),
            NoKey,
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "compact-view",
        cat: "explorer",
        en: "Compact view in File Explorer",
        tr: "Dosya Gezgini'nde sıkı görünüm",
        detail_en: "less space between rows, more files on screen",
        detail_tr: "satırlar arası boşluk azalır, ekrana daha çok dosya sığar",
        regs: &[r(ADV, "UseCompactMode", D(1), Absent)],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "hide-gallery",
        cat: "explorer",
        en: "Hide Gallery in the navigation pane",
        tr: "Gezinti bölmesinde Galeri'yi gizle",
        detail_en: "only the shortcut goes; your photos stay where they are",
        detail_tr: "yalnızca kısayol kalkar; fotoğraflarınız yerinde kalır",
        regs: &[r(
            r"HKCU\Software\Classes\CLSID\{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}",
            "System.IsPinnedToNameSpaceTree",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "hide-home",
        cat: "explorer",
        en: "Hide Home in the navigation pane",
        tr: "Gezinti bölmesinde Giriş'i gizle",
        detail_en: "the page of recent and recommended files",
        detail_tr: "son kullanılan ve önerilen dosyalar sayfası",
        regs: &[r(
            r"HKCU\Software\Classes\CLSID\{f874310e-b6b7-47dc-bc84-b9e6b38f5903}",
            "System.IsPinnedToNameSpaceTree",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "hide-onedrive",
        cat: "explorer",
        en: "Hide OneDrive in the navigation pane",
        tr: "Gezinti bölmesinde OneDrive'ı gizle",
        detail_en: "nothing is unlinked or deleted, the entry is only unpinned",
        detail_tr: "hiçbir şey silinmez ya da bağlantısı kesilmez, yalnızca sabitleme kalkar",
        regs: &[r(
            r"HKCU\Software\Classes\CLSID\{018D5C66-4533-4307-9B53-224DE2ED1FE6}",
            "System.IsPinnedToNameSpaceTree",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    // ---- taskbar ----------------------------------------------------------------------
    Tweak {
        id: "taskbar-left",
        cat: "taskbar",
        en: "Taskbar icons on the left",
        tr: "Görev çubuğu simgeleri solda",
        detail_en: "the Start button back in the corner, as in Windows 10",
        detail_tr: "Başlat düğmesi Windows 10'daki gibi köşeye döner",
        regs: &[r(ADV, "TaskbarAl", D(0), Absent)],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "end-task",
        cat: "taskbar",
        en: "\"End task\" in the taskbar menu",
        tr: "Görev çubuğu menüsünde \"Görevi sonlandır\"",
        detail_en: "right-click a frozen app's button to close it",
        detail_tr: "donan uygulamayı düğmesine sağ tıklayarak kapatın",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\TaskbarDeveloperSettings",
            "TaskbarEndTask",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "search-icon",
        cat: "taskbar",
        en: "Search as an icon only",
        tr: "Arama yalnızca simge olsun",
        detail_en: "the wide search box shrinks to a button; Win+S still works",
        detail_tr: "geniş arama kutusu düğmeye dönüşür; Win+S çalışmaya devam eder",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Search",
            "SearchboxTaskbarMode",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "task-view",
        cat: "taskbar",
        en: "Hide the Task View button",
        tr: "Görev Görünümü düğmesini gizle",
        detail_en: "Win+Tab still opens it",
        detail_tr: "Win+Tab ile yine açılır",
        regs: &[r(ADV, "ShowTaskViewButton", D(0), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "widgets",
        cat: "taskbar",
        en: "Turn off widgets and the news feed",
        tr: "Pencere öğelerini ve haber akışını kapat",
        detail_en: "the weather / news panel on the taskbar and its background process",
        detail_tr: "görev çubuğundaki hava durumu / haber paneli ve arka plan işlemi",
        regs: &[
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Dsh",
                "AllowNewsAndInterests",
                D(0),
                Absent,
            ),
            r(
                r"HKLM\SOFTWARE\Policies\Microsoft\Windows\Windows Feeds",
                "EnableFeeds",
                D(0),
                Absent,
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "clock-seconds",
        cat: "taskbar",
        en: "Show seconds in the clock",
        tr: "Saatte saniyeleri göster",
        detail_en: "the taskbar clock shows seconds",
        detail_tr: "görev çubuğu saati saniyeleri gösterir",
        regs: &[r(ADV, "ShowSecondsInSystemClock", D(1), Absent)],
        tasks: &[],
        os: Os::Win11,
        after: After::Explorer,
    },
    Tweak {
        id: "last-active-click",
        cat: "taskbar",
        en: "Click opens the last used window",
        tr: "Tıklama son kullanılan pencereyi açsın",
        detail_en: "for apps with several windows, instead of the thumbnail row",
        detail_tr: "birden çok pencereli uygulamalarda küçük resim satırı yerine",
        regs: &[r(ADV, "LastActiveClick", D(1), Absent)],
        tasks: &[],
        os: Os::Any,
        after: After::Explorer,
    },
    Tweak {
        id: "meet-now",
        cat: "taskbar",
        en: "Hide the Meet Now button",
        tr: "Şimdi Toplantı Yap düğmesini gizle",
        detail_en: "the Skype video call icon next to the clock",
        detail_tr: "saatin yanındaki Skype görüntülü arama simgesi",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer",
            "HideSCAMeetNow",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Win10,
        after: After::Explorer,
    },
    Tweak {
        id: "tray-icons",
        cat: "taskbar",
        en: "Show all tray icons",
        tr: "Tüm bildirim simgelerini göster",
        detail_en: "no icons hidden behind the ^ arrow",
        detail_tr: "^ okunun arkasına gizlenen simge kalmaz",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer",
            "EnableAutoTray",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Win10,
        after: After::Explorer,
    },
    // ---- look and feel ------------------------------------------------------------------
    Tweak {
        id: "dark-mode",
        cat: "look",
        en: "Dark mode",
        tr: "Koyu mod",
        detail_en: "Windows and apps use the dark colours",
        detail_tr: "Windows ve uygulamalar koyu renkleri kullanır",
        regs: &[
            r(PERSONALIZE, "AppsUseLightTheme", D(0), D(1)),
            r(PERSONALIZE, "SystemUsesLightTheme", D(0), D(1)),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "transparency",
        cat: "look",
        en: "Turn off transparency",
        tr: "Saydamlığı kapat",
        detail_en: "solid taskbar and Start menu; a little less work for weak graphics",
        detail_tr: "düz görev çubuğu ve Başlat menüsü; zayıf ekran kartına daha az yük",
        regs: &[r(PERSONALIZE, "EnableTransparency", D(0), D(1))],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "animations",
        cat: "look",
        en: "Turn off window animations",
        tr: "Pencere animasyonlarını kapat",
        detail_en: "windows and taskbar buttons appear at once instead of sliding",
        detail_tr: "pencereler ve görev çubuğu düğmeleri kaymadan hemen görünür",
        regs: &[
            r(
                r"HKCU\Control Panel\Desktop\WindowMetrics",
                "MinAnimate",
                S("0"),
                S("1"),
            ),
            r(ADV, "TaskbarAnimations", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    Tweak {
        id: "menu-delay",
        cat: "look",
        en: "Faster menus",
        tr: "Daha hızlı menüler",
        detail_en: "submenus open after 100 ms instead of 400 ms",
        detail_tr: "alt menüler 400 ms yerine 100 ms sonra açılır",
        regs: &[r(
            r"HKCU\Control Panel\Desktop",
            "MenuShowDelay",
            S("100"),
            S("400"),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    // ---- gaming and input ---------------------------------------------------------------
    Tweak {
        id: "game-dvr",
        cat: "gaming",
        en: "Turn off background game recording",
        tr: "Arka planda oyun kaydını kapat",
        detail_en: "Game DVR stops recording the last minutes of play; Game Mode stays",
        detail_tr: "Game DVR son dakikaları kaydetmez; Oyun Modu yerinde kalır",
        regs: &[
            r(
                r"HKCU\System\GameConfigStore",
                "GameDVR_Enabled",
                D(0),
                D(1),
            ),
            r(
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR",
                "AppCaptureEnabled",
                D(0),
                Absent,
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "game-bar-tips",
        cat: "gaming",
        en: "Quiet Game Bar",
        tr: "Sessiz Game Bar",
        detail_en: "no pop-up when a game starts, and a controller's Xbox button does not open it",
        detail_tr: "oyun başlarken pencere çıkmaz, oyun kumandasının Xbox düğmesi onu açmaz",
        regs: &[
            r(
                r"HKCU\Software\Microsoft\GameBar",
                "ShowStartupPanel",
                D(0),
                Absent,
            ),
            r(
                r"HKCU\Software\Microsoft\GameBar",
                "UseNexusForGameBarEnabled",
                D(0),
                Absent,
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "mouse-acceleration",
        cat: "gaming",
        en: "Turn off mouse acceleration",
        tr: "Fare ivmesini kapat",
        detail_en: "\"enhance pointer precision\" off: the same move, the same distance",
        detail_tr: "\"işaretçi hassasiyetini artır\" kapalı: aynı hareket, aynı mesafe",
        regs: &[
            r(r"HKCU\Control Panel\Mouse", "MouseSpeed", S("0"), S("1")),
            r(
                r"HKCU\Control Panel\Mouse",
                "MouseThreshold1",
                S("0"),
                S("6"),
            ),
            r(
                r"HKCU\Control Panel\Mouse",
                "MouseThreshold2",
                S("0"),
                S("10"),
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    Tweak {
        id: "sticky-keys",
        cat: "gaming",
        en: "No Sticky Keys prompt",
        tr: "Yapışkan Tuşlar sorusu çıkmasın",
        detail_en: "pressing Shift five times no longer interrupts a game",
        detail_tr: "Shift'e beş kez basmak artık oyunu bölmez",
        regs: &[r(
            r"HKCU\Control Panel\Accessibility\StickyKeys",
            "Flags",
            S("506"),
            S("510"),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    Tweak {
        id: "power-throttling",
        cat: "gaming",
        en: "Turn off power throttling",
        tr: "Güç kısıtlamayı kapat",
        detail_en: "background programs get the full CPU speed; costs battery on laptops",
        detail_tr: "arka plan programları tam işlemci hızı alır; dizüstünde pil harcar",
        regs: &[r(
            r"HKLM\SYSTEM\CurrentControlSet\Control\Power\PowerThrottling",
            "PowerThrottlingOff",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Reboot,
    },
    // ---- system ---------------------------------------------------------------------
    Tweak {
        id: "bsod-details",
        cat: "system",
        en: "Show details on blue screens",
        tr: "Mavi ekranda ayrıntıları göster",
        detail_en: "the stop code and its parameters instead of the sad face",
        detail_tr: "üzgün yüz yerine durma kodu ve parametreleri",
        regs: &[
            r(
                r"HKLM\SYSTEM\CurrentControlSet\Control\CrashControl",
                "DisplayParameters",
                D(1),
                Absent,
            ),
            r(
                r"HKLM\SYSTEM\CurrentControlSet\Control\CrashControl",
                "DisableEmoticon",
                D(1),
                Absent,
            ),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "verbose-status",
        cat: "system",
        en: "Detailed startup and shutdown messages",
        tr: "Ayrıntılı açılış ve kapanış iletileri",
        detail_en: "shows what Windows waits for instead of \"please wait\"",
        detail_tr: "\"lütfen bekleyin\" yerine Windows'un neyi beklediği görünür",
        regs: &[r(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
            "VerboseStatus",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "long-paths",
        cat: "system",
        en: "Allow long file paths",
        tr: "Uzun dosya yollarına izin ver",
        detail_en: "lifts the 260 character limit for programs that support it",
        detail_tr: "destekleyen programlar için 260 karakter sınırı kalkar",
        regs: &[r(
            r"HKLM\SYSTEM\CurrentControlSet\Control\FileSystem",
            "LongPathsEnabled",
            D(1),
            D(0),
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Reboot,
    },
    Tweak {
        id: "background-apps",
        cat: "system",
        en: "Store apps do not run in the background",
        tr: "Store uygulamaları arka planda çalışmasın",
        detail_en: "saves memory and battery; their notifications may arrive late",
        detail_tr: "bellek ve pil tasarrufu; bildirimleri gecikebilir",
        regs: &[r(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications",
            "GlobalUserDisabled",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::SignOut,
    },
    // ---- Windows Update ---------------------------------------------------------------
    Tweak {
        id: "update-no-reboot",
        cat: "updates",
        en: "No automatic restart while signed in",
        tr: "Oturum açıkken otomatik yeniden başlatma yok",
        detail_en: "updates wait for you to restart, open work is never lost",
        detail_tr: "güncellemeler yeniden başlatmanızı bekler, açık işler kaybolmaz",
        regs: &[r(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsUpdate\AU",
            "NoAutoRebootWithLoggedOnUsers",
            D(1),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "update-p2p",
        cat: "updates",
        en: "Download updates only from Microsoft",
        tr: "Güncellemeleri yalnızca Microsoft'tan indir",
        detail_en: "no sharing update parts with other PCs over the internet",
        detail_tr: "güncelleme parçaları internetteki başka bilgisayarlarla paylaşılmaz",
        regs: &[r(
            r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization",
            "DODownloadMode",
            D(0),
            Absent,
        )],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
    Tweak {
        id: "update-not-early",
        cat: "updates",
        en: "Do not get updates early",
        tr: "Güncellemeleri erken alma",
        detail_en: "turns off \"get the latest updates as soon as they're available\"",
        detail_tr: "\"en son güncellemeleri hazır olur olmaz al\" kapanır",
        regs: &[r(WU_UX, "IsContinuousInnovationOptedIn", D(0), Absent)],
        tasks: &[],
        os: Os::Win11,
        after: After::Nothing,
    },
    Tweak {
        id: "update-pause",
        cat: "updates",
        en: "Allow pausing updates longer",
        tr: "Güncellemeleri daha uzun duraklatmaya izin ver",
        detail_en: "Settings offers longer pauses than five weeks",
        detail_tr: "Ayarlar beş haftadan uzun duraklatma seçenekleri sunar",
        regs: &[
            r(WU_UX, "FlightSettingsMaxPauseDays", D(700), Absent),
            r(WU, "SetDisablePauseUXAccess", D(0), Absent),
        ],
        tasks: &[],
        os: Os::Any,
        after: After::Nothing,
    },
];

// ---- registry state -----------------------------------------------------------------

/// A value as it is in the registry now (also what tweaks.json stores).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
struct Cur {
    /// the key exists
    k: bool,
    /// value kind (DWord, String, ...) when the value exists
    #[serde(default, skip_serializing_if = "Option::is_none")]
    t: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    v: Option<String>,
}

#[derive(Deserialize)]
struct State {
    build: u32,
    regs: Vec<Cur>,
}

/// One backed-up tweak in tweaks.json.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Saved {
    at: String,
    regs: Vec<Cur>,
    /// scheduled tasks kazma turned off (they were on)
    #[serde(default)]
    tasks: Vec<String>,
}

fn is_service(key: &str) -> bool {
    key.to_ascii_lowercase()
        .contains(r"\currentcontrolset\services\")
}

/// Does the value as it is now equal `want`?
fn matches(key: &str, cur: &Cur, want: V) -> bool {
    if !cur.k && is_service(key) {
        return true; // the service is not on this PC: nothing to do
    }
    match want {
        NoKey => !cur.k,
        Absent => cur.t.is_none(),
        D(n) => {
            cur.t.as_deref() == Some("DWord")
                && cur.v.as_deref().and_then(|v| v.parse::<u32>().ok()) == Some(n)
        }
        S(s) => {
            matches!(cur.t.as_deref(), Some("String" | "ExpandString"))
                && cur.v.as_deref() == Some(s)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum On {
    Yes,
    No,
    Partly,
}

fn state_of(t: &Tweak, cur: &[Cur]) -> On {
    let hits = t
        .regs
        .iter()
        .zip(cur)
        .filter(|(r, c)| matches(r.key, c, r.set))
        .count();
    if hits == t.regs.len() {
        On::Yes
    } else if hits == 0 {
        On::No
    } else {
        On::Partly
    }
}

fn needs_admin(t: &Tweak) -> bool {
    !t.tasks.is_empty()
        || t.regs.iter().any(|r| {
            r.key.starts_with("HKLM") || r.key.to_ascii_lowercase().contains(r"\policies\")
        })
}

fn available(t: &Tweak, build: u32) -> bool {
    match t.os {
        Os::Any => true,
        Os::Win10 => build < 22000,
        Os::Win11 => build >= 22000,
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// `HKCU\x` -> `Registry::HKEY_CURRENT_USER\x`, a path PowerShell takes literally.
fn ps_path(key: &str) -> String {
    let (hive, rest) = key.split_once('\\').unwrap_or((key, ""));
    let long = match hive {
        "HKCU" => "HKEY_CURRENT_USER",
        "HKLM" => "HKEY_LOCAL_MACHINE",
        other => other,
    };
    ps_quote(&format!("Registry::{long}\\{rest}"))
}

const READ_FN: &str = r#"
function KzGet($p, $n) {
  $k = Get-Item -LiteralPath $p -ErrorAction SilentlyContinue
  if (-not $k) { return @{ k = $false } }
  $v = $k.GetValue($n, $null, 'DoNotExpandEnvironmentNames')
  if ($null -eq $v) { return @{ k = $true } }
  $t = $k.GetValueKind($n).ToString()
  if ($t -eq 'DWord') { $v = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]$v), 0) }
  @{ k = $true; t = $t; v = [string]$v }
}
$l = New-Object System.Collections.ArrayList
"#;

/// PowerShell printing {"build":N,"regs":[...]} with every value of every tweak, in order.
fn read_script() -> String {
    let mut s = String::from(READ_FN);
    for t in TWEAKS {
        for r in t.regs {
            s.push_str(&format!(
                "[void]$l.Add((KzGet {} {}))\n",
                ps_path(r.key),
                ps_quote(r.name)
            ));
        }
    }
    s.push_str(
        "$b = [int](Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion').CurrentBuildNumber\n\
         ConvertTo-Json -InputObject @{ build = $b; regs = @($l) } -Compress -Depth 4\n",
    );
    s
}

/// The current value of every registry value of every tweak, and the Windows build.
fn read_state() -> anyhow::Result<(u32, Vec<Vec<Cur>>)> {
    let out = ps::powershell(&read_script(), &[])?;
    if !out.status.success() {
        anyhow::bail!("{}", ps::last_line(&String::from_utf8_lossy(&out.stderr)));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let st: State = serde_json::from_str(text.trim().trim_start_matches('\u{feff}'))?;
    let mut it = st.regs.into_iter();
    let per = TWEAKS
        .iter()
        .map(|t| {
            t.regs
                .iter()
                .map(|_| it.next().unwrap_or_default())
                .collect()
        })
        .collect();
    Ok((st.build, per))
}

const WRITE_FN: &str = r#"
function KzSet($p, $n, $t, $v, $mk) {
  if (-not (Test-Path -LiteralPath $p)) {
    if (-not $mk) { return }
    New-Item -Path $p -Force | Out-Null
  }
  if ($n -eq '') { Set-Item -LiteralPath $p -Value $v }
  else { New-ItemProperty -LiteralPath $p -Name $n -PropertyType $t -Value $v -Force | Out-Null }
}
function KzDel($p, $n) {
  if (Test-Path -LiteralPath $p) { Remove-ItemProperty -LiteralPath $p -Name $n -ErrorAction SilentlyContinue }
}
function KzDelKey($p) {
  if (-not (Test-Path -LiteralPath $p)) { return }
  Remove-Item -LiteralPath $p -Recurse -Force
  # the parent kazma created along with it, if nothing else is in it
  $up = Get-Item -LiteralPath $p.Substring(0, $p.LastIndexOf('\')) -ErrorAction SilentlyContinue
  if ($up -and $up.SubKeyCount -eq 0 -and $up.ValueCount -eq 0) { Remove-Item -LiteralPath $up.PSPath -Force }
}
function KzTaskOff($t) {
  $i = $t.LastIndexOf('\')
  $x = Get-ScheduledTask -TaskPath $t.Substring(0, $i + 1) -TaskName $t.Substring($i + 1) -ErrorAction SilentlyContinue
  if ($x -and $x.State -ne 'Disabled') {
    $null = $x | Disable-ScheduledTask -ErrorAction SilentlyContinue
    "TASK $t"
  }
}
function KzTaskOn($t) {
  $i = $t.LastIndexOf('\')
  $x = Get-ScheduledTask -TaskPath $t.Substring(0, $i + 1) -TaskName $t.Substring($i + 1) -ErrorAction SilentlyContinue
  if ($x) { $null = $x | Enable-ScheduledTask -ErrorAction SilentlyContinue }
}
"#;

/// One registry write. `kind`/`value` as PowerShell takes them.
enum Op {
    Set(&'static str, &'static str, &'static str, String),
    DelValue(&'static str, &'static str),
    DelKey(&'static str),
}

fn op_for(r: &Reg, v: V) -> Op {
    match v {
        D(n) => Op::Set(r.key, r.name, "DWord", n.to_string()),
        S(s) => Op::Set(r.key, r.name, "String", s.to_string()),
        Absent => Op::DelValue(r.key, r.name),
        NoKey => Op::DelKey(r.key),
    }
}

/// What restores a saved value; None for value kinds kazma does not write back.
fn op_from_saved(r: &Reg, c: &Cur) -> Option<Op> {
    if !c.k {
        return Some(if r.default == NoKey {
            Op::DelKey(r.key)
        } else {
            Op::DelValue(r.key, r.name)
        });
    }
    let kind = match c.t.as_deref() {
        None => return Some(Op::DelValue(r.key, r.name)),
        Some("DWord") => "DWord",
        Some("QWord") => "QWord",
        Some("String") => "String",
        Some("ExpandString") => "ExpandString",
        Some(_) => return None,
    };
    Some(Op::Set(
        r.key,
        r.name,
        kind,
        c.v.clone().unwrap_or_default(),
    ))
}

fn ops_script(ops: &[Op]) -> String {
    let mut s = String::from(WRITE_FN);
    for op in ops {
        match op {
            Op::Set(key, name, kind, value) => {
                let value = if matches!(*kind, "DWord" | "QWord") {
                    value.clone() // a number, written as is
                } else {
                    ps_quote(value)
                };
                s.push_str(&format!(
                    "KzSet {} {} {kind} {value} ${}\n",
                    ps_path(key),
                    ps_quote(name),
                    !is_service(key)
                ));
            }
            Op::DelValue(key, name) => {
                s.push_str(&format!("KzDel {} {}\n", ps_path(key), ps_quote(name)))
            }
            Op::DelKey(key) => s.push_str(&format!("KzDelKey {}\n", ps_path(key))),
        }
    }
    s
}

// ---- backups --------------------------------------------------------------------------

fn backup_file() -> Option<PathBuf> {
    oplog::data_dir().map(|d| d.join("tweaks.json"))
}

fn load_backups() -> BTreeMap<String, Saved> {
    backup_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_backups(map: &BTreeMap<String, Saved>) -> anyhow::Result<()> {
    let f = backup_file().ok_or_else(|| anyhow::anyhow!("no settings folder"))?;
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = f.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(map)?)?;
    std::fs::rename(&tmp, &f)?;
    Ok(())
}

// ---- applying ---------------------------------------------------------------------------

fn name(t: &Tweak) -> &'static str {
    ui::t(t.en, t.tr)
}

fn detail(t: &Tweak) -> &'static str {
    ui::t(t.detail_en, t.detail_tr)
}

fn cat_name(key: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|c| c.0 == key)
        .map(|c| ui::t(c.1, c.2))
        .unwrap_or("")
}

fn first_error(out: &std::process::Output) -> String {
    let err = String::from_utf8_lossy(&out.stderr);
    err.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("failed")
        .trim()
        .to_string()
}

/// Turn one tweak on, saving the values that were there first.
fn apply(i: usize, cur: &[Cur], backups: &mut BTreeMap<String, Saved>) -> anyhow::Result<()> {
    let t = &TWEAKS[i];
    // keep the oldest backup: that is what was there before kazma touched it
    if !backups.contains_key(t.id) {
        backups.insert(
            t.id.to_string(),
            Saved {
                at: oplog::now_utc(),
                regs: cur.to_vec(),
                tasks: vec![],
            },
        );
        save_backups(backups)?;
    }
    let ops: Vec<Op> = t.regs.iter().map(|r| op_for(r, r.set)).collect();
    let mut script = ops_script(&ops);
    for task in t.tasks {
        script.push_str(&format!("KzTaskOff {}\n", ps_quote(task)));
    }
    let out = ps::powershell(&script, &[])?;
    let off: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().strip_prefix("TASK ").map(str::to_string))
        .collect();
    if !off.is_empty()
        && let Some(s) = backups.get_mut(t.id)
    {
        for task in off {
            if !s.tasks.contains(&task) {
                s.tasks.push(task);
            }
        }
        save_backups(backups)?;
    }
    if !out.status.success() {
        anyhow::bail!("{}", first_error(&out));
    }
    Ok(())
}

/// Turn one tweak off: back to the saved values, or to the Windows default.
fn undo(i: usize, backups: &mut BTreeMap<String, Saved>) -> anyhow::Result<()> {
    let t = &TWEAKS[i];
    let saved = backups.get(t.id);
    let ops: Vec<Op> = t
        .regs
        .iter()
        .enumerate()
        .map(|(j, r)| {
            saved
                .and_then(|s| s.regs.get(j))
                .and_then(|c| op_from_saved(r, c))
                .unwrap_or_else(|| op_for(r, r.default))
        })
        .collect();
    let mut script = ops_script(&ops);
    // tasks kazma turned off; without a backup every listed task (on by default)
    let tasks: Vec<String> = match saved {
        Some(s) => s.tasks.clone(),
        None => t.tasks.iter().map(|s| s.to_string()).collect(),
    };
    for task in &tasks {
        script.push_str(&format!("KzTaskOn {}\n", ps_quote(task)));
    }
    let out = ps::powershell(&script, &[])?;
    if !out.status.success() {
        anyhow::bail!("{}", first_error(&out));
    }
    if backups.remove(t.id).is_some() {
        save_backups(backups)?;
    }
    Ok(())
}

// ---- command ------------------------------------------------------------------------------

/// Tweak indexes named by `args`: ids, category keys, or `all`.
fn resolve(args: &[String], all: impl Fn(usize) -> bool) -> anyhow::Result<Vec<usize>> {
    let mut out = vec![];
    for a in args {
        let a = a.to_lowercase();
        let hit: Vec<usize> = if a == "all" {
            (0..TWEAKS.len()).filter(|&i| all(i)).collect()
        } else if CATEGORIES.iter().any(|c| c.0 == a) {
            (0..TWEAKS.len()).filter(|&i| TWEAKS[i].cat == a).collect()
        } else if let Some(i) = TWEAKS.iter().position(|t| t.id == a) {
            vec![i]
        } else {
            anyhow::bail!(
                "{}",
                tr!(
                    "unknown setting: {a} (see kz tweak list)",
                    "bilinmeyen ayar: {a} (bkz. kz tweak list)"
                )
            );
        };
        for i in hit {
            if !out.contains(&i) {
                out.push(i);
            }
        }
    }
    Ok(out)
}

fn list(build: u32, cur: &[Vec<Cur>], filter: &[usize]) {
    let admin = sys::is_admin();
    let idw = TWEAKS.iter().map(|t| t.id.len()).max().unwrap_or(0);
    let room = ui::term_width().saturating_sub(2 + 5 + idw + 2 + 2);
    let mut marked = false;
    for (key, _, _) in CATEGORIES {
        let rows: Vec<usize> = (0..TWEAKS.len())
            .filter(|&i| {
                TWEAKS[i].cat == *key
                    && available(&TWEAKS[i], build)
                    && (filter.is_empty() || filter.contains(&i))
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        ui::section(cat_name(key));
        for i in rows {
            let t = &TWEAKS[i];
            let star = if !admin && needs_admin(t) {
                marked = true;
                " *"
            } else {
                ""
            };
            let text = ui::truncate_end(&format!("{}{star}", name(t)), room);
            let line = format!("{} {text}", ui::pad(t.id, idw + 1));
            match state_of(t, &cur[i]) {
                On::Yes => ui::ok(&line),
                On::Partly => ui::info(&line),
                On::No => ui::skip(&line),
            }
        }
    }
    println!();
    println!(
        "  {}",
        tr!(
            "[ok] on   [..] partly on   [--] off",
            "[ok] açık   [..] kısmen açık   [--] kapalı"
        )
    );
    if marked {
        println!(
            "  {}",
            tr!(
                "* needs an elevated terminal (Run as administrator)",
                "* yönetici terminali gerekir (Yönetici olarak çalıştır)"
            )
        );
    }
    println!(
        "  {}\n",
        tr!(
            "change with: kz tweak apply ID   kz tweak undo ID   (or just kz tweak)",
            "değiştirmek için: kz tweak apply ID   kz tweak undo ID   (ya da yalnızca kz tweak)"
        )
    );
}

/// Ready-made sets: (key, English, Turkish, setting ids).
pub const PROFILES: &[(&str, &str, &str, &[&str])] = &[
    (
        "recommended",
        "safe choices for almost everyone",
        "neredeyse herkes için güvenli seçimler",
        &[
            "feedback",
            "activity-history",
            "app-tracking",
            "language-list",
            "consumer-features",
            "finish-setup",
            "welcome-experience",
            "edge-shopping",
            "edge-first-run",
            "edge-default-prompt",
            "file-extensions",
            "end-task",
            "bsod-details",
            "update-no-reboot",
        ],
    ),
    (
        "privacy",
        "send Microsoft as little as possible, AI features off",
        "Microsoft'a olabildiğince az veri, yapay zekâ özellikleri kapalı",
        &[
            "telemetry",
            "feedback",
            "error-reporting",
            "activity-history",
            "app-tracking",
            "location",
            "online-speech",
            "typing-data",
            "language-list",
            "device-metadata",
            "consumer-features",
            "spotlight",
            "copilot",
            "recall",
            "click-to-do",
            "edge-ai",
            "notepad-ai",
            "paint-ai",
        ],
    ),
    (
        "gaming",
        "fewer interruptions and background work while playing",
        "oyun sırasında daha az kesinti ve arka plan işi",
        &[
            "game-dvr",
            "game-bar-tips",
            "mouse-acceleration",
            "sticky-keys",
            "power-throttling",
            "background-apps",
        ],
    ),
    (
        "classic",
        "File Explorer and taskbar the way Windows 10 had them",
        "Dosya Gezgini ve görev çubuğu Windows 10'daki gibi",
        &[
            "file-extensions",
            "this-pc",
            "classic-menu",
            "compact-view",
            "taskbar-left",
            "search-icon",
            "task-view",
            "end-task",
        ],
    ),
];

/// Ids of the settings that are on now (for `export` and `kz backup`).
pub fn applied_ids() -> anyhow::Result<Vec<String>> {
    let (build, cur) = read_state()?;
    Ok(TWEAKS
        .iter()
        .enumerate()
        .filter(|(i, t)| available(t, build) && state_of(t, &cur[*i]) == On::Yes)
        .map(|(_, t)| t.id.to_string())
        .collect())
}

/// Setting ids in a file written by `export` (one per line, # comments).
pub fn parse_ids(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_lowercase())
        .filter(|l| TWEAKS.iter().any(|t| t.id == l))
        .collect()
}

fn list_profiles() {
    let kw = PROFILES.iter().map(|p| p.0.len()).max().unwrap_or(0) + 2;
    for (key, en, trd, ids) in PROFILES {
        ui::info(&format!(
            "{}{}  ({})",
            ui::pad(key, kw),
            ui::t(en, trd),
            tr!("{} settings", "{} ayar", ids.len())
        ));
    }
    println!(
        "\n  {}\n",
        tr!(
            "apply one: kz tweak profile gaming   (--dry-run shows what would change)",
            "uygulamak için: kz tweak profile gaming   (--dry-run ne değişeceğini gösterir)"
        )
    );
}

pub fn run(
    action: Option<String>,
    ids: Vec<String>,
    dry: bool,
    no_restore_point: bool,
) -> anyhow::Result<()> {
    // profile / import / export are apply and list in other clothes
    let (action, ids) = match action.as_deref() {
        Some("profile") | Some("profiles") => {
            let Some(name) = ids.first() else {
                ui::print_banner(&tr!(
                    "tweak :: ready-made sets of settings",
                    "tweak :: hazır ayar setleri"
                ));
                list_profiles();
                return Ok(());
            };
            let Some(p) = PROFILES.iter().find(|p| p.0 == name.to_lowercase()) else {
                anyhow::bail!(
                    "{}",
                    tr!(
                        "unknown profile: {name}. use recommended | privacy | gaming | classic",
                        "bilinmeyen profil: {name}. recommended | privacy | gaming | classic kullanın"
                    )
                );
            };
            (
                Some("apply".to_string()),
                p.3.iter().map(|s| s.to_string()).collect(),
            )
        }
        Some("import") => {
            let file = ids
                .first()
                .ok_or_else(|| anyhow::anyhow!("kz tweak import FILE"))?;
            let list = parse_ids(&std::fs::read_to_string(crate::whitelist::expand(file))?);
            if list.is_empty() {
                anyhow::bail!(
                    "{}",
                    tr!("no setting ids in {file}", "{file} içinde ayar kimliği yok")
                );
            }
            (Some("apply".to_string()), list)
        }
        Some("export") => {
            ui::print_banner(&tr!(
                "tweak :: save the settings that are on",
                "tweak :: açık ayarları kaydet"
            ));
            let file = ids
                .first()
                .ok_or_else(|| anyhow::anyhow!("kz tweak export FILE"))?;
            let on = applied_ids()?;
            let path = crate::whitelist::expand(file);
            std::fs::write(
                &path,
                format!(
                    "# kazma tweak settings, apply with: kz tweak import FILE\n{}\n",
                    on.join("\n")
                ),
            )?;
            ui::ok(&tr!(
                "{} settings saved to {}",
                "{} ayar {} dosyasına kaydedildi",
                on.len(),
                path.display()
            ));
            println!();
            return Ok(());
        }
        _ => (action, ids),
    };
    run_inner(action, ids, dry, no_restore_point)
}

fn run_inner(
    action: Option<String>,
    ids: Vec<String>,
    dry: bool,
    no_restore_point: bool,
) -> anyhow::Result<()> {
    ui::print_banner(&tr!(
        "tweak :: Windows settings, every change reversible",
        "tweak :: Windows ayarları, her değişiklik geri alınabilir"
    ));
    if !cfg!(windows) {
        ui::warn(&tr!(
            "kz tweak works on Windows only.",
            "kz tweak yalnızca Windows'ta çalışır."
        ));
        return Ok(());
    }
    ps::working(&tr!("reading settings...", "ayarlar okunuyor..."));
    let (build, cur) = read_state()?;
    println!("{:60}", "");
    let backups = load_backups();

    // (index, turn on?) for every change to make
    let changes: Vec<(usize, bool)> = match action.as_deref() {
        Some("list") | Some("ls") => {
            let filter = resolve(&ids, |_| true)?;
            list(build, &cur, &filter);
            return Ok(());
        }
        Some("apply") | Some("on") => {
            if ids.is_empty() {
                anyhow::bail!("kz tweak apply ID...");
            }
            resolve(&ids, |_| true)?
                .into_iter()
                .filter(|&i| available(&TWEAKS[i], build))
                .filter(|&i| state_of(&TWEAKS[i], &cur[i]) != On::Yes)
                .map(|i| (i, true))
                .collect()
        }
        Some("undo") | Some("off") => {
            if ids.is_empty() {
                anyhow::bail!("kz tweak undo ID...   (kz tweak undo all)");
            }
            // `all` means everything kazma changed, not every setting in the list
            resolve(&ids, |i| backups.contains_key(TWEAKS[i].id))?
                .into_iter()
                .filter(|&i| {
                    backups.contains_key(TWEAKS[i].id) || state_of(&TWEAKS[i], &cur[i]) != On::No
                })
                .map(|i| (i, false))
                .collect()
        }
        Some(other) => anyhow::bail!(
            "{}",
            tr!(
                "unknown action: {other}. use list | apply | undo | profile | export | import",
                "bilinmeyen işlem: {other}. list | apply | undo | profile | export | import kullanın"
            )
        ),
        None if dry || !ui::interactive() => {
            list(build, &cur, &[]);
            return Ok(());
        }
        None => {
            let admin = sys::is_admin();
            let order: Vec<usize> = CATEGORIES
                .iter()
                .flat_map(|(key, _, _)| {
                    (0..TWEAKS.len())
                        .filter(|&i| TWEAKS[i].cat == *key && available(&TWEAKS[i], build))
                        .collect::<Vec<_>>()
                })
                .collect();
            let was: Vec<bool> = order
                .iter()
                .map(|&i| state_of(&TWEAKS[i], &cur[i]) == On::Yes)
                .collect();
            let items = order
                .iter()
                .zip(&was)
                .map(|(&i, &on)| {
                    let t = &TWEAKS[i];
                    let lock = if !admin && needs_admin(t) {
                        tr!(" (admin)", " (yönetici)")
                    } else {
                        String::new()
                    };
                    ui::CheckItem {
                        label: name(t).to_string(),
                        detail: format!("{}{lock}", t.id),
                        size: 0,
                        checked: on,
                    }
                })
                .collect();
            let notes: Vec<String> = order
                .iter()
                .map(|&i| format!("{} :: {}", cat_name(TWEAKS[i].cat), detail(&TWEAKS[i])))
                .collect();
            let Some(sel) = ui::checklist_explained(
                &tr!(
                    "kazma tweak :: [x] = on, enter applies the changes",
                    "kazma tweak :: [x] = açık, enter değişiklikleri uygular"
                ),
                items,
                &notes,
            )?
            else {
                println!("  {}", tr!("cancelled.", "iptal edildi."));
                return Ok(());
            };
            order
                .iter()
                .zip(was.iter().zip(sel))
                .filter(|(_, (was, now))| *was != now)
                .map(|(&i, (_, now))| (i, now))
                .collect()
        }
    };

    if changes.is_empty() {
        ui::ok(&tr!("Nothing to change.", "Değiştirilecek bir şey yok."));
        return Ok(());
    }

    if dry {
        ui::section(&tr!("Would change", "Değiştirilecekler"));
        for &(i, on) in &changes {
            let verb = if on {
                tr!("[on] ", "[aç] ")
            } else {
                tr!("[off]", "[kapat]")
            };
            ui::info(&format!("{verb} {}  {}", TWEAKS[i].id, name(&TWEAKS[i])));
        }
        println!();
        return Ok(());
    }

    let admin = sys::is_admin();
    let (allowed, refused): (Vec<_>, Vec<_>) = changes
        .into_iter()
        .partition(|&(i, _)| admin || !needs_admin(&TWEAKS[i]));
    if !refused.is_empty() {
        ui::section(&tr!(
            "Skipped: these need an elevated terminal",
            "Atlandı: bunlar için yönetici terminali gerekir"
        ));
        for &(i, _) in &refused {
            ui::skip(&format!("{}  {}", TWEAKS[i].id, name(&TWEAKS[i])));
        }
    }
    if allowed.is_empty() {
        println!();
        return Ok(());
    }

    if admin && !no_restore_point && allowed.iter().any(|&(i, _)| needs_admin(&TWEAKS[i])) {
        ps::working(&tr!(
            "creating a restore point...",
            "geri yükleme noktası oluşturuluyor..."
        ));
        match ps::powershell(
            "Checkpoint-Computer -Description 'kazma: before tweak' -RestorePointType MODIFY_SETTINGS -WarningAction SilentlyContinue",
            &[],
        ) {
            Ok(o) if o.status.success() => ui::ok(&tr!(
                "restore point created                ",
                "geri yükleme noktası oluşturuldu       "
            )),
            _ => ui::skip(&tr!(
                "no new restore point (off, or one was made in the last 24 h)",
                "yeni geri yükleme noktası yok (kapalı ya da son 24 saatte oluşturulmuş)"
            )),
        }
    }

    ui::section(&tr!("Applying", "Uygulanıyor"));
    let mut backups = backups;
    let mut after = After::Nothing;
    for (i, on) in allowed {
        let t = &TWEAKS[i];
        let verb = if on {
            tr!("[on] ", "[aç] ")
        } else {
            tr!("[off]", "[kapat]")
        };
        let label = format!("{verb} {}  {}", t.id, name(t));
        let res = if on {
            apply(i, &cur[i], &mut backups)
        } else {
            undo(i, &mut backups)
        };
        match res {
            Ok(()) => {
                ui::ok(&label);
                oplog::record(
                    "tweak",
                    &format!("{} {}", if on { "on" } else { "off" }, t.id),
                );
                after = after.max(t.after);
            }
            Err(e) => ui::warn(&format!("{label}  ({e})")),
        }
    }
    println!();
    match after {
        After::Nothing => {}
        After::Explorer => {
            if ui::interactive()
                && crate::remove::ask(&tr!(
                    "Restart Explorer now so the changes show?",
                    "Değişikliklerin görünmesi için Gezgin şimdi yeniden başlatılsın mı?"
                ))
            {
                match ps::powershell(crate::optimize::RESTART_EXPLORER, &[]) {
                    Ok(o) if o.status.success() => {
                        ui::ok(&tr!("Explorer restarted", "Gezgin yeniden başlatıldı"))
                    }
                    _ => ui::warn(&tr!(
                        "could not restart Explorer, sign out and back in instead",
                        "Gezgin yeniden başlatılamadı, oturumu kapatıp açın"
                    )),
                }
            } else {
                println!(
                    "  {}",
                    tr!(
                        "some changes show after Explorer restarts or you sign in again.",
                        "bazı değişiklikler Gezgin yeniden başlayınca ya da oturum yeniden açılınca görünür."
                    )
                );
            }
        }
        After::SignOut => println!(
            "  {}",
            tr!(
                "sign out and back in for every change to take effect.",
                "tüm değişikliklerin geçerli olması için oturumu kapatıp açın."
            )
        ),
        After::Reboot => println!(
            "  {}",
            tr!(
                "restart Windows for every change to take effect.",
                "tüm değişikliklerin geçerli olması için Windows'u yeniden başlatın."
            )
        ),
    }
    println!(
        "  {}\n",
        tr!(
            "undo any time: kz tweak undo ID   (kz tweak undo all)",
            "geri almak için: kz tweak undo ID   (kz tweak undo all)"
        )
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        let mut ids = std::collections::HashSet::new();
        for t in TWEAKS {
            assert!(ids.insert(t.id), "duplicate id {}", t.id);
            assert!(
                t.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "id {}",
                t.id
            );
            assert!(
                CATEGORIES.iter().any(|c| c.0 == t.cat),
                "{}: unknown category",
                t.id
            );
            assert!(
                !CATEGORIES.iter().any(|c| c.0 == t.id),
                "{}: id equals a category",
                t.id
            );
            assert!(!t.regs.is_empty(), "{}: no registry values", t.id);
            for r in t.regs {
                assert!(
                    r.key.starts_with(r"HKCU\") || r.key.starts_with(r"HKLM\"),
                    "{}: {}",
                    t.id,
                    r.key
                );
                assert_ne!(r.set, r.default, "{}: {} does nothing", t.id, r.name);
                assert!(r.set != NoKey && r.set != Absent, "{}: sets nothing", t.id);
                // a service must exist already; kazma never creates service keys
                if is_service(r.key) {
                    assert!(matches!(r.set, D(_)), "{}", t.id);
                }
            }
            for task in t.tasks {
                assert!(task.starts_with('\\'), "{task}");
            }
        }
        // every category is used
        for c in CATEGORIES {
            assert!(TWEAKS.iter().any(|t| t.cat == c.0), "{} is empty", c.0);
        }
    }

    #[test]
    fn value_matching() {
        let dword = |v: &str| Cur {
            k: true,
            t: Some("DWord".into()),
            v: Some(v.into()),
        };
        let gone = Cur {
            k: true,
            t: None,
            v: None,
        };
        let no_key = Cur::default();
        assert!(matches(r"HKCU\x", &dword("0"), D(0)));
        assert!(!matches(r"HKCU\x", &dword("1"), D(0)));
        assert!(matches(r"HKCU\x", &gone, Absent));
        assert!(matches(r"HKCU\x", &no_key, Absent));
        assert!(matches(r"HKCU\x", &no_key, NoKey));
        assert!(!matches(r"HKCU\x", &gone, NoKey));
        let sz = Cur {
            k: true,
            t: Some("String".into()),
            v: Some(String::new()),
        };
        assert!(matches(r"HKCU\x", &sz, S("")));
        assert!(!matches(r"HKCU\x", &dword("0"), S("0")));
        // a service that is not installed counts as done
        assert!(matches(
            r"HKLM\SYSTEM\CurrentControlSet\Services\Nope",
            &no_key,
            D(4)
        ));
    }

    #[test]
    fn scripts() {
        assert_eq!(
            ps_path(r"HKCU\Software\It's"),
            r"'Registry::HKEY_CURRENT_USER\Software\It''s'"
        );
        let s = read_script();
        let calls = s.matches("$l.Add((KzGet ").count();
        assert_eq!(calls, TWEAKS.iter().map(|t| t.regs.len()).sum::<usize>());
        // services are never created, other keys are
        let svc = Reg {
            key: r"HKLM\SYSTEM\CurrentControlSet\Services\DiagTrack",
            name: "Start",
            set: D(4),
            default: D(2),
        };
        assert!(ops_script(&[op_for(&svc, svc.set)]).contains("'Start' DWord 4 $false"));
        let menu = TWEAKS.iter().find(|t| t.id == "classic-menu").unwrap();
        let on = ops_script(&[op_for(&menu.regs[0], menu.regs[0].set)]);
        assert!(on.contains("'' String '' $true"));
        let off = ops_script(&[op_for(&menu.regs[0], menu.regs[0].default)]);
        assert!(off.contains(
            "\nKzDelKey 'Registry::HKEY_CURRENT_USER\\Software\\Classes\\CLSID\\{86ca1aa0"
        ));
    }

    #[test]
    fn undo_uses_the_backup() {
        let r = Reg {
            key: r"HKCU\Control Panel\Desktop",
            name: "MenuShowDelay",
            set: S("100"),
            default: S("400"),
        };
        let was = Cur {
            k: true,
            t: Some("String".into()),
            v: Some("250".into()),
        };
        let s = ops_script(&[op_from_saved(&r, &was).unwrap()]);
        assert!(s.contains("'MenuShowDelay' String '250'"));
        let missing = Cur {
            k: true,
            t: None,
            v: None,
        };
        assert!(matches!(
            op_from_saved(&r, &missing),
            Some(Op::DelValue(..))
        ));
        let binary = Cur {
            k: true,
            t: Some("Binary".into()),
            v: Some("1 2".into()),
        };
        assert!(op_from_saved(&r, &binary).is_none());
    }

    #[test]
    fn profiles_name_real_settings() {
        for (key, _, _, ids) in PROFILES {
            assert!(!ids.is_empty(), "{key}");
            for id in *ids {
                assert!(TWEAKS.iter().any(|t| t.id == *id), "{key}: {id}");
            }
        }
        assert_eq!(
            parse_ids("# x\ntelemetry\nnope\n DARK-MODE # c\n"),
            vec!["telemetry", "dark-mode"]
        );
    }

    #[test]
    fn resolve_names() {
        let ids = resolve(&["privacy".into(), "telemetry".into()], |_| true).unwrap();
        assert_eq!(
            ids.len(),
            TWEAKS.iter().filter(|t| t.cat == "privacy").count()
        );
        assert!(resolve(&["nope".into()], |_| true).is_err());
        assert!(resolve(&["all".into()], |_| false).unwrap().is_empty());
    }
}
