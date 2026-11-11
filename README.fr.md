<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**Boîte à outils système oldschool, tout en ASCII, pour Windows**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [Deutsch](README.de.md) · **Français** · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-tr.png" width="49%" alt="menu principal de kazma en turc"> <img src="docs/screenshots/menu-en.png" width="49%" alt="menu principal de kazma en anglais, thème ambre">

</div>

---

**kazma** (« pioche » en turc) est un unique `kz.exe` d'environ 2,7 MB qui nettoie, explore et entretient
un PC Windows depuis le terminal, avec un look rétro d'écran vert. Pas d'installateur, pas de service en
arrière-plan, pas de télémétrie. L'interface est disponible en 11 langues (English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский) et démarre dans la langue d'affichage de Windows ; changez-la avec `kz config lang de`.

## Fonctionnalités

| | |
|---|---|
| **Nettoyage** | plus de 100 types de caches, fichiers temporaires, journaux et la Corbeille, y compris outils d'IA, lanceurs de jeux et restes de pilotes graphiques (`clean`) · Windows.old, fichiers de mise à niveau, vidages mémoire, fichier de veille prolongée, anciens points de restauration (`deepclean`) · explorateur de disque avec treemap et vue des plus gros fichiers (`analyze`) · types de fichiers (`types`) · artefacts de build des projets (`purge`) · anciens installateurs (`installer`) · fichiers en double (`dupes`) · dossiers vides et raccourcis cassés (`empty`) · traces d'utilisation (`privacy`) |
| **Programmes et pilotes** | désinstaller des programmes et leurs restes (`uninstall`) · mettre à jour les programmes avec winget (`upgrade`) · mises à jour de pilotes via Windows Update (`drivers`) · applications de démarrage (`startup`) · applis promo et publicités de Windows (`debloat`) · sauvegarde pour migrer vers un nouveau PC (`backup`) |
| **Système** | bilan de santé (`health`) · moniteur en direct avec GPU et batterie, gestionnaire de processus qui explique pourquoi un processus tourne (`status`) · maintenance : DNS, cache d'icônes, TRIM SSD, DISM, SFC (`optimize`) · réparations : réseau, Windows Update, Store, fichiers système (`repair`) · modes d'alimentation et batterie (`power`) · points de restauration (`restore`) · rapport HTML (`report`) · test de vitesse du disque (`bench`) · nettoyage automatique (`schedule`) |
| **Réseau** | IP, latence, comparatif et changement de DNS, test de débit, Wi-Fi enregistrés (`net`) · qui utilise quel port (`ports`) |
| **Pratique** | quel programme garde un fichier ouvert (`lock`) · garder l'écran allumé (`awake`) · mode nettoyage d'écran (`cleanscreen`) |
| **kazma** | chemins protégés (`whitelist`) · vos propres règles de nettoyage (`rules`) · menu contextuel de l'Explorateur (`context`) · langue et thème (`config`) · mise à jour automatique (`update`) · journal (`history`) · désinstaller kazma (`remove`) |

Chaque liste a une recherche `/` et fonctionne à la souris (clic pour sélectionner, second clic pour ouvrir, molette pour défiler).
Le menu vous prévient quand une nouvelle version sort.

C'est rapide. Lancés en administrateur sur un disque NTFS, `analyze` et `types` lisent directement la Master File Table
du disque (l'astuce derrière WizTree) : les tailles d'un disque entier sont prêtes en quelques secondes. Sinon, les tailles sont
calculées en parallèle, et `dupes` compare par taille, puis par les premiers et derniers
64 KB, et seulement ensuite hache les fichiers entiers avec BLAKE3 (83 000 fichiers / 5,9 GB en 7 secondes environ).

## Installation

Windows 10/11, x64 et ARM64. Aucun droit administrateur requis.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

Cela installe dans `%LOCALAPPDATA%\Programs\kazma` et ajoute `kz` à votre PATH. Alternatives :

- **Scoop** : `scoop bucket add kazma https://github.com/hhasanguclu/kazma` puis `scoop install kazma`
- **winget** : les manifestes de chaque version sont générés dans [`packaging/winget`](packaging/winget) ; une fois acceptés : `winget install hhasanguclu.kazma`
- **Manuel** : téléchargez `kz.exe` depuis la [dernière version](https://github.com/hhasanguclu/kazma/releases/latest)

> L'exe n'est pas encore signé, SmartScreen peut donc afficher un avertissement : **Informations complémentaires -> Exécuter quand même**.
> La signature via SignPath est en préparation, voir [docs/SIGNING.md](docs/SIGNING.md).

**Désinstallation** : `kz remove` (supprime l'exe, son entrée PATH et `%LOCALAPPDATA%\kazma`).

## Commandes

| Commande | Ce qu'elle fait |
|---|---|
| `kz` | Menu interactif |
| `kz clean` | Fichiers temporaires, vidages sur incident, caches de navigateurs / applis / outils de dev, Corbeille |
| `kz deepclean` | Windows.old, fichiers d'installation de mise à niveau, fichiers ESD, vidages mémoire, fichier de veille prolongée, anciens points de restauration (admin) |
| `kz analyze [path]` | Explorateur d'utilisation du disque : `t` treemap, `b` plus gros fichiers, `s` tri, `/` recherche, `w` qui l'utilise ; explique les fichiers spéciaux (hiberfil, disques WSL, sauvegardes d'iPhone...) ; la suppression passe par la Corbeille |
| `kz types [path]` | Quels types de fichiers prennent la place (vidéo, images, archives...) et les extensions principales |
| `kz purge [paths...]` | Artefacts de build : `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | Anciens fichiers d'installation (`.exe`, `.msi`, `.msix`, `.iso`) dans Téléchargements et sur le Bureau |
| `kz dupes [paths...]` | Fichiers en double (taille -> hachage partiel -> BLAKE3, tout en parallèle) |
| `kz empty [paths...]` | Dossiers vides, raccourcis cassés et fichiers de 0 octet (vers la Corbeille) |
| `kz privacy` | Fichiers récents, historique Exécuter / Explorateur, presse-papiers, historique et cookies des navigateurs (sur option) |
| `kz uninstall` | Supprime les applis classiques, Microsoft Store et Scoop, puis les dossiers et clés de registre restants (sauvegardés d'abord) ; `--leftovers` trouve les restes d'applis supprimées autrement |
| `kz upgrade` | Met à jour les programmes installés via winget |
| `kz drivers` | Mises à jour de pilotes via Windows Update ; `--installed` liste les pilotes tiers par âge |
| `kz startup` | Active/désactive les applications de démarrage (mêmes indicateurs que le Gestionnaire des tâches, entièrement réversible) |
| `kz debloat` | Supprime les applis promo préinstallées d'une liste fixe, coupe les publicités ; `--restore` annule |
| `kz backup [folder]` | Kit nouveau PC : liste d'applis winget, pilotes, liste des programmes, paramètres kazma ; `kz backup restore FOLDER` réinstalle |
| `kz health` | SMART des disques, usure de la batterie, écrans bleus, erreurs, mises à jour, Defender, pare-feu |
| `kz status` | CPU, GPU, mémoire, disque, réseau, batterie en direct ; `enter` explique pourquoi un processus tourne, `x` l'arrête, `m` trie par mémoire |
| `kz power` | Modes d'alimentation (`set saver / balanced / high / ultimate`), rapport `battery`, sources de réveil `wake`, `hibernate off / reduced / full` |
| `kz restore` | Points de Restauration du système : `list`, `create "desc"`, `enable`, `open` (l'assistant de Windows) |
| `kz report` | Rapport système HTML : matériel, santé, démarrage, programmes, pilotes |
| `kz bench [path]` | Test de vitesse du disque : séquentiel et aléatoire 4K, cache du système contourné |
| `kz net` | IP et latence ; comparatif `dns`, `dns set cloudflare` ; test `speed` ; `wifi [--show]` |
| `kz ports [filter]` | Quel programme écoute sur quel port ; `--kill PORT` le ferme |
| `kz optimize` | Entrées de démarrage cassées (sauvegardées), redémarrage de Démarrer/Recherche, vidage DNS, cache d'icônes, TRIM SSD / DISM / SFC quand c'est utile |
| `kz repair [fix]` | `network`, `update` (réinitialisation de Windows Update), `store`, `dism` (+SFC), `time`, `printer` ; point de restauration d'abord (admin) |
| `kz schedule weekly` | Lance `kz clean --yes` automatiquement (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | Quel programme a ouvert un fichier ou un dossier |
| `kz awake [minutes]` | Garde l'écran et le PC éveillés |
| `kz cleanscreen [seconds]` | Écran noir qui ignore les saisies, pour nettoyer l'écran |
| `kz whitelist` | Chemins que kazma ne doit jamais supprimer (`--add`, `--remove`, `--edit`) |
| `kz rules` | Vos propres règles de nettoyage pour `kz clean` (`--edit` ouvre le Bloc-notes) |
| `kz context install` | « Analyser avec kazma » dans le menu contextuel de l'Explorateur (`uninstall` le retire) |
| `kz config` | `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | Met à jour kazma depuis GitHub (somme de contrôle vérifiée) |
| `kz history` | Journal de tout ce que kazma a fait |
| `kz remove` | Désinstalle kazma |

Chaque commande qui supprime ou installe accepte `--dry-run` (`-n`) pour un aperçu préalable.
Lancez-la depuis un terminal **élevé** (Exécuter en tant qu'administrateur) pour installer des pilotes, effectuer
les tâches de maintenance admin et nettoyer les caches de Windows Update et les fichiers temporaires du système.

## Sécurité

- `clean` ne touche qu'à des emplacements de cache et de fichiers temporaires connus et régénérables ; les fichiers temporaires de moins de 24 h sont conservés.
- `purge` ne supprime un dossier que si un marqueur de projet se trouve à côté (`package.json`, `Cargo.toml`, `.csproj`...).
- Les racines de disques, votre profil, `Windows`, `Program Files`, `AppData` et les dossiers similaires sont bloqués sans exception.
- Les restes de `analyze`, `installer`, `dupes` et `uninstall` vont dans la **Corbeille**.
- `dupes` garde toujours au moins un fichier de chaque groupe.
- `drivers` n'installe que des pilotes proposés par **Windows Update** et crée d'abord un **point de restauration**.
- `startup` ne fait que basculer les indicateurs activé/désactivé du Gestionnaire des tâches ; `debloat` ne supprime que les applis d'une liste fixe et `--restore` annule ses réglages.
- `update` vérifie le téléchargement avec le `SHA256SUMS.txt` de la version.
- `deepclean` supprime Windows.old et les fichiers de mise à niveau via le Nettoyage de disque de Windows, et garde toujours le point de restauration le plus récent.
- `repair` renomme les dossiers de Windows Update au lieu de les supprimer et propose d'abord un point de restauration.
- Le lecteur de MFT ne fait que lire le disque ; il n'y écrit jamais.
- Les restes du registre et les entrées de démarrage cassées sont exportés dans `%LOCALAPPDATA%\kazma\backups` avant d'être supprimés.
- Les suggestions de `uninstall --leftovers` sont désélectionnées par défaut et vont dans la Corbeille.
- **Liste blanche** : les chemins listés dans `%LOCALAPPDATA%\kazma\whitelist.txt` ne sont jamais supprimés. Si ce fichier existe mais ne peut pas être lu, kazma refuse de s'exécuter.
- Chaque action est journalisée (`kz history`) ; sans terminal, rien n'est supprimé sauf si `--yes` est passé.

## Compilation

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

Le code se compile aussi sous Linux/macOS pour le développement ; les parties propres à Windows sont derrière `cfg(windows)`.
Pour publier une version, augmentez `version` dans `Cargo.toml` et poussez sur `main` : la CI teste sous Windows, exécute réellement
chaque commande en lecture seule, puis publie `vX.Y.Z` avec `kz.exe`, un zip et `SHA256SUMS.txt`, et met à jour les
manifestes Scoop / winget.

## Captures d'écran

| | |
|---|---|
| **Menu principal, turc**<br><img src="docs/screenshots/menu-tr.png" alt="menu principal, turc"> | **Menu principal, anglais, thème ambre**<br><img src="docs/screenshots/menu-en.png" alt="menu principal, anglais, thème ambre"> |
| **`kz analyze`**<br><img src="docs/screenshots/analyze-tr.png" alt="analyze"> | **`kz analyze` + `b`** : plus gros fichiers<br><img src="docs/screenshots/analyze-largest.png" alt="plus gros fichiers"> |
| **`kz analyze` + `t`** : treemap<br><img src="docs/screenshots/treemap-tr.png" alt="treemap"> | **`kz types`** : types de fichiers<br><img src="docs/screenshots/types-en.png" alt="types de fichiers"> |
| Sélection **`kz clean`**<br><img src="docs/screenshots/clean-tr.png" alt="clean"> | **`kz purge`**, thème bleu<br><img src="docs/screenshots/purge-en.png" alt="purge"> |
| **`kz health`** sous Windows<br><img src="docs/screenshots/health-windows.png" alt="bilan de santé sous Windows"> | **`kz startup --list`** sous Windows<br><img src="docs/screenshots/startup-windows.png" alt="liste de démarrage sous Windows"> |

<sub>`health` et `startup` ont été capturés lors d'une vraie exécution sous Windows (GitHub Actions, Windows Server 2025).
Les autres écrans proviennent du même code exécuté dans un terminal avec des données de démonstration, d'où leurs chemins
du type `/home/user/demo`.</sub>

## Remerciements

**Inspiration.** kazma s'inspire de [Mole](https://github.com/tw93/Mole) de
[tw93](https://github.com/tw93), une boîte à outils de nettoyage en profondeur pour macOS. kazma est écrit de zéro en Rust et
ne partage aucun code avec lui.

**Traductions.** Chaque langue autre que l'anglais et le turc se trouve dans [`locales/`](locales) sous forme de fichier JSON
qui associe le texte anglais à sa traduction. Les corrections sont les bienvenues : modifiez le fichier et ouvrez une pull request
(`python tools/i18n_extract.py --missing de` liste ce qui manque encore à une langue).

**Bibliothèques.** Merci aux auteurs et mainteneurs de :

| Crate | Utilisé pour | Licence |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | interface terminal | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | saisie et couleurs du terminal | MIT |
| [clap](https://github.com/clap-rs/clap) | ligne de commande | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, mémoire, disques, processus | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | analyse en parallèle | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | hachage des doublons | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | Corbeille | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | JSON PowerShell, traductions | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | erreurs | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | la macro de traduction `tr!` | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | API Windows | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | icône et infos de version de l'exe | MIT |

**Plateforme.** [winget](https://github.com/microsoft/winget-cli) de Microsoft (mises à jour des programmes), l'API Windows Update
Agent (mises à jour des pilotes), [Scoop](https://scoop.sh) (distribution) et la
[SignPath Foundation](https://signpath.org) (signature de code gratuite pour l'open source).

**Graphisme.** La bannière utilise le style de lettres classique « standard » de [FIGlet](http://www.figlet.org) ; les captures d'écran
sont rendues avec la police [DejaVu Sans Mono](https://dejavu-fonts.github.io).

**Auteur.** [hhasanguclu](https://github.com/hhasanguclu). Rapports de bugs, idées et pull requests sont les bienvenus.

## Licence

[MIT](LICENSE)
