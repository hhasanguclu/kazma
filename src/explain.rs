//! Plain-language notes for files and folders that surprise people in a disk
//! explorer: what they are, whether they can go, and the safe way to shrink them.

use crate::ui;
use std::path::Path;

/// A short explanation for `path` (named `name`), if it is a well-known item.
pub fn explain(path: &Path, name: &str, is_dir: bool) -> Option<&'static str> {
    let n = name.to_lowercase();
    let p = path.to_string_lossy().to_lowercase().replace('/', "\\");
    let parent = path
        .parent()
        .and_then(|x| x.file_name())
        .map(|x| x.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let has = |f: &str| path.parent().is_some_and(|d| d.join(f).exists());
    let t = ui::t;
    Some(match n.as_str() {
        "hiberfil.sys" => t(
            "Hibernation file. Shrink it: kz power hibernate reduced. Remove it: kz power hibernate off.",
            "Hazırda bekletme dosyası. Küçült: kz power hibernate reduced. Kaldır: kz power hibernate off.",
        ),
        "pagefile.sys" => t(
            "Virtual memory (page file). Windows manages its size; do not delete it.",
            "Sanal bellek (disk belleği). Boyutunu Windows yönetir; silmeyin.",
        ),
        "swapfile.sys" => t(
            "Swap file for Store apps. Managed by Windows.",
            "Store uygulamaları için takas dosyası. Windows yönetir.",
        ),
        "memory.dmp" | "minidump" | "livekernelreports" => t(
            "Crash dump from a blue screen. Safe to remove: kz deepclean.",
            "Mavi ekran çökme dökümü. Silinebilir: kz deepclean.",
        ),
        "windows.old" => t(
            "Previous Windows installation, kept for rollback. Remove it with kz deepclean (no rollback afterwards).",
            "Önceki Windows kurulumu (geri dönüş için). kz deepclean ile kaldırın (sonra geri dönülemez).",
        ),
        "$windows.~bt" | "$windows.~ws" | "$getcurrent" | "esd" if is_dir => t(
            "Windows upgrade files. Remove them with kz deepclean.",
            "Windows yükseltme dosyaları. kz deepclean ile kaldırın.",
        ),
        "$recycle.bin" => t(
            "The Recycle Bin. Empty it with kz clean.",
            "Geri Dönüşüm Kutusu. kz clean ile boşaltın.",
        ),
        "system volume information" => t(
            "Restore points and shadow copies. Older ones can go: kz deepclean.",
            "Geri yükleme noktaları ve gölge kopyalar. Eskileri silinebilir: kz deepclean.",
        ),
        "winsxs" => t(
            "Windows component store. Looks bigger than it is (shared hard links). Shrink: kz optimize (DISM cleanup).",
            "Windows bileşen deposu. Göründüğünden küçüktür (paylaşılan bağlantılar). Küçült: kz optimize (DISM).",
        ),
        "installer" if parent == "windows" => t(
            "Windows Installer cache. Needed to repair and uninstall programs; do not delete it.",
            "Windows Installer önbelleği. Programları onarmak ve kaldırmak için gerekir; silmeyin.",
        ),
        "softwaredistribution" => t(
            "Windows Update downloads. kz clean (as administrator) empties the download cache.",
            "Windows Update indirmeleri. kz clean (yönetici) indirme önbelleğini temizler.",
        ),
        "ext4.vhdx" if p.contains("\\docker") => t(
            "Docker Desktop's Linux disk. Free space with 'docker system prune', then compact the disk.",
            "Docker Desktop'ın Linux diski. 'docker system prune' ile yer açın, sonra diski sıkıştırın.",
        ),
        "ext4.vhdx" => t(
            "WSL Linux disk. It does not shrink by itself: run 'wsl --shutdown', then compact it (Optimize-VHD or diskpart).",
            "WSL Linux diski. Kendiliğinden küçülmez: 'wsl --shutdown' çalıştırıp diski sıkıştırın (Optimize-VHD ya da diskpart).",
        ),
        "docker_data.vhdx" => t(
            "Docker Desktop data disk. Free space with 'docker system prune -a'.",
            "Docker Desktop veri diski. 'docker system prune -a' ile yer açın.",
        ),
        "backup" if parent == "mobilesync" => t(
            "iPhone / iPad backups made by iTunes or Apple Devices. Delete old ones from that app.",
            "iTunes ya da Apple Devices'ın iPhone / iPad yedekleri. Eskileri o uygulamadan silin.",
        ),
        "mobilesync" => t(
            "iPhone / iPad backups live in Backup\\ inside.",
            "iPhone / iPad yedekleri içindeki Backup\\ klasöründe.",
        ),
        "node_modules" if has("package.json") => t(
            "JavaScript dependencies. kz purge removes them; npm install brings them back.",
            "JavaScript bağımlılıkları. kz purge kaldırır; npm install geri getirir.",
        ),
        "target" if has("Cargo.toml") => t(
            "Rust build output. kz purge removes it; cargo build recreates it.",
            "Rust derleme çıktısı. kz purge kaldırır; cargo build yeniden oluşturur.",
        ),
        ".ollama" | "models" if p.contains(".ollama") => t(
            "AI models downloaded by Ollama. Remove one with 'ollama rm <model>'.",
            "Ollama'nın indirdiği yapay zekâ modelleri. 'ollama rm <model>' ile kaldırın.",
        ),
        "steamapps" | "common" if p.contains("steam") => t(
            "Installed Steam games. Uninstall games from Steam to free space.",
            "Kurulu Steam oyunları. Yer açmak için oyunları Steam'den kaldırın.",
        ),
        "onedrive" if is_dir => t(
            "OneDrive folder. Online-only files take no space; 'Free up space' in OneDrive makes more of them.",
            "OneDrive klasörü. Yalnızca çevrimiçi dosyalar yer kaplamaz; OneDrive'da 'Alanı boşalt' ile artırın.",
        ),
        "appdata" => t(
            "App settings and caches. kz clean handles the caches safely.",
            "Uygulama ayarları ve önbellekleri. kz clean önbellekleri güvenle temizler.",
        ),
        "program files" | "program files (x86)" => t(
            "Installed programs. Remove them with kz uninstall.",
            "Kurulu programlar. kz uninstall ile kaldırın.",
        ),
        "downloads" => t(
            "Downloads. kz installer finds old setup files, kz dupes finds copies.",
            "İndirilenler. kz installer eski kurulum dosyalarını, kz dupes kopyaları bulur.",
        ),
        ".git" if is_dir => t(
            "Git history of a project. Do not delete it unless you mean to.",
            "Bir projenin Git geçmişi. Bilerek değilse silmeyin.",
        ),
        _ if n.ends_with(".vhdx") || n.ends_with(".vhd") => t(
            "Virtual disk image (Hyper-V, WSL or Docker). Compact it from its owner app.",
            "Sanal disk kalıbı (Hyper-V, WSL ya da Docker). Sahibi olan uygulamadan sıkıştırın.",
        ),
        _ if n.ends_with(".iso") => t(
            "Disk image, usually an installer. Usually safe to delete after use.",
            "Disk kalıbı, genelde bir kurulum. Kullandıktan sonra genelde silinebilir.",
        ),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn known_items() {
        assert!(explain(&PathBuf::from("C:\\hiberfil.sys"), "hiberfil.sys", false).is_some());
        assert!(
            explain(
                &PathBuf::from("/u/AppData/Roaming/Apple Computer/MobileSync/Backup"),
                "Backup",
                true
            )
            .is_some()
        );
        assert!(explain(&PathBuf::from("/x/Backup"), "Backup", true).is_none());
        assert!(
            explain(
                &PathBuf::from("/x/Docker/wsl/ext4.vhdx"),
                "ext4.vhdx",
                false
            )
            .unwrap()
                != explain(&PathBuf::from("/x/Ubuntu/ext4.vhdx"), "ext4.vhdx", false).unwrap()
        );
        assert!(explain(&PathBuf::from("/x/random.txt"), "random.txt", false).is_none());
    }
}
