import type {
  AppConfig,
  HistoryEntry,
  LibrarySnapshot,
  MediaItem,
  PluginEqTransformResult,
  PluginRegistrySnapshot,
  ScenePreset,
  SystemMetrics,
} from "../types";

export const isTauriRuntime = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// Local browser fallback keys for seamless dev/preview without Tauri shell
const LS_CONFIG = "lofi_desktop_config_v1";
const LS_SCENES = "lofi_desktop_scenes_v1";
const LS_HISTORY = "lofi_desktop_history_v1";

const DEFAULT_CONFIG: AppConfig = {
  volume: 0.72,
  muted: false,
  activeSceneId: "rainy-window-chill",
  visual: {
    kind: "canvas",
    id: "rainy-window-cat",
    label: "Rainy Window Cat",
    streamUrl: null,
  },
  visualFx: {
    rainOverlay: 0.55,
    vignette: 0.35,
    filmGrain: 0.1,
    dim: 0.04,
    warmTint: 0.08,
    blur: 0,
    fpsCap: 30,
    pauseWhenHidden: true,
  },
  eqPresetId: "flat",
  eqBands: [0, 0, 0, 0, 0],
  tapeCutoffHz: 18500,
  ambience: {},
  audioSource: {
    kind: "station",
    id: "focus-zone-relax-mellow",
    label: "focus-zone-relax-mellow",
  },
  shuffle: false,
  repeatMode: "all",
  customMusicDirs: [],
  customVisualDirs: [],
  customSoundDirs: [],
  customPluginDirs: [],
  allowedFiles: [],
  customEqPresets: [],
  customStreams: [],
  focusMinutes: 25,
  breakMinutes: 5,
  completedFocusSessions: 0,
  focusTasks: [
    { id: "task-1", text: "Deep work session with lofi beats", done: false },
  ],
  disabledPlugins: [],
  hudAutoHide: false,
};

const DEFAULT_SCENES: ScenePreset[] = [
  {
    id: "rainy-window-chill",
    name: "Rainy Window Cat",
    mood: "CHILLING",
    description:
      "Midnight rain outside an open apartment window with a quiet silhouette companion.",
    builtin: true,
    visual: {
      kind: "canvas",
      id: "rainy-window-cat",
      label: "Rainy Window Cat",
      streamUrl: null,
    },
    visualFx: {
      rainOverlay: 0.55,
      vignette: 0.35,
      filmGrain: 0.1,
      dim: 0.04,
      warmTint: 0.08,
      blur: 0,
      fpsCap: 30,
      pauseWhenHidden: true,
    },
    eqPresetId: "flat",
    eqBands: [0, 0, 0, 0, 0],
    tapeCutoffHz: 18500,
    ambience: {},
    audioSource: {
      kind: "station",
      id: "focus-zone-relax-mellow",
      label: "focus-zone-relax-mellow",
    },
    accentColor: "#7dd3fc",
  },
  {
    id: "rain-study-session",
    name: "Rain & Vinyl Study",
    mood: "FOCUSING",
    description:
      "Gentle rain on glass, warm tape EQ, and dusty vinyl crackle for deep focus.",
    builtin: true,
    visual: {
      kind: "canvas",
      id: "rainy-window-cat",
      label: "Rainy Window Cat",
      streamUrl: null,
    },
    visualFx: {
      rainOverlay: 0.72,
      vignette: 0.4,
      filmGrain: 0.15,
      dim: 0.1,
      warmTint: 0.14,
      blur: 0,
      fpsCap: 30,
      pauseWhenHidden: true,
    },
    eqPresetId: "warm-lofi",
    eqBands: [3, 2, -0.5, -2, -3.5],
    tapeCutoffHz: 6800,
    ambience: { rain: 0.42, vinyl: 0.18 },
    audioSource: {
      kind: "station",
      id: "midnight-study-rhodes",
      label: "midnight-study-rhodes",
    },
    accentColor: "#a78bfa",
  },
  {
    id: "neon-rain-skyline",
    name: "Neon Rain Skyline",
    mood: "DRIFTING",
    description:
      "Distant city towers shimmering through late-night storm clouds and soft thunder.",
    builtin: true,
    visual: {
      kind: "canvas",
      id: "neon-rain-skyline",
      label: "Neon Rain Skyline",
      streamUrl: null,
    },
    visualFx: {
      rainOverlay: 0.65,
      vignette: 0.42,
      filmGrain: 0.12,
      dim: 0.08,
      warmTint: 0,
      blur: 0,
      fpsCap: 30,
      pauseWhenHidden: true,
    },
    eqPresetId: "late-night",
    eqBands: [4, 1.5, -1.5, -3, -4.5],
    tapeCutoffHz: 5200,
    ambience: { rain: 0.58, thunder: 0.22, wind: 0.18 },
    audioSource: {
      kind: "station",
      id: "tokyo-afterhours-dub",
      label: "tokyo-afterhours-dub",
    },
    accentColor: "#f472b6",
  },
  {
    id: "warm-hearth-dusk",
    name: "Warm Dusk Hearth",
    mood: "UNWINDING",
    description:
      "Golden dusk clouds, hearth crackle, and warm mellow keys for winding down.",
    builtin: true,
    visual: {
      kind: "canvas",
      id: "warm-study-dusk",
      label: "Warm Study Dusk",
      streamUrl: null,
    },
    visualFx: {
      rainOverlay: 0,
      vignette: 0.3,
      filmGrain: 0.14,
      dim: 0.06,
      warmTint: 0.25,
      blur: 0,
      fpsCap: 30,
      pauseWhenHidden: true,
    },
    eqPresetId: "radio-vinyl",
    eqBands: [-2, 2.5, 3, -1, -5],
    tapeCutoffHz: 4800,
    ambience: { vinyl: 0.24, fireplace: 0.3 },
    audioSource: {
      kind: "station",
      id: "sleepy-sunset-tape",
      label: "sleepy-sunset-tape",
    },
    accentColor: "#fbbf24",
  },
];

// In-memory browser-only media store when running outside Tauri
const browserLibrary: { music: MediaItem[]; visuals: MediaItem[]; ambientSounds: MediaItem[] } = {
  music: [],
  visuals: [],
  ambientSounds: [],
};

const browserDisabledPlugins = new Set<string>();

function getBrowserPluginSnapshot(): PluginRegistrySnapshot {
  const plugins = [
    {
      id: "core-atmosphere",
      name: "Core Atmosphere",
      version: "1.0.0",
      author: "Lofi Desktop",
      description: "Native Rust acoustic presets linked via unified LofiPlugin trait.",
      kind: "native" as const,
      filePath: null,
      capabilities: ["eq-presets", "eq-dsp", "scene-presets"],
      enabled: !browserDisabledPlugins.has("core-atmosphere"),
    },
    {
      id: "wasm-midnight-tape",
      name: "Midnight Tape Warmth",
      version: "1.0.0",
      author: "Lofi Wasm SDK",
      description:
        "WebAssembly DSP module that sculpts analog reel-to-reel warmth and mellow late-night roll-off.",
      kind: "wasm" as const,
      filePath: "plugins/midnight-tape.wasm",
      capabilities: ["eq-dsp", "eq-presets", "scene-presets", "event-hooks"],
      enabled: !browserDisabledPlugins.has("wasm-midnight-tape"),
    },
    {
      id: "wasm-deep-focus",
      name: "Deep Focus Clarity",
      version: "1.0.0",
      author: "Lofi Wasm SDK",
      description:
        "WebAssembly module tuned for gentle vocal-band dip and soft air presence during long study sessions.",
      kind: "wasm" as const,
      filePath: "plugins/deep-focus-boost.wasm",
      capabilities: ["eq-dsp", "eq-presets", "scene-presets", "event-hooks"],
      enabled: !browserDisabledPlugins.has("wasm-deep-focus"),
    },
  ];

  return {
    plugins,
    eqPresets: [
      ...(plugins[0].enabled
        ? [
            {
              id: "core-cassette-haze",
              name: "Cassette Haze",
              bands: [2.5, 1.8, -0.8, -2.5, -4.2] as [number, number, number, number, number],
              tapeCutoffHz: 5600,
              source: "plugin:core-atmosphere",
            },
          ]
        : []),
      ...(plugins[1].enabled
        ? [
            {
              id: "wasm-midnight-reel",
              name: "Midnight Reel (Wasm)",
              bands: [3.8, 2.2, -1.0, -2.8, -5.2] as [number, number, number, number, number],
              tapeCutoffHz: 4600,
              source: "plugin:wasm-midnight-tape",
            },
          ]
        : []),
      ...(plugins[2].enabled
        ? [
            {
              id: "wasm-study-clarity",
              name: "Study Clarity (Wasm)",
              bands: [1.4, 0.8, -1.8, 1.2, -1.5] as [number, number, number, number, number],
              tapeCutoffHz: 11200,
              source: "plugin:wasm-deep-focus",
            },
          ]
        : []),
    ],
    sceneContributions: [
      ...(plugins[1].enabled
        ? [
            {
              id: "wasm-midnight-tape-scene",
              name: "Analog Midnight Reel",
              mood: "LATE NIGHT",
              description: "Wasm-crafted reel-to-reel warmth paired with soft storm rain.",
              visualId: "rainy-window-cat",
              eqBands: [3.8, 2.2, -1.0, -2.8, -5.2] as [number, number, number, number, number],
              tapeCutoffHz: 4600,
              ambience: { rain: 0.52, vinyl: 0.32 },
              accentColor: "#38bdf8",
              pluginId: "wasm-midnight-tape",
            },
          ]
        : []),
      ...(plugins[2].enabled
        ? [
            {
              id: "wasm-deep-focus-scene",
              name: "Library Rain Focus",
              mood: "DEEP FOCUS",
              description: "Balanced Wasm acoustic profile for distraction-free coding.",
              visualId: "neon-rain-skyline",
              eqBands: [1.4, 0.8, -1.8, 1.2, -1.5] as [number, number, number, number, number],
              tapeCutoffHz: 11200,
              ambience: { rain: 0.35, fireplace: 0.2 },
              accentColor: "#34d399",
              pluginId: "wasm-deep-focus",
            },
          ]
        : []),
    ],
    loadErrors: [],
  };
}

export const backend = {
  async getConfig(): Promise<AppConfig> {
    if (isTauriRuntime()) {
      return tauriInvoke<AppConfig>("get_config");
    }
    try {
      const raw = localStorage.getItem(LS_CONFIG);
      if (raw) return { ...DEFAULT_CONFIG, ...JSON.parse(raw) };
    } catch {
      // ignore
    }
    return structuredClone(DEFAULT_CONFIG);
  },

  async saveConfig(config: AppConfig): Promise<AppConfig> {
    if (isTauriRuntime()) {
      return tauriInvoke<AppConfig>("save_config", { config });
    }
    localStorage.setItem(LS_CONFIG, JSON.stringify(config));
    return config;
  },

  async scanLibrary(): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<LibrarySnapshot>("scan_library");
    }
    return {
      music: [...browserLibrary.music],
      visuals: [...browserLibrary.visuals],
      ambientSounds: [...browserLibrary.ambientSounds],
      directories: {
        appDataDir: "Local Browser Preview",
        defaultMusicDir: "AppData/lofi-desktop/music",
        defaultVisualsDir: "AppData/lofi-desktop/visuals",
        defaultSoundsDir: "AppData/lofi-desktop/sounds",
        defaultPluginsDir: "AppData/lofi-desktop/plugins",
        customMusicDirs: [],
        customVisualDirs: [],
        customSoundDirs: [],
        customPluginDirs: [],
      },
    };
  },

  async addLibraryDirectory(
    category: "music" | "visuals" | "sounds" | "plugins",
    path: string
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<LibrarySnapshot>("add_library_directory", { category, path });
    }
    return this.scanLibrary();
  },

  async removeLibraryDirectory(
    category: "music" | "visuals" | "sounds" | "plugins",
    path: string
  ): Promise<LibrarySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<LibrarySnapshot>("remove_library_directory", { category, path });
    }
    return this.scanLibrary();
  },

  async registerLocalFile(filePath: string): Promise<MediaItem> {
    if (isTauriRuntime()) {
      return tauriInvoke<MediaItem>("register_local_file", { filePath });
    }
    throw new Error("Direct path registration requires Tauri desktop runtime");
  },

  registerBrowserFile(file: File, forceAmbient = false): MediaItem | null {
    const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
    const url = URL.createObjectURL(file);
    const title = file.name.replace(/\.[^/.]+$/, "");
    let kind: MediaItem["kind"] | null = null;
    if (["mp4", "webm", "mov", "m4v", "ogv"].includes(ext)) kind = "video";
    else if (["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif"].includes(ext)) kind = "image";
    else if (["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus"].includes(ext)) {
      kind = forceAmbient ? "ambient" : "audio";
    }
    if (!kind) return null;

    const item: MediaItem = {
      id: `blob-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      title,
      subtitle: "Local File",
      filePath: file.name,
      streamUrl: url,
      kind,
      extension: ext,
      sizeBytes: file.size,
      modifiedUnix: Math.floor(file.lastModified / 1000),
    };

    if (kind === "audio") browserLibrary.music.unshift(item);
    else if (kind === "ambient") browserLibrary.ambientSounds.unshift(item);
    else browserLibrary.visuals.unshift(item);
    return item;
  },

  async pickFolder(title: string): Promise<string | null> {
    if (!isTauriRuntime()) return null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, multiple: false, title });
    return typeof selected === "string" ? selected : null;
  },

  async pickFiles(
    title: string,
    extensions: string[]
  ): Promise<string[]> {
    if (!isTauriRuntime()) return [];
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: false,
      multiple: true,
      title,
      filters: [{ name: title, extensions }],
    });
    if (Array.isArray(selected)) return selected;
    if (typeof selected === "string") return [selected];
    return [];
  },

  async getScenes(): Promise<ScenePreset[]> {
    if (isTauriRuntime()) {
      return tauriInvoke<ScenePreset[]>("get_scenes");
    }
    try {
      const raw = localStorage.getItem(LS_SCENES);
      if (raw) {
        const userScenes = JSON.parse(raw) as ScenePreset[];
        return [...DEFAULT_SCENES, ...userScenes.filter((s) => !s.builtin)];
      }
    } catch {
      // ignore
    }
    return structuredClone(DEFAULT_SCENES);
  },

  async saveScene(scene: ScenePreset): Promise<ScenePreset[]> {
    if (isTauriRuntime()) {
      return tauriInvoke<ScenePreset[]>("save_scene", { scene });
    }
    const all = await this.getScenes();
    const userOnly = all.filter((s) => !s.builtin && s.id !== scene.id);
    userOnly.push({ ...scene, builtin: false });
    localStorage.setItem(LS_SCENES, JSON.stringify(userOnly));
    return [...DEFAULT_SCENES, ...userOnly];
  },

  async deleteScene(sceneId: string): Promise<ScenePreset[]> {
    if (isTauriRuntime()) {
      return tauriInvoke<ScenePreset[]>("delete_scene", { sceneId });
    }
    const all = await this.getScenes();
    const userOnly = all.filter((s) => !s.builtin && s.id !== sceneId);
    localStorage.setItem(LS_SCENES, JSON.stringify(userOnly));
    return [...DEFAULT_SCENES, ...userOnly];
  },

  async getHistory(): Promise<HistoryEntry[]> {
    if (isTauriRuntime()) {
      return tauriInvoke<HistoryEntry[]>("get_history");
    }
    try {
      const raw = localStorage.getItem(LS_HISTORY);
      if (raw) return JSON.parse(raw) as HistoryEntry[];
    } catch {
      // ignore
    }
    return [];
  },

  async appendHistory(
    title: string,
    subtitle: string,
    kind: HistoryEntry["kind"],
    refId: string
  ): Promise<HistoryEntry[]> {
    if (isTauriRuntime()) {
      return tauriInvoke<HistoryEntry[]>("append_history", { title, subtitle, kind, refId });
    }
    const list = await this.getHistory();
    const entry: HistoryEntry = {
      id: `hist-${Date.now()}`,
      title,
      subtitle,
      kind,
      refId,
      timestampUnix: Math.floor(Date.now() / 1000),
    };
    if (list[0]?.refId === refId && list[0]?.kind === kind) {
      list[0] = entry;
    } else {
      list.unshift(entry);
    }
    const trimmed = list.slice(0, 80);
    localStorage.setItem(LS_HISTORY, JSON.stringify(trimmed));
    return trimmed;
  },

  async clearHistory(): Promise<void> {
    if (isTauriRuntime()) {
      await tauriInvoke<void>("clear_history");
      return;
    }
    localStorage.removeItem(LS_HISTORY);
  },

  async listPlugins(): Promise<PluginRegistrySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<PluginRegistrySnapshot>("list_plugins");
    }
    return getBrowserPluginSnapshot();
  },

  async reloadPlugins(): Promise<PluginRegistrySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<PluginRegistrySnapshot>("reload_plugins");
    }
    return getBrowserPluginSnapshot();
  },

  async setPluginEnabled(pluginId: string, enabled: boolean): Promise<PluginRegistrySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<PluginRegistrySnapshot>("set_plugin_enabled", { pluginId, enabled });
    }
    if (enabled) browserDisabledPlugins.delete(pluginId);
    else browserDisabledPlugins.add(pluginId);
    return getBrowserPluginSnapshot();
  },

  async installWasmPlugin(sourcePath: string): Promise<PluginRegistrySnapshot> {
    if (isTauriRuntime()) {
      return tauriInvoke<PluginRegistrySnapshot>("install_wasm_plugin", { sourcePath });
    }
    return getBrowserPluginSnapshot();
  },

  async runPluginEqTransform(
    pluginId: string,
    bands: [number, number, number, number, number],
    intensity: number
  ): Promise<PluginEqTransformResult> {
    if (isTauriRuntime()) {
      return tauriInvoke<PluginEqTransformResult>("run_plugin_eq_transform", {
        pluginId,
        bands,
        intensity,
      });
    }
    const deltas =
      pluginId === "wasm-deep-focus"
        ? [1.4, 0.8, -1.8, 1.2, -1.5]
        : [3.8, 2.2, -1.0, -2.8, -5.2];
    const out = bands.map((v, i) =>
      Math.round(Math.max(-12, Math.min(12, v + deltas[i] * intensity)) * 10) / 10
    ) as [number, number, number, number, number];
    return {
      pluginId,
      pluginName: pluginId,
      inputBands: bands,
      outputBands: out,
      suggestedTapeCutoffHz: pluginId === "wasm-deep-focus" ? 11200 : 4600,
      note: "Applied plugin EQ transform",
    };
  },

  async notifyFocusComplete(totalSessions: number): Promise<number> {
    if (isTauriRuntime()) {
      return tauriInvoke<number>("notify_focus_complete", { totalSessions });
    }
    return 0;
  },

  async getSystemMetrics(): Promise<SystemMetrics> {
    if (isTauriRuntime()) {
      return tauriInvoke<SystemMetrics>("get_system_metrics");
    }
    return {
      uptimeSecs: Math.floor(performance.now() / 1000),
      loadedPlugins: 3,
      wasmPlugins: 2,
      indexedTracks: browserLibrary.music.length,
      indexedVisuals: browserLibrary.visuals.length,
    };
  },

  async toggleFullscreen(): Promise<boolean> {
    if (isTauriRuntime()) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const current = await win.isFullscreen();
      await win.setFullscreen(!current);
      return !current;
    }
    if (!document.fullscreenElement) {
      await document.documentElement.requestFullscreen?.();
      return true;
    } else {
      await document.exitFullscreen?.();
      return false;
    }
  },
};
