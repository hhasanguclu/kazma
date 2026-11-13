<div align="center">

```
 _  __    _     _____  __  __     _
| |/ /   / \   |__  / |  \/  |   / \
| ' /   / _ \    / /  | |\/| |  / _ \
| . \  / ___ \  / /_  | |  | | / ___ \
|_|\_\/_/   \_\/____| |_|  |_|/_/   \_\
```

**올드스쿨 감성, 순수 ASCII로 만든 Windows 시스템 도구 모음**

[![release](https://img.shields.io/github/v/release/hhasanguclu/kazma?style=flat-square)](https://github.com/hhasanguclu/kazma/releases/latest)
[![ci](https://img.shields.io/github/actions/workflow/status/hhasanguclu/kazma/ci.yml?branch=main&style=flat-square&label=windows%20ci)](https://github.com/hhasanguclu/kazma/actions)
[![license](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
![platform](https://img.shields.io/badge/windows-10%20%7C%2011-blue?style=flat-square)

[English](README.md) · [Türkçe](README.tr.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · **한국어** · [Deutsch](README.de.md) · [Français](README.fr.md) · [Polski](README.pl.md) · [Українська](README.uk.md) · [Русский](README.ru.md)

<img src="docs/screenshots/menu-en.png" width="49%" alt="kazma main menu in English"> <img src="docs/screenshots/menu-ja.png" width="49%" alt="kazma main menu in Japanese, blue theme">

</div>

---

**kazma**(튀르키예어로 *곡괭이*)는 약 2.7 MB짜리 `kz.exe` 하나로 터미널에서 Windows PC를 정리하고, 살펴보고,
관리하는 도구입니다. 레트로한 초록색 화면 느낌을 살렸습니다. 설치 프로그램도, 백그라운드 서비스도, 원격 분석도 없습니다.
인터페이스는 11개 언어(English, Türkçe, 简体中文, 繁體中文, 日本語, 한국어, Deutsch, Français, Polski,
Українська, Русский)를 지원하며 Windows 표시 언어로 시작합니다. `kz config lang de`로 바꿀 수 있습니다.

## 기능

| | |
|---|---|
| **정리** | AI 도구, 게임 런처, GPU 드라이버 잔여 파일을 포함한 100가지 이상의 캐시, 임시 파일, 로그와 휴지통 (`clean`) · Windows.old, 업그레이드 파일, 메모리 덤프, 최대 절전 모드 파일, 오래된 복원 지점 (`deepclean`) · 트리맵과 큰 파일 보기를 갖춘 디스크 탐색기 (`analyze`) · 파일 유형 (`types`) · 프로젝트 빌드 결과물 (`purge`) · 오래된 설치 파일 (`installer`) · 중복 파일 (`dupes`) · 빈 폴더와 깨진 바로 가기 (`empty`) · 사용 흔적 (`privacy`) |
| **프로그램 및 드라이버** | 프로그램과 남은 파일 제거 (`uninstall`) · winget으로 프로그램 업데이트 (`upgrade`) · Windows 업데이트의 드라이버 업데이트 (`drivers`) · 시작 프로그램 (`startup`) · 홍보 앱과 Windows 광고 (`debloat`) · 새 PC 이사용 백업 (`backup`) |
| **시스템** | 상태 보고서 (`health`) · GPU와 배터리까지 보여주는 실시간 모니터, 프로세스가 왜 실행 중인지 알려주는 프로세스 관리자 (`status`) · 유지 관리: DNS, 아이콘 캐시, SSD TRIM, DISM, SFC (`optimize`) · 복구: 네트워크, Windows 업데이트, Store, 시스템 파일 (`repair`) · 전원 관리 옵션과 배터리 (`power`) · 복원 지점 (`restore`) · HTML 보고서 (`report`) · 디스크 속도 테스트 (`bench`) · 자동 정리 (`schedule`) |
| **네트워크** | IP, 지연 시간, DNS 벤치마크 및 변경, 속도 테스트, 저장된 Wi-Fi (`net`) · 어떤 포트를 누가 쓰는지 (`ports`) |
| **편의 기능** | 파일을 열고 있는 프로그램 찾기 (`lock`) · 화면 켜두기 (`awake`) · 화면 닦기 모드 (`cleanscreen`) |
| **kazma** | 보호 경로 (`whitelist`) · 나만의 정리 규칙 (`rules`) · 파일 탐색기 오른쪽 클릭 메뉴 (`context`) · 언어와 테마 (`config`) · 자체 업데이트 (`update`) · 기록 (`history`) · kazma 제거 (`remove`) |

모든 목록에서 `/`로 검색할 수 있고 마우스로도 조작할 수 있습니다(클릭으로 선택, 한 번 더 클릭하면 열기, 휠로 스크롤).
새 버전이 나오면 메뉴에서 알려줍니다.

빠릅니다. NTFS 드라이브에서 관리자 권한으로 실행하면 `analyze`와 `types`는 드라이브의 마스터 파일 테이블(MFT)을
직접 읽기 때문에(WizTree가 쓰는 방법) 드라이브 전체 크기가 몇 초 만에 나옵니다. 그 밖의 경우에는 크기를
병렬로 계산하고, `dupes`는 크기를 먼저 비교한 뒤 처음과 마지막 64 KB를 비교하고, 그다음에야 전체 파일을
BLAKE3로 해시합니다(83,000개 파일 / 5.9 GB를 약 7초에 처리).

## 설치

Windows 10/11, x64 및 ARM64. 관리자 권한이 필요 없습니다.

```powershell
iwr -useb https://raw.githubusercontent.com/hhasanguclu/kazma/main/install.ps1 | iex
```

`%LOCALAPPDATA%\Programs\kazma`에 설치되고 `kz`가 PATH에 추가됩니다. 다른 방법:

- **Scoop**: `scoop bucket add kazma https://github.com/hhasanguclu/kazma` 다음 `scoop install kazma`
- **winget**: 릴리스마다 매니페스트가 [`packaging/winget`](packaging/winget)에 생성됩니다. 승인되면: `winget install hhasanguclu.kazma`
- **수동**: [최신 릴리스](https://github.com/hhasanguclu/kazma/releases/latest)에서 `kz.exe`를 다운로드하세요

> 아직 exe에 코드 서명이 되어 있지 않아 SmartScreen 경고가 뜰 수 있습니다: **추가 정보 -> 실행**.
> SignPath를 통한 서명은 준비되어 있습니다. [docs/SIGNING.md](docs/SIGNING.md)를 참고하세요.

**제거**: `kz remove` (exe, PATH 항목, `%LOCALAPPDATA%\kazma`를 제거합니다).

## 명령

| 명령 | 하는 일 |
|---|---|
| `kz` | 대화형 메뉴 |
| `kz clean` | 임시 파일, 크래시 덤프, 브라우저 / 앱 / 개발 도구 캐시, 휴지통 |
| `kz deepclean` | Windows.old, 업그레이드 설치 파일, ESD 파일, 메모리 덤프, 최대 절전 모드 파일, 오래된 복원 지점 (관리자) |
| `kz analyze [path]` | 디스크 사용량 탐색기: `t` 트리맵, `b` 큰 파일, `s` 정렬, `/` 검색, `w` 사용 중인 프로그램; 특수 파일(hiberfil, WSL 디스크, iPhone 백업...)을 설명해 주며, 삭제한 항목은 휴지통으로 갑니다 |
| `kz types [path]` | 어떤 종류의 파일(동영상, 이미지, 압축 파일...)이 공간을 차지하는지와 상위 확장자 |
| `kz purge [paths...]` | 빌드 결과물: `node_modules`, `target`, `bin/obj`, `.venv`, `__pycache__`, ... |
| `kz installer [paths...]` | 다운로드 폴더와 바탕 화면의 오래된 설치 파일(`.exe`, `.msi`, `.msix`, `.iso`) |
| `kz dupes [paths...]` | 중복 파일 (크기 -> 부분 해시 -> BLAKE3, 모두 병렬) |
| `kz empty [paths...]` | 빈 폴더, 깨진 바로 가기, 0바이트 파일 (휴지통으로) |
| `kz privacy` | 최근 파일, 실행 / 파일 탐색기 기록, 클립보드, 브라우저 방문 기록과 쿠키 (선택 시에만) |
| `kz uninstall` | 일반 앱, Microsoft Store 앱, Scoop 앱을 제거한 뒤 남은 폴더와 레지스트리 키까지 정리(먼저 백업); `--leftovers`는 다른 방법으로 제거한 앱의 잔여물을 찾습니다 |
| `kz upgrade` | winget으로 설치된 프로그램 업데이트 |
| `kz drivers` | Windows 업데이트의 드라이버 업데이트; `--installed`는 타사 드라이버를 오래된 순으로 보여줍니다 |
| `kz startup` | 시작 프로그램 켜기/끄기 (작업 관리자와 같은 플래그, 언제든 되돌릴 수 있음) |
| `kz debloat` | 정해진 목록에 있는 기본 설치 홍보 앱 제거, 광고 끄기; `--restore`로 되돌리기 |
| `kz backup [folder]` | 새 PC 키트: winget 앱 목록, 드라이버, 프로그램 목록, kazma 설정; `kz backup restore FOLDER`로 다시 설치 |
| `kz health` | 디스크 SMART, 배터리 마모도, 블루스크린, 오류, 업데이트, Defender, 방화벽 |
| `kz status` | 실시간 CPU, GPU, 메모리, 디스크, 네트워크, 배터리; `enter`는 프로세스가 실행 중인 이유를 설명하고, `x`는 종료, `m`은 메모리순 정렬 |
| `kz power` | 전원 관리 옵션(`set saver / balanced / high / ultimate`), `battery` 보고서, `wake` 절전 해제 원인, `hibernate off / reduced / full` |
| `kz restore` | 시스템 복원 지점: `list`, `create "desc"`, `enable`, `open` (Windows 기본 마법사) |
| `kz report` | HTML 시스템 보고서: 하드웨어, 상태, 시작 프로그램, 프로그램, 드라이버 |
| `kz bench [path]` | 디스크 속도 테스트: 순차 및 4K 랜덤, OS 캐시 우회 |
| `kz net` | IP와 지연 시간; `dns` 벤치마크, `dns set cloudflare`; `speed` 테스트; `wifi [--show]` |
| `kz ports [filter]` | 어떤 프로그램이 어떤 포트에서 대기 중인지; `--kill PORT`로 종료 |
| `kz optimize` | 깨진 시작 항목(백업 후), 시작/검색 다시 시작, DNS 플러시, 아이콘 캐시, 필요할 때 SSD TRIM / DISM / SFC |
| `kz repair [fix]` | `network`, `update` (Windows 업데이트 초기화), `store`, `dism` (+SFC), `time`, `printer`; 먼저 복원 지점 생성 (관리자) |
| `kz schedule weekly` | `kz clean --yes`를 자동 실행 (`daily`, `weekly`, `monthly`, `off`, `status`) |
| `kz lock <path>` | 파일이나 폴더를 열고 있는 프로그램 |
| `kz awake [minutes]` | 화면과 PC를 켜진 상태로 유지 |
| `kz cleanscreen [seconds]` | 입력을 무시하는 검은 화면으로 디스플레이 닦기 |
| `kz whitelist` | kazma가 절대 삭제하지 않을 경로 (`--add`, `--remove`, `--edit`) |
| `kz rules` | `kz clean`에 쓸 나만의 정리 규칙 (`--edit`는 메모장을 엽니다) |
| `kz context install` | 파일 탐색기 오른쪽 클릭 메뉴에 "Analyze with kazma" 추가 (`uninstall`로 제거) |
| `kz config` | `kz config` ↑↓ enter · `kz config lang de` (auto, en, tr, zh-CN, zh-TW, ja, ko, de, fr, pl, uk, ru), `kz config theme amber` (green, amber, white, blue) |
| `kz update` | GitHub에서 kazma 자체 업데이트 (체크섬 확인) |
| `kz history` | kazma가 한 모든 작업의 기록 |
| `kz remove` | kazma 제거 |

삭제하거나 설치하는 모든 명령은 `--dry-run`(`-n`)으로 먼저 미리 볼 수 있습니다.
드라이버 설치, 관리자 유지 관리 작업, Windows 업데이트 및 시스템 임시 캐시 정리는 **관리자 권한** 터미널(관리자 권한으로 실행)에서
실행하세요.

## 안전

- `clean`은 알려져 있고 다시 생성되는 캐시와 임시 폴더만 건드립니다. 24시간이 안 된 임시 파일은 남겨 둡니다.
- `purge`는 바로 옆에 프로젝트 표식(`package.json`, `Cargo.toml`, `.csproj`...)이 있을 때만 폴더를 삭제합니다.
- 드라이브 루트, 사용자 프로필, `Windows`, `Program Files`, `AppData` 같은 폴더는 무조건 차단됩니다.
- `analyze`, `installer`, `dupes`, `uninstall`의 잔여 파일은 **휴지통**으로 갑니다.
- `dupes`는 그룹마다 항상 파일을 하나 이상 남깁니다.
- `drivers`는 **Windows 업데이트**가 제공하는 드라이버만 설치하며, 먼저 **복원 지점**을 만듭니다.
- `startup`은 작업 관리자의 켜기/끄기 플래그만 바꿉니다. `debloat`는 정해진 목록의 앱만 제거하며, `--restore`로 설정을 되돌릴 수 있습니다.
- `update`는 다운로드한 파일을 릴리스의 `SHA256SUMS.txt`로 검증합니다.
- `deepclean`은 Windows.old와 업그레이드 파일을 Windows의 디스크 정리로 제거하며, 가장 최근 복원 지점은 항상 남깁니다.
- `repair`는 Windows 업데이트 폴더를 삭제하지 않고 이름을 바꾸며, 먼저 복원 지점 생성을 제안합니다.
- MFT 리더는 드라이브를 읽기만 하고 절대 쓰지 않습니다.
- 레지스트리 잔여물과 깨진 시작 항목은 제거 전에 `%LOCALAPPDATA%\kazma\backups`로 내보냅니다.
- `uninstall --leftovers`가 제안한 항목은 선택 해제 상태로 시작하며 휴지통으로 갑니다.
- **화이트리스트**: `%LOCALAPPDATA%\kazma\whitelist.txt`에 있는 경로는 절대 삭제되지 않습니다. 이 파일이 있는데 읽을 수 없으면 kazma는 실행을 거부합니다.
- 모든 작업은 기록됩니다(`kz history`). 터미널이 없으면 `--yes`를 넘기지 않는 한 아무것도 삭제하지 않습니다.

## 빌드

```bash
cargo build --release                                  # on Windows -> target\release\kz.exe
rustup target add x86_64-pc-windows-gnu                # cross-compile from Linux
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

개발용으로 Linux/macOS에서도 빌드됩니다. Windows 전용 부분은 `cfg(windows)` 뒤에 있습니다.
릴리스하려면 `Cargo.toml`의 `version`을 올리고 `main`에 푸시하세요. CI가 Windows에서 테스트하고, 읽기 전용 명령을
모두 실제로 실행한 다음, `kz.exe`, zip, `SHA256SUMS.txt`와 함께 `vX.Y.Z`를 게시하고 Scoop / winget 매니페스트를
업데이트합니다.

## 스크린샷

| | |
|---|---|
| **`kz`** · English<br><img src="docs/screenshots/menu-en.png" alt="kz main menu, English"> | **`kz`** · 日本語<br><img src="docs/screenshots/menu-ja.png" alt="kz main menu, Japanese, blue theme"> |
| **`kz`** · Türkçe<br><img src="docs/screenshots/menu-tr.png" alt="kz main menu, Turkish, amber theme"> | **`kz analyze`** · `(i)`<br><img src="docs/screenshots/analyze-tr.png" alt="kz analyze"> |
| **`kz analyze`** + `b`<br><img src="docs/screenshots/analyze-largest.png" alt="kz analyze, largest files"> | **`kz analyze`** + `t`<br><img src="docs/screenshots/treemap-tr.png" alt="kz analyze, treemap"> |
| **`kz clean`** · Türkçe<br><img src="docs/screenshots/clean-tr.png" alt="kz clean, Turkish"> | **`kz clean`** · Deutsch<br><img src="docs/screenshots/clean-de.png" alt="kz clean, German, white theme"> |
| **`kz types`**<br><img src="docs/screenshots/types-en.png" alt="kz types"> | **`kz purge`**<br><img src="docs/screenshots/purge-en.png" alt="kz purge, blue theme"> |
| **`kz health`** · Windows<br><img src="docs/screenshots/health-windows.png" alt="kz health on Windows"> | **`kz health`** · Windows · 日本語<br><img src="docs/screenshots/health-windows-ja.png" alt="kz health on Windows, Japanese"> |
| **`kz startup --list`** · Windows<br><img src="docs/screenshots/startup-windows.png" alt="kz startup on Windows"> |  |

<sub>`health`와 `startup`은 실제 Windows 실행 화면(GitHub Actions, Windows Server 2025)을 캡처한 것입니다.
나머지 화면은 같은 코드를 데모 데이터로 터미널에서 실행한 것이라 경로가
`/home/user/demo`처럼 보입니다.</sub>

## 크레딧

**영감.** kazma는 [tw93](https://github.com/tw93)의 macOS용 딥 클린 도구 모음인
[Mole](https://github.com/tw93/Mole)에서 영감을 받았습니다. kazma는 Rust로 처음부터 새로 작성했으며
Mole과 코드를 공유하지 않습니다.

**번역.** 영어와 튀르키예어를 제외한 모든 언어는 [`locales/`](locales)에 영어 텍스트와 번역을 짝지은 JSON 파일로
들어 있습니다. 수정 제안은 언제든 환영합니다. 파일을 고쳐서 pull request를 열어 주세요
(`python tools/i18n_extract.py --missing de`로 해당 언어에 아직 빠진 항목을 볼 수 있습니다).

**라이브러리.** 다음 프로젝트의 작성자와 메인테이너분들께 감사드립니다:

| Crate | 용도 | 라이선스 |
|---|---|---|
| [ratatui](https://github.com/ratatui/ratatui) | 터미널 UI | MIT |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 터미널 입력 및 색상 | MIT |
| [clap](https://github.com/clap-rs/clap) | 명령줄 | MIT / Apache-2.0 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | CPU, 메모리, 디스크, 프로세스 | MIT |
| [rayon](https://github.com/rayon-rs/rayon) | 병렬 스캔 | MIT / Apache-2.0 |
| [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) | 중복 파일 해시 | CC0 / Apache-2.0 |
| [trash](https://github.com/Byron/trash-rs) | 휴지통 | MIT |
| [serde](https://github.com/serde-rs/serde), [serde_json](https://github.com/serde-rs/json) | PowerShell JSON, 번역 | MIT / Apache-2.0 |
| [anyhow](https://github.com/dtolnay/anyhow) | 오류 처리 | MIT / Apache-2.0 |
| [syn](https://github.com/dtolnay/syn), [quote](https://github.com/dtolnay/quote) | `tr!` 번역 매크로 | MIT / Apache-2.0 |
| [windows-sys](https://github.com/microsoft/windows-rs) | Windows API | MIT / Apache-2.0 |
| [winresource](https://github.com/BenjaminRi/winresource) | exe 아이콘 및 버전 정보 | MIT |

**플랫폼.** Microsoft의 [winget](https://github.com/microsoft/winget-cli)(프로그램 업데이트), Windows Update
Agent API(드라이버 업데이트), [Scoop](https://scoop.sh)(배포), 그리고
[SignPath Foundation](https://signpath.org)(오픈 소스를 위한 무료 코드 서명).

**아트.** 배너는 클래식한 [FIGlet](http://www.figlet.org) "standard" 글꼴 스타일을 사용하며, 스크린샷은
[DejaVu Sans Mono](https://dejavu-fonts.github.io) 글꼴로 렌더링했습니다.

**만든 사람.** [hhasanguclu](https://github.com/hhasanguclu). 버그 제보, 아이디어, pull request 모두 환영합니다.

## 라이선스

[MIT](LICENSE)
