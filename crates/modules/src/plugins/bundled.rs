//! Bundled enhanced plugins: built directly into the Nook binary and
//! automatically provisioned into `%APPDATA%\Nook\plugins` so Nook Enhanced Edition
//! is 100% self-contained and zero-setup on any machine.

use std::path::PathBuf;

pub struct BundledFile {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub struct BundledPlugin {
    pub id: &'static str,
    pub files: &'static [BundledFile],
}

pub static BUNDLED_PLUGINS: &[BundledPlugin] = &[
    BundledPlugin {
        id: "prayer-times",
        files: &[
            BundledFile {
                name: "plugin.toml",
                bytes: include_bytes!("../../../../plugins/prayer-times/plugin.toml"),
            },
            BundledFile {
                name: "prayer.ps1",
                bytes: include_bytes!("../../../../plugins/prayer-times/prayer.ps1"),
            },
            BundledFile {
                name: "athkar.json",
                bytes: include_bytes!("../../../../plugins/prayer-times/athkar.json"),
            },
            BundledFile {
                name: "prayer-icon.png",
                bytes: include_bytes!("../../../../plugins/prayer-times/prayer-icon.png"),
            },
        ],
    },
    BundledPlugin {
        id: "game-launcher",
        files: &[
            BundledFile {
                name: "plugin.toml",
                bytes: include_bytes!("../../../../plugins/game-launcher/plugin.toml"),
            },
            BundledFile {
                name: "launcher.ps1",
                bytes: include_bytes!("../../../../plugins/game-launcher/launcher.ps1"),
            },
            BundledFile {
                name: "NookIcon.cs",
                bytes: include_bytes!("../../../../plugins/game-launcher/NookIcon.cs"),
            },
        ],
    },
    BundledPlugin {
        id: "volume-mixer",
        files: &[
            BundledFile {
                name: "plugin.toml",
                bytes: include_bytes!("../../../../plugins/volume-mixer/plugin.toml"),
            },
            BundledFile {
                name: "mixer.ps1",
                bytes: include_bytes!("../../../../plugins/volume-mixer/mixer.ps1"),
            },
            BundledFile {
                name: "AudioMixer.cs",
                bytes: include_bytes!("../../../../plugins/volume-mixer/AudioMixer.cs"),
            },
        ],
    },
    BundledPlugin {
        id: "power-mode",
        files: &[
            BundledFile {
                name: "plugin.toml",
                bytes: include_bytes!("../../../../plugins/power-mode/plugin.toml"),
            },
            BundledFile {
                name: "power.ps1",
                bytes: include_bytes!("../../../../plugins/power-mode/power.ps1"),
            },
        ],
    },
    BundledPlugin {
        id: "dawwar",
        files: &[
            BundledFile {
                name: "plugin.toml",
                bytes: include_bytes!("../../../../plugins/dawwar/plugin.toml"),
            },
            BundledFile {
                name: "dawwar.ps1",
                bytes: include_bytes!("../../../../plugins/dawwar/dawwar.ps1"),
            },
            BundledFile {
                name: "dawwar-icon.png",
                bytes: include_bytes!("../../../../plugins/dawwar/dawwar-icon.png"),
            },
        ],
    },
];

/// Ensures all bundled plugins exist in `%APPDATA%\Nook\plugins`.
/// If files are missing or updated in this build, writes them out safely.
pub fn ensure_installed() {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return;
    };
    let plugins_dir = PathBuf::from(appdata).join("Nook").join("plugins");

    for plugin in BUNDLED_PLUGINS {
        let target_dir = plugins_dir.join(plugin.id);
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            tracing::warn!("creating plugin dir {}: {e}", target_dir.display());
            continue;
        }
        for file in plugin.files {
            let file_path = target_dir.join(file.name);
            let should_write = match std::fs::read(&file_path) {
                Ok(existing) => existing != file.bytes,
                Err(_) => true,
            };
            if should_write {
                if let Err(e) = std::fs::write(&file_path, file.bytes) {
                    tracing::warn!("writing bundled plugin file {}: {e}", file_path.display());
                } else {
                    tracing::info!("provisioned bundled plugin file {}", file_path.display());
                }
            }
        }
    }
}
