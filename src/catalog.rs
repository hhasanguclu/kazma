//! Catalog of extra cleanup rules for `kz clean`: app, launcher, developer and
//! AI-tool caches. Everything listed here is regenerable: the owning program
//! rebuilds it (or downloads it again) when needed.
//!
//! Path syntax: a base, then `\`-separated segments.
//!   L = %LOCALAPPDATA%   R = %APPDATA%   P = %ProgramData%   H = user profile
//!   W = %SystemRoot%     F = Program Files   X = Program Files (x86)   C = system drive root
//!   `*` matches any folder name, `@` expands to the usual Electron/Chromium
//!   cache folders (Cache, Code Cache, GPUCache, DawnCache, DawnGraphiteCache).

use crate::fsutil::{env_path, home};
use std::path::{Path, PathBuf};

pub struct Rule {
    pub group: &'static str,
    pub en: &'static str,
    pub tr: &'static str,
    pub paths: &'static [&'static str],
    pub min_age_h: Option<u64>,
    pub admin: bool,
    /// Selected by default. Off for big, slow-to-rebuild things (models, package stores).
    pub on: bool,
}

const fn r(
    group: &'static str,
    en: &'static str,
    tr: &'static str,
    paths: &'static [&'static str],
    on: bool,
) -> Rule {
    Rule {
        group,
        en,
        tr,
        paths,
        min_age_h: None,
        admin: false,
        on,
    }
}

const fn ra(
    group: &'static str,
    en: &'static str,
    tr: &'static str,
    paths: &'static [&'static str],
    on: bool,
) -> Rule {
    Rule {
        group,
        en,
        tr,
        paths,
        min_age_h: None,
        admin: true,
        on,
    }
}

pub const ELECTRON: &[&str] = &[
    "Cache",
    "Code Cache",
    "GPUCache",
    "DawnCache",
    "DawnGraphiteCache",
];

pub const RULES: &[Rule] = &[
    // ---- AI tools ---------------------------------------------------------------------
    r(
        "AI",
        "Claude desktop cache",
        "Claude masaüstü önbelleği",
        &["R\\Claude\\@", "R\\Claude\\logs"],
        true,
    ),
    r(
        "AI",
        "ChatGPT desktop cache",
        "ChatGPT masaüstü önbelleği",
        &["L\\Packages\\OpenAI.ChatGPT-Desktop_*\\LocalCache\\Roaming\\ChatGPT\\@"],
        true,
    ),
    r(
        "AI",
        "Cursor editor cache",
        "Cursor düzenleyici önbelleği",
        &["R\\Cursor\\@", "R\\Cursor\\CachedData", "R\\Cursor\\logs"],
        true,
    ),
    r(
        "AI",
        "Windsurf editor cache",
        "Windsurf düzenleyici önbelleği",
        &[
            "R\\Windsurf\\@",
            "R\\Windsurf\\CachedData",
            "R\\Windsurf\\logs",
        ],
        true,
    ),
    r(
        "AI",
        "LM Studio app cache",
        "LM Studio uygulama önbelleği",
        &["R\\LM Studio\\@"],
        true,
    ),
    r(
        "AI",
        "Hugging Face model cache",
        "Hugging Face model önbelleği",
        &["H\\.cache\\huggingface\\hub"],
        false,
    ),
    r(
        "AI",
        "PyTorch hub cache",
        "PyTorch hub önbelleği",
        &["H\\.cache\\torch"],
        false,
    ),
    r(
        "AI",
        "Ollama models",
        "Ollama modelleri",
        &["H\\.ollama\\models"],
        false,
    ),
    r(
        "AI",
        "LM Studio models",
        "LM Studio modelleri",
        &["H\\.lmstudio\\models", "H\\.cache\\lm-studio\\models"],
        false,
    ),
    r(
        "AI",
        "Whisper models",
        "Whisper modelleri",
        &["H\\.cache\\whisper"],
        false,
    ),
    // ---- browsers (extra) ----------------------------------------------------------------
    r(
        "Browser",
        "Chromium cache",
        "Chromium önbelleği",
        &["L\\Chromium\\User Data\\*\\@"],
        true,
    ),
    r(
        "Browser",
        "Yandex Browser cache",
        "Yandex Browser önbelleği",
        &["L\\Yandex\\YandexBrowser\\User Data\\*\\@"],
        true,
    ),
    r(
        "Browser",
        "Opera GX cache",
        "Opera GX önbelleği",
        &["L\\Opera Software\\Opera GX Stable\\@"],
        true,
    ),
    r(
        "Browser",
        "Internet cache (legacy)",
        "İnternet önbelleği (eski)",
        &[
            "L\\Microsoft\\Windows\\INetCache\\IE",
            "L\\Microsoft\\Windows\\INetCache\\Low",
        ],
        true,
    ),
    r(
        "Browser",
        "WebView2 caches of apps",
        "Uygulamaların WebView2 önbellekleri",
        &["L\\*\\EBWebView\\*\\@"],
        true,
    ),
    // ---- chat & productivity apps ------------------------------------------------------------
    r(
        "Apps",
        "Microsoft Teams (classic) cache",
        "Microsoft Teams (klasik) önbelleği",
        &[
            "R\\Microsoft\\Teams\\@",
            "R\\Microsoft\\Teams\\Service Worker\\CacheStorage",
            "R\\Microsoft\\Teams\\logs",
        ],
        true,
    ),
    r(
        "Apps",
        "Zoom logs and cache",
        "Zoom logları ve önbelleği",
        &["R\\Zoom\\logs", "R\\Zoom\\data\\WebviewCacheX"],
        true,
    ),
    r(
        "Apps",
        "WhatsApp cache",
        "WhatsApp önbelleği",
        &["L\\Packages\\5319275A.WhatsAppDesktop_*\\LocalState\\shared\\transfers"],
        false,
    ),
    r(
        "Apps",
        "Telegram media cache",
        "Telegram medya önbelleği",
        &[
            "R\\Telegram Desktop\\tdata\\user_data\\cache",
            "R\\Telegram Desktop\\tdata\\user_data\\media_cache",
        ],
        true,
    ),
    r(
        "Apps",
        "Notion cache",
        "Notion önbelleği",
        &["R\\Notion\\@", "R\\Notion\\logs"],
        true,
    ),
    r(
        "Apps",
        "Obsidian cache",
        "Obsidian önbelleği",
        &["R\\obsidian\\@"],
        true,
    ),
    r(
        "Apps",
        "Figma cache",
        "Figma önbelleği",
        &["R\\Figma\\DesktopProfile\\*\\@", "L\\Figma\\logs"],
        true,
    ),
    r(
        "Apps",
        "Postman cache",
        "Postman önbelleği",
        &["R\\Postman\\@", "R\\Postman\\logs"],
        true,
    ),
    r(
        "Apps",
        "GitHub Desktop cache",
        "GitHub Desktop önbelleği",
        &["R\\GitHub Desktop\\@", "R\\GitHub Desktop\\logs"],
        true,
    ),
    r(
        "Apps",
        "Signal cache",
        "Signal önbelleği",
        &["R\\Signal\\@", "R\\Signal\\logs"],
        true,
    ),
    r(
        "Apps",
        "Microsoft Edge WebView logs",
        "Microsoft Edge WebView logları",
        &["L\\Microsoft\\EdgeWebView\\Application\\*\\Logs"],
        true,
    ),
    r(
        "Apps",
        "OneDrive logs",
        "OneDrive logları",
        &["L\\Microsoft\\OneDrive\\logs"],
        true,
    ),
    r(
        "Apps",
        "Office telemetry and logs",
        "Office telemetri ve logları",
        &[
            "L\\Microsoft\\Office\\OTele",
            "L\\Microsoft\\Office\\16.0\\Floodgate",
        ],
        true,
    ),
    r(
        "Apps",
        "Adobe media cache",
        "Adobe medya önbelleği",
        &[
            "R\\Adobe\\Common\\Media Cache Files",
            "R\\Adobe\\Common\\Media Cache",
            "R\\Adobe\\Common\\Peak Files",
        ],
        true,
    ),
    r(
        "Apps",
        "Spotify cache (Store version)",
        "Spotify önbelleği (Store sürümü)",
        &["L\\Packages\\SpotifyAB.SpotifyMusic_*\\LocalCache\\Spotify\\Data"],
        true,
    ),
    r(
        "Apps",
        "VLC art cache",
        "VLC kapak önbelleği",
        &["R\\vlc\\art"],
        true,
    ),
    r(
        "Apps",
        "Windows Store app caches",
        "Windows Store uygulama önbellekleri",
        &["L\\Packages\\*\\AC\\INetCache", "L\\Packages\\*\\AC\\Temp"],
        true,
    ),
    // ---- games & launchers --------------------------------------------------------------------
    r(
        "Games",
        "Epic Games Launcher web cache",
        "Epic Games Launcher web önbelleği",
        &[
            "L\\EpicGamesLauncher\\Saved\\webcache",
            "L\\EpicGamesLauncher\\Saved\\webcache_4147",
            "L\\EpicGamesLauncher\\Saved\\webcache_4430",
            "L\\EpicGamesLauncher\\Saved\\Logs",
        ],
        true,
    ),
    r(
        "Games",
        "Steam web and log cache",
        "Steam web ve log önbelleği",
        &[
            "X\\Steam\\appcache\\httpcache",
            "X\\Steam\\logs",
            "X\\Steam\\dumps",
        ],
        true,
    ),
    r(
        "Games",
        "Steam shader cache",
        "Steam gölgelendirici önbelleği",
        &["X\\Steam\\steamapps\\shadercache"],
        false,
    ),
    r(
        "Games",
        "EA app cache",
        "EA uygulaması önbelleği",
        &[
            "L\\Electronic Arts\\EA Desktop\\cache",
            "L\\Electronic Arts\\EA Desktop\\Logs",
        ],
        true,
    ),
    r(
        "Games",
        "Battle.net cache",
        "Battle.net önbelleği",
        &[
            "L\\Battle.net\\Cache",
            "L\\Battle.net\\BrowserCaches",
            "L\\Blizzard Entertainment\\Battle.net\\Cache",
        ],
        true,
    ),
    r(
        "Games",
        "Ubisoft Connect cache",
        "Ubisoft Connect önbelleği",
        &[
            "L\\Ubisoft Game Launcher\\cache",
            "L\\Ubisoft Game Launcher\\logs",
        ],
        true,
    ),
    r(
        "Games",
        "Riot Client logs",
        "Riot Client logları",
        &["L\\Riot Games\\Riot Client\\Logs"],
        true,
    ),
    r(
        "Games",
        "GOG Galaxy web cache",
        "GOG Galaxy web önbelleği",
        &["P\\GOG.com\\Galaxy\\webcache"],
        true,
    ),
    r(
        "Games",
        "Xbox app cache",
        "Xbox uygulaması önbelleği",
        &["L\\Packages\\Microsoft.GamingApp_*\\LocalCache"],
        true,
    ),
    r(
        "Games",
        "Unity cache",
        "Unity önbelleği",
        &["L\\Unity\\cache", "R\\Unity\\Asset Store-5.x"],
        false,
    ),
    // ---- GPU drivers --------------------------------------------------------------------------
    ra(
        "GPU",
        "NVIDIA driver downloads",
        "NVIDIA sürücü indirmeleri",
        &["P\\NVIDIA Corporation\\Downloader"],
        true,
    ),
    ra(
        "GPU",
        "NVIDIA installer logs",
        "NVIDIA kurulum logları",
        &[
            "P\\NVIDIA Corporation\\NVIDIA app\\logs",
            "P\\NVIDIA\\*.log",
        ],
        true,
    ),
    r(
        "GPU",
        "NVIDIA shader cache",
        "NVIDIA gölgelendirici önbelleği",
        &["L\\NVIDIA Corporation\\NV_Cache"],
        true,
    ),
    ra(
        "GPU",
        "AMD driver installer files",
        "AMD sürücü kurulum dosyaları",
        &["C\\AMD\\*"],
        true,
    ),
    r(
        "GPU",
        "AMD shader cache",
        "AMD gölgelendirici önbelleği",
        &["L\\AMD\\VkCache", "L\\AMD\\GLCache"],
        true,
    ),
    r(
        "GPU",
        "Intel shader cache",
        "Intel gölgelendirici önbelleği",
        &["L\\Intel\\ShaderCache"],
        true,
    ),
    // ---- developer ------------------------------------------------------------------------------
    r(
        "Dev",
        "VS Code Insiders cache",
        "VS Code Insiders önbelleği",
        &[
            "R\\Code - Insiders\\@",
            "R\\Code - Insiders\\CachedData",
            "R\\Code - Insiders\\logs",
        ],
        true,
    ),
    r(
        "Dev",
        "VS Code extension downloads",
        "VS Code eklenti indirmeleri",
        &["R\\Code\\CachedExtensionVSIXs"],
        true,
    ),
    r(
        "Dev",
        "JetBrains IDE caches and logs",
        "JetBrains IDE önbellek ve logları",
        &[
            "L\\JetBrains\\*\\caches",
            "L\\JetBrains\\*\\log",
            "L\\JetBrains\\*\\tmp",
        ],
        true,
    ),
    r(
        "Dev",
        "Visual Studio component cache",
        "Visual Studio bileşen önbelleği",
        &["L\\Microsoft\\VisualStudio\\*\\ComponentModelCache"],
        true,
    ),
    r(
        "Dev",
        "npx cache",
        "npx önbelleği",
        &["L\\npm-cache\\_npx"],
        true,
    ),
    r(
        "Dev",
        "node-gyp headers",
        "node-gyp başlık dosyaları",
        &["L\\node-gyp\\Cache"],
        true,
    ),
    r(
        "Dev",
        "Electron downloads",
        "Electron indirmeleri",
        &["L\\electron\\Cache", "L\\electron-builder\\Cache"],
        true,
    ),
    r(
        "Dev",
        "Bun cache",
        "Bun önbelleği",
        &["H\\.bun\\install\\cache"],
        true,
    ),
    r("Dev", "Deno cache", "Deno önbelleği", &["L\\deno"], true),
    r(
        "Dev",
        "Yarn Berry cache",
        "Yarn Berry önbelleği",
        &["H\\.yarn\\berry\\cache"],
        true,
    ),
    r(
        "Dev",
        "Poetry cache",
        "Poetry önbelleği",
        &["L\\pypoetry\\Cache"],
        true,
    ),
    r("Dev", "uv cache", "uv önbelleği", &["L\\uv\\cache"], true),
    r(
        "Dev",
        "pre-commit cache",
        "pre-commit önbelleği",
        &["H\\.cache\\pre-commit"],
        true,
    ),
    r(
        "Dev",
        "Composer cache",
        "Composer önbelleği",
        &[
            "L\\Composer\\cache",
            "L\\Composer\\files",
            "L\\Composer\\repo",
        ],
        true,
    ),
    r(
        "Dev",
        "Cargo git checkouts",
        "Cargo git kopyaları",
        &["H\\.cargo\\git\\checkouts"],
        true,
    ),
    r(
        "Dev",
        "rustup downloads",
        "rustup indirmeleri",
        &["H\\.rustup\\downloads", "H\\.rustup\\tmp"],
        true,
    ),
    r(
        "Dev",
        "Android build cache",
        "Android derleme önbelleği",
        &["H\\.android\\cache", "H\\.android\\build-cache"],
        true,
    ),
    r(
        "Dev",
        "Cypress binaries",
        "Cypress dosyaları",
        &["L\\Cypress\\Cache"],
        false,
    ),
    r(
        "Dev",
        "Playwright browsers",
        "Playwright tarayıcıları",
        &["L\\ms-playwright"],
        false,
    ),
    r(
        "Dev",
        "Maven repository",
        "Maven deposu",
        &["H\\.m2\\repository"],
        false,
    ),
    r(
        "Dev",
        "Go module cache",
        "Go modül önbelleği",
        &["H\\go\\pkg\\mod\\cache"],
        false,
    ),
    r(
        "Dev",
        "conda package cache",
        "conda paket önbelleği",
        &[
            "H\\anaconda3\\pkgs",
            "H\\miniconda3\\pkgs",
            "H\\.conda\\pkgs",
        ],
        false,
    ),
    r(
        "Dev",
        "Flutter / Dart pub cache",
        "Flutter / Dart pub önbelleği",
        &["L\\Pub\\Cache"],
        false,
    ),
    r(
        "Dev",
        "Docker Desktop logs",
        "Docker Desktop logları",
        &["L\\Docker\\log"],
        true,
    ),
    r(
        "Dev",
        "WSL crash dumps",
        "WSL çökme dökümleri",
        &["L\\Temp\\wsl-crashes"],
        true,
    ),
    // ---- Windows ------------------------------------------------------------------------------------
    r(
        "User",
        "Windows web cache (Search, widgets)",
        "Windows web önbelleği (Arama, widget'lar)",
        &[
            "L\\Packages\\MicrosoftWindows.Client.WebExperience_*\\AC\\INetCache",
            "L\\Packages\\Microsoft.Windows.Search_*\\AC\\INetCache",
        ],
        true,
    ),
    r(
        "User",
        "Windows Store downloads cache",
        "Windows Store indirme önbelleği",
        &["L\\Packages\\Microsoft.WindowsStore_*\\LocalCache"],
        true,
    ),
    ra(
        "System",
        "Windows Update logs",
        "Windows Update logları",
        &[
            "W\\Logs\\WindowsUpdate",
            "W\\SoftwareDistribution\\DataStore\\Logs",
        ],
        true,
    ),
    ra(
        "System",
        "Windows Defender scan history (cleared items)",
        "Windows Defender tarama geçmişi (temizlenenler)",
        &[
            "P\\Microsoft\\Windows Defender\\Scans\\History\\Results\\Quick",
            "P\\Microsoft\\Windows Defender\\Scans\\History\\Results\\Resource",
        ],
        false,
    ),
    ra(
        "System",
        "Windows setup and panther logs",
        "Windows kurulum ve panther logları",
        &["W\\Panther", "W\\Logs\\MoSetup"],
        false,
    ),
    ra(
        "System",
        "Old Windows performance traces",
        "Eski Windows performans izleri",
        &[
            "W\\Performance\\WinSAT\\DataStore",
            "P\\Microsoft\\Diagnosis\\ETLLogs",
        ],
        true,
    ),
    ra(
        "System",
        "Font cache",
        "Yazı tipi önbelleği",
        &["W\\ServiceProfiles\\LocalService\\AppData\\Local\\FontCache"],
        false,
    ),
];

fn base(token: &str) -> Option<PathBuf> {
    match token {
        "L" => env_path("LOCALAPPDATA"),
        "R" => env_path("APPDATA"),
        "P" => env_path("ProgramData"),
        "H" => home(),
        "W" => env_path("SystemRoot").or_else(|| env_path("windir")),
        "F" => env_path("ProgramFiles"),
        "X" => env_path("ProgramFiles(x86)"),
        "C" => std::env::var("SystemDrive")
            .ok()
            .map(|d| PathBuf::from(format!("{d}\\"))),
        _ => None,
    }
}

fn subdirs(p: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(p)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| e.path())
                .collect()
        })
        .unwrap_or_default()
}

/// Expand one rule path into the existing folders (or files, for `*.log`) it names.
pub fn expand(spec: &str) -> Vec<PathBuf> {
    let mut parts = spec.split('\\');
    let Some(start) = parts.next().and_then(base) else {
        return vec![];
    };
    let mut cur = vec![start];
    for seg in parts {
        let mut next = vec![];
        for p in &cur {
            match seg {
                "*" => next.extend(subdirs(p)),
                "@" => next.extend(ELECTRON.iter().map(|c| p.join(c))),
                s if s.starts_with("*.") => {
                    let ext = &s[1..];
                    if let Ok(rd) = std::fs::read_dir(p) {
                        next.extend(rd.flatten().map(|e| e.path()).filter(|f| {
                            f.is_file() && f.to_string_lossy().to_lowercase().ends_with(ext)
                        }));
                    }
                }
                s if s.contains('*') => {
                    let (pre, post) = s.split_once('*').unwrap_or((s, ""));
                    next.extend(subdirs(p).into_iter().filter(|d| {
                        let n = d.file_name().unwrap_or_default().to_string_lossy();
                        n.starts_with(pre) && n.ends_with(post)
                    }));
                }
                s => next.push(p.join(s)),
            }
        }
        cur = next;
        if cur.is_empty() {
            break;
        }
    }
    cur.into_iter().filter(|p| p.exists()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_sane() {
        assert!(RULES.len() >= 70, "only {} rules", RULES.len());
        let mut names: Vec<&str> = RULES.iter().map(|r| r.en).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "duplicate rule name");
        for rule in RULES {
            for p in rule.paths {
                let b = p.split('\\').next().unwrap();
                assert!(["L", "R", "P", "H", "W", "F", "X", "C"].contains(&b), "{p}");
                // never a whole profile, app-data or system folder
                assert!(p.split('\\').count() >= 2, "{p}");
            }
        }
    }

    #[test]
    fn expands_wildcards() {
        let base = std::env::temp_dir().join(format!("kazma-cat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in [
            "JetBrains/IntelliJIdea2025.2/caches",
            "JetBrains/PyCharm2025.1/caches",
            "Packages/OpenAI.ChatGPT-Desktop_abc/x",
            "App/Cache",
            "App/GPUCache",
        ] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        std::fs::write(base.join("App/a.log"), b"x").unwrap();
        // SAFETY: test-only override of the env var `L` resolves to
        unsafe { std::env::set_var("LOCALAPPDATA", &base) };
        assert_eq!(expand("L\\JetBrains\\*\\caches").len(), 2);
        assert_eq!(expand("L\\Packages\\OpenAI.ChatGPT-Desktop_*\\x").len(), 1);
        assert_eq!(expand("L\\App\\@").len(), 2);
        assert_eq!(expand("L\\App\\*.log").len(), 1);
        assert!(expand("L\\Nope\\*").is_empty());
        std::fs::remove_dir_all(&base).unwrap();
    }
}
