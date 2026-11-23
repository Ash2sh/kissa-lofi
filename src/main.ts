import {
  AMBIENT_CHANNELS,
  BUILTIN_EQ_PRESETS,
  BUILTIN_STATIONS,
  LofiAudioEngine,
} from "./services/audioEngine";
import { backend, isTauriRuntime } from "./services/backend";
import { BUILTIN_CANVAS_SCENES, VisualEngine } from "./services/visualEngine";
import type {
  AppConfig,
  EqPreset,
  HistoryEntry,
  LibrarySnapshot,
  MediaItem,
  ModuleId,
  PluginRegistrySnapshot,
  PluginSceneContribution,
  ScenePreset,
} from "./types";
import { DOCK_MODULES, DockController } from "./ui/dock";
import { icons } from "./ui/icons";
import {
  formatClock,
  PopoverController,
  type PopoverContext,
  type PlaybackProgress,
  type TimerViewState,
} from "./ui/popovers";

class LofiApp {
  private audio = new LofiAudioEngine();
  private visual!: VisualEngine;
  private dock!: DockController;
  private popovers!: PopoverController;

  private config!: AppConfig;
  private library!: LibrarySnapshot;
  private scenes: ScenePreset[] = [];
  private history: HistoryEntry[] = [];
  private plugins!: PluginRegistrySnapshot;

  private hudHidden = false;
  private saveDebounce: number | null = null;
  private progress: PlaybackProgress = { currentTime: 0, duration: 0 };

  private timers: TimerViewState = {
    focusRunning: false,
    focusMode: "focus",
    focusRemainingSec: 25 * 60,
    sleepRunning: false,
    sleepRemainingSec: 0,
    sleepTotalSec: 0,
  };

  private appRoot = document.getElementById("app")!;
  private topHudLeft = document.getElementById("top-hud-left")!;
  private topHudRight = document.getElementById("top-hud-right")!;
  private popoverHost = document.getElementById("popover-host")!;
  private dockEl = document.getElementById("dock")!;
  private restoreBtn = document.getElementById("hud-restore-btn")!;
  private dropOverlay = document.getElementById("drop-overlay")!;
  private toastHost = document.getElementById("toast-host")!;
  private browserFileInput = document.getElementById("browser-file-input") as HTMLInputElement;

  public async init(): Promise<void> {
    const canvas = document.getElementById("bg-canvas") as HTMLCanvasElement;
    const video = document.getElementById("bg-video") as HTMLVideoElement;
    const image = document.getElementById("bg-image") as HTMLImageElement;
    const dimLayer = document.getElementById("bg-dim-layer") as HTMLDivElement;

    this.visual = new VisualEngine(canvas, video, image, dimLayer);

    // Load local state & Wasm plugins in parallel from Rust backend
    const [config, library, scenes, history, plugins] = await Promise.all([
      backend.getConfig(),
      backend.scanLibrary(),
      backend.getScenes(),
      backend.getHistory(),
      backend.listPlugins(),
    ]);

    this.config = config;
    this.library = library;
    this.scenes = scenes;
    this.history = history;
    this.plugins = plugins;
    this.timers.focusRemainingSec = (this.config.focusMinutes || 25) * 60;

    // Apply persisted visual & audio settings
    this.applyVisualFromConfig();
    this.audio.setVolume(this.config.volume);
    this.audio.setMuted(this.config.muted);
    this.audio.setEqBands(this.config.eqBands);
    this.audio.setTapeCutoff(this.config.tapeCutoffHz);
    this.audio.setAmbienceMap(this.config.ambience);

    this.audio.onTrackEnded = () => this.handleTrackEnded();
    this.audio.onPlaybackError = (msg) => this.toast(msg);
    this.audio.onTimeUpdate = (cur, dur) => {
      this.progress = { currentTime: cur, duration: dur };
      this.popovers.updateLiveTick(this.popoverCtx());
    };

    // Mount UI controllers
    this.dock = new DockController(this.dockEl, {
      onTogglePlay: () => void this.togglePlay(),
      onToggleMute: () => this.toggleMute(),
      onVolumeChange: (v) => this.setVolume(v),
      onToggleModule: (m) => this.toggleModule(m),
      onTrackClick: () => this.toggleModule("music"),
    });

    this.popovers = new PopoverController(this.popoverHost, {
      onClose: () => this.closePopover(),
      onSetEqBand: (idx, db) => {
        this.config.eqBands[idx] = db;
        this.config.eqPresetId = "custom";
        this.audio.setEqBands(this.config.eqBands);
        this.refreshDock();
        this.scheduleConfigSave();
      },
      onSetTapeCutoff: (hz) => {
        this.config.tapeCutoffHz = hz;
        this.audio.setTapeCutoff(hz);
        this.scheduleConfigSave();
      },
      onApplyEqPreset: (preset) => {
        this.config.eqPresetId = preset.id;
        this.config.eqBands = [...preset.bands] as [number, number, number, number, number];
        this.config.tapeCutoffHz = preset.tapeCutoffHz;
        this.audio.setEqBands(this.config.eqBands);
        this.audio.setTapeCutoff(this.config.tapeCutoffHz);
        this.refreshAll();
        this.scheduleConfigSave();
        this.toast(`EQ preset: ${preset.name}`);
      },
      onResetEq: () => {
        this.config.eqPresetId = "flat";
        this.config.eqBands = [0, 0, 0, 0, 0];
        this.config.tapeCutoffHz = 18500;
        this.audio.setEqBands(this.config.eqBands);
        this.audio.setTapeCutoff(18500);
        this.refreshAll();
        this.scheduleConfigSave();
      },
      onSaveCustomEqPreset: (name) => {
        const preset: EqPreset = {
          id: `custom-${Date.now()}`,
          name,
          bands: [...this.config.eqBands] as [number, number, number, number, number],
          tapeCutoffHz: this.config.tapeCutoffHz,
          source: "user",
        };
        this.config.customEqPresets.push(preset);
        this.config.eqPresetId = preset.id;
        this.refreshAll();
        this.scheduleConfigSave();
        this.toast(`Saved EQ preset "${name}"`);
      },
      onRunPluginEqTransform: (pluginId, intensity) =>
        void this.runPluginEqTransform(pluginId, intensity),

      onSelectStation: (id) => void this.selectStation(id, true),
      onPlayLocalTrack: (item) => void this.playLocalTrack(item, true),
      onPlayStream: (id, name, mood, url) => void this.playStream(id, name, mood, url),
      onAddCustomStream: (name, mood, url) => {
        this.config.customStreams.push({
          id: `stream-${Date.now()}`,
          name,
          mood,
          url,
        });
        this.scheduleConfigSave();
        this.refreshPopover();
        this.toast(`Added stream "${name}"`);
      },
      onRemoveCustomStream: (id) => {
        this.config.customStreams = this.config.customStreams.filter((s) => s.id !== id);
        this.scheduleConfigSave();
        this.refreshPopover();
      },
      onNextTrack: () => void this.stepTrack(1),
      onPrevTrack: () => void this.stepTrack(-1),
      onToggleShuffle: () => {
        this.config.shuffle = !this.config.shuffle;
        this.scheduleConfigSave();
        this.refreshPopover();
      },
      onCycleRepeat: () => {
        const order: AppConfig["repeatMode"][] = ["all", "one", "off"];
        const next = order[(order.indexOf(this.config.repeatMode) + 1) % order.length];
        this.config.repeatMode = next;
        this.scheduleConfigSave();
        this.refreshPopover();
      },
      onSeek: (sec) => this.audio.seekTo(sec),

      onAddDirectory: (category) => void this.addDirectory(category),
      onRemoveDirectory: (category, path) => void this.removeDirectory(category, path),
      onPickLocalFiles: (category) => void this.pickLocalFiles(category),
      onRescanLibrary: () => void this.rescanLibrary(),

      onSelectVisual: (source) => {
        this.config.visual = source;
        this.visual.setSource(source);
        this.scheduleConfigSave();
        this.refreshAll();
        this.toast(`Background: ${source.label}`);
      },
      onUpdateVisualFx: (partial) => {
        this.config.visualFx = { ...this.config.visualFx, ...partial };
        this.visual.setFx(this.config.visualFx);
        this.scheduleConfigSave();
        if (partial.fpsCap !== undefined) this.refreshPopover();
      },

      onSetAmbience: (id, level, customUrl) => {
        if (level <= 0.005) {
          delete this.config.ambience[id];
        } else {
          this.config.ambience[id] = level;
        }
        this.audio.setAmbienceLevel(id, level, customUrl);
        this.refreshDock();
        this.refreshPopover();
        this.scheduleConfigSave();
      },
      onClearAmbience: () => {
        this.config.ambience = {};
        this.audio.clearAmbience();
        this.refreshAll();
        this.scheduleConfigSave();
      },

      onApplyScene: (scene) => void this.applyScene(scene),
      onApplyPluginScene: (ps) => void this.applyPluginScene(ps),
      onSaveCurrentScene: (name, mood) => void this.saveCurrentScene(name, mood),
      onDeleteScene: (id) => void this.deleteScene(id),
      onTogglePlugin: (id, enabled) => void this.togglePlugin(id, enabled),
      onReloadPlugins: () => void this.reloadPlugins(),
      onInstallWasmPlugin: () => void this.installWasmPlugin(),

      onReplayHistory: (entry) => void this.replayHistory(entry),
      onClearHistory: () => void this.clearHistory(),

      onCopyShareCode: () => void this.copyShareCode(),
      onImportShareCode: (raw) => void this.importShareCode(raw),
      onExportSceneFile: () => this.exportSceneFile(),

      onToggleHud: () => this.toggleHud(),
      onToggleFullscreen: () => void backend.toggleFullscreen(),

      onStartPauseFocus: () => {
        this.timers.focusRunning = !this.timers.focusRunning;
        this.refreshTopHud();
        this.refreshDock();
        this.refreshPopover();
      },
      onResetFocus: (mode, minutes) => {
        this.timers.focusRunning = false;
        if (mode) this.timers.focusMode = mode;
        const mins =
          minutes ??
          (this.timers.focusMode === "focus" ? this.config.focusMinutes : this.config.breakMinutes);
        this.timers.focusRemainingSec = mins * 60;
        this.refreshTopHud();
        this.refreshDock();
        this.refreshPopover();
      },
      onAddFocusTask: (text) => {
        this.config.focusTasks.unshift({
          id: `task-${Date.now()}`,
          text,
          done: false,
        });
        this.scheduleConfigSave();
        this.refreshPopover();
      },
      onToggleFocusTask: (id) => {
        const task = this.config.focusTasks.find((t) => t.id === id);
        if (task) task.done = !task.done;
        this.scheduleConfigSave();
        this.refreshPopover();
      },
      onDeleteFocusTask: (id) => {
        this.config.focusTasks = this.config.focusTasks.filter((t) => t.id !== id);
        this.scheduleConfigSave();
        this.refreshPopover();
      },

      onStartSleep: (minutes) => {
        this.timers.sleepRunning = true;
        this.timers.sleepTotalSec = minutes * 60;
        this.timers.sleepRemainingSec = minutes * 60;
        this.audio.setSleepFade(1);
        this.refreshTopHud();
        this.refreshDock();
        this.refreshPopover();
        this.toast(`Sleep timer set for ${minutes} min`);
      },
      onCancelSleep: () => {
        this.timers.sleepRunning = false;
        this.timers.sleepRemainingSec = 0;
        this.audio.setSleepFade(1);
        this.refreshTopHud();
        this.refreshDock();
        this.refreshPopover();
        this.toast("Sleep timer cancelled");
      },
    });

    // Prepare initial audio source without violating autoplay policy
    await this.prepareInitialAudioSource();

    this.visual.start();
    this.refreshAll();
    this.bindGlobalEvents();
    this.startTimerLoop();
  }

  private async prepareInitialAudioSource(): Promise<void> {
    if (this.config.audioSource.kind === "local") {
      const match = this.library.music.find((m) => m.id === this.config.audioSource.id);
      if (match) {
        await this.audio.playMediaUrl(match.streamUrl, "local", false);
        return;
      }
    }
    await this.audio.selectStation(this.config.audioSource.id, false);
  }

  private applyVisualFromConfig(): void {
    const v = this.config.visual;
    if (v.kind !== "canvas" && !v.streamUrl) {
      const match = this.library.visuals.find((item) => item.filePath === v.id);
      if (match) {
        this.config.visual.streamUrl = match.streamUrl;
      } else {
        this.config.visual = {
          kind: "canvas",
          id: "rainy-window-cat",
          label: "Rainy Window Cat",
          streamUrl: null,
        };
      }
    }
    this.visual.setSource(this.config.visual);
    this.visual.setFx(this.config.visualFx);
  }

  private popoverCtx(): PopoverContext {
    return {
      config: this.config,
      library: this.library,
      scenes: this.scenes,
      history: this.history,
      plugins: this.plugins,
      timers: this.timers,
      progress: this.progress,
      isTauri: isTauriRuntime(),
    };
  }

  private refreshAll(): void {
    this.refreshDock();
    this.refreshTopHud();
    this.refreshPopover();
  }

  private refreshPopover(): void {
    this.popovers.render(this.popoverCtx());
  }

  private refreshDock(): void {
    const { mood, title, subtitle } = this.currentTrackDisplay();
    const eqName = this.currentEqName();
    const activeAmbience = Object.entries(this.config.ambience)
      .filter(([, v]) => v > 0.01)
      .map(([id]) => AMBIENT_CHANNELS.find((c) => c.id === id)?.name ?? "Custom");
    const ambSummary =
      activeAmbience.length === 0
        ? "No ambience"
        : activeAmbience.length <= 2
        ? activeAmbience.join(", ")
        : `${activeAmbience[0]} +${activeAmbience.length - 1}`;

    this.dock.render({
      mood,
      title,
      subtitle,
      statusLine: `${eqName} · ${ambSummary}`,
      playing: this.audio.getPlaying(),
      volume: this.config.volume,
      muted: this.config.muted,
      activeModule: this.popovers.getActiveModule(),
      scenesDot: Boolean(this.config.activeSceneId) || this.plugins.plugins.some((p) => p.kind === "wasm" && p.enabled),
      soundsActive: activeAmbience.length > 0,
      focusActive: this.timers.focusRunning,
      sleepActive: this.timers.sleepRunning,
    });
  }

  private refreshTopHud(): void {
    const leftParts: string[] = [];
    if (this.timers.focusRunning) {
      leftParts.push(`
        <button type="button" class="hud-pill hud-pill--timer" data-hud-open="focus">
          <span class="dot"></span>
          <span>${this.timers.focusMode === "focus" ? "Focus" : "Break"} ${formatClock(this.timers.focusRemainingSec)}</span>
        </button>
      `);
    }
    if (this.timers.sleepRunning) {
      leftParts.push(`
        <button type="button" class="hud-pill hud-pill--timer hud-pill--sleep" data-hud-open="sleep">
          <span class="dot"></span>
          <span>Sleep ${formatClock(this.timers.sleepRemainingSec)}</span>
        </button>
      `);
    }
    this.topHudLeft.innerHTML = leftParts.join("");
    this.topHudLeft.querySelectorAll<HTMLElement>("[data-hud-open]").forEach((el) => {
      el.addEventListener("click", () => this.toggleModule(el.dataset.hudOpen as ModuleId));
    });

    const wasmActive = this.plugins.plugins.filter((p) => p.kind === "wasm" && p.enabled).length;
    this.topHudRight.innerHTML = `
      <button type="button" class="hud-pill hud-pill--quiet" data-hud-wasm title="WebAssembly Plugins">
        ${icons.cpu}
        <span>${wasmActive} Wasm</span>
      </button>
      <button type="button" class="hud-pill hud-pill--quiet" data-hud-cinema title="Hide HUD (H)">
        ${icons.eyeOff}
      </button>
      <button type="button" class="hud-pill hud-pill--quiet" data-hud-fs title="Fullscreen (F)">
        ${icons.fullscreen}
      </button>
    `;
    this.topHudRight.querySelector("[data-hud-wasm]")?.addEventListener("click", () => {
      this.openModule("scenes");
    });
    this.topHudRight.querySelector("[data-hud-cinema]")?.addEventListener("click", () => {
      this.toggleHud();
    });
    this.topHudRight.querySelector("[data-hud-fs]")?.addEventListener("click", () => {
      void backend.toggleFullscreen();
    });
  }

  private currentTrackDisplay(): { mood: string; title: string; subtitle: string } {
    const src = this.config.audioSource;
    if (src.kind === "local") {
      const track = this.library.music.find((m) => m.id === src.id);
      return {
        mood: "LOCAL LIBRARY",
        title: track?.title ?? src.label,
        subtitle: `${this.library.music.length} local tracks`,
      };
    }
    if (src.kind === "stream") {
      const st = this.config.customStreams.find((s) => s.id === src.id);
      return {
        mood: st?.mood || "LIVE STREAM",
        title: st?.name || src.label,
        subtitle: "Custom stream",
      };
    }
    const station = BUILTIN_STATIONS.find((s) => s.id === src.id) ?? BUILTIN_STATIONS[0];
    return {
      mood: station.mood,
      title: station.name,
      subtitle: `${station.listeners} listening`,
    };
  }

  private currentEqName(): string {
    const all = [
      ...BUILTIN_EQ_PRESETS,
      ...this.plugins.eqPresets,
      ...this.config.customEqPresets,
    ];
    return all.find((p) => p.id === this.config.eqPresetId)?.name ?? "Custom EQ";
  }

  // --- Playback Actions ---
  private async togglePlay(): Promise<void> {
    await this.audio.togglePlay();
    this.refreshDock();
  }

  private toggleMute(): void {
    this.config.muted = !this.config.muted;
    this.audio.setMuted(this.config.muted);
    this.refreshDock();
    this.scheduleConfigSave();
  }

  private setVolume(vol: number): void {
    this.config.volume = Math.max(0, Math.min(1, vol));
    if (this.config.volume > 0 && this.config.muted) {
      this.config.muted = false;
      this.audio.setMuted(false);
    }
    this.audio.setVolume(this.config.volume);
    this.refreshDock();
    this.scheduleConfigSave();
  }

  private async selectStation(stationId: string, recordHistory = true): Promise<void> {
    const station = await this.audio.selectStation(stationId, true);
    this.config.audioSource = {
      kind: "station",
      id: station.id,
      label: station.name,
    };
    this.scheduleConfigSave();
    if (recordHistory) {
      this.history = await backend.appendHistory(
        station.name,
        station.mood,
        "station",
        station.id
      );
    }
    this.refreshAll();
  }

  private async playLocalTrack(item: MediaItem, recordHistory = true): Promise<void> {
    this.config.audioSource = {
      kind: "local",
      id: item.id,
      label: item.title,
    };
    await this.audio.playMediaUrl(item.streamUrl, "local", true);
    this.scheduleConfigSave();
    if (recordHistory) {
      this.history = await backend.appendHistory(item.title, item.subtitle, "track", item.id);
    }
    this.refreshAll();
  }

  private async playStream(id: string, name: string, mood: string, url: string): Promise<void> {
    this.config.audioSource = { kind: "stream", id, label: name };
    await this.audio.playMediaUrl(url, "stream", true);
    this.scheduleConfigSave();
    this.history = await backend.appendHistory(name, mood, "station", id);
    this.refreshAll();
  }

  private async handleTrackEnded(): Promise<void> {
    if (this.config.audioSource.kind !== "local" || this.library.music.length === 0) return;
    if (this.config.repeatMode === "one") {
      const current = this.library.music.find((m) => m.id === this.config.audioSource.id);
      if (current) await this.playLocalTrack(current, false);
      return;
    }
    await this.stepTrack(1);
  }

  private async stepTrack(delta: number): Promise<void> {
    if (this.config.audioSource.kind === "local" && this.library.music.length > 0) {
      const list = this.library.music;
      let nextIdx: number;
      if (this.config.shuffle && list.length > 1) {
        nextIdx = Math.floor(Math.random() * list.length);
      } else {
        const cur = list.findIndex((m) => m.id === this.config.audioSource.id);
        nextIdx = (cur + delta + list.length) % list.length;
      }
      await this.playLocalTrack(list[nextIdx], true);
      return;
    }

    // Cycle built-in stations when not in local library mode
    const cur = BUILTIN_STATIONS.findIndex((s) => s.id === this.config.audioSource.id);
    const next = BUILTIN_STATIONS[(cur + delta + BUILTIN_STATIONS.length) % BUILTIN_STATIONS.length];
    await this.selectStation(next.id, true);
  }

  // --- Local File & Folder Management ---
  private async addDirectory(category: "music" | "visuals" | "sounds" | "plugins"): Promise<void> {
    const folder = await backend.pickFolder(`Select local ${category} folder`);
    if (!folder) return;
    try {
      this.library = await backend.addLibraryDirectory(category, folder);
      if (category === "plugins") {
        this.plugins = await backend.listPlugins();
      }
      this.refreshAll();
      this.toast(`Added ${category} folder`);
    } catch (err) {
      this.toast(String(err));
    }
  }

  private async removeDirectory(
    category: "music" | "visuals" | "sounds" | "plugins",
    path: string
  ): Promise<void> {
    this.library = await backend.removeLibraryDirectory(category, path);
    if (category === "plugins") {
      this.plugins = await backend.listPlugins();
    }
    this.refreshAll();
  }

  private async pickLocalFiles(category: "music" | "visuals" | "sounds"): Promise<void> {
    if (isTauriRuntime()) {
      const exts =
        category === "visuals"
          ? ["mp4", "webm", "mov", "jpg", "jpeg", "png", "webp", "gif"]
          : ["mp3", "wav", "flac", "ogg", "m4a", "aac", "opus"];
      const paths = await backend.pickFiles(`Select ${category} files`, exts);
      if (paths.length === 0) return;

      let lastItem: MediaItem | null = null;
      for (const p of paths) {
        try {
          lastItem = await backend.registerLocalFile(p);
        } catch (err) {
          this.toast(String(err));
        }
      }
      this.library = await backend.scanLibrary();
      if (lastItem) {
        if (lastItem.kind === "image" || lastItem.kind === "video") {
          this.config.visual = {
            kind: lastItem.kind,
            id: lastItem.filePath,
            label: lastItem.title,
            streamUrl: lastItem.streamUrl,
          };
          this.visual.setSource(this.config.visual);
          this.scheduleConfigSave();
        } else if (category === "music") {
          await this.playLocalTrack(lastItem, true);
        }
      }
      this.refreshAll();
      this.toast(`Added ${paths.length} local file(s)`);
      return;
    }

    // Browser preview fallback
    this.browserFileInput.accept =
      category === "visuals" ? "image/*,video/*" : "audio/*";
    this.browserFileInput.onchange = async () => {
      const files = Array.from(this.browserFileInput.files || []);
      let lastItem: MediaItem | null = null;
      for (const f of files) {
        const item = backend.registerBrowserFile(f, category === "sounds");
        if (item) lastItem = item;
      }
      this.library = await backend.scanLibrary();
      if (lastItem) {
        if (lastItem.kind === "image" || lastItem.kind === "video") {
          this.config.visual = {
            kind: lastItem.kind,
            id: lastItem.filePath,
            label: lastItem.title,
            streamUrl: lastItem.streamUrl,
          };
          this.visual.setSource(this.config.visual);
        } else if (lastItem.kind === "audio") {
          await this.playLocalTrack(lastItem, true);
        }
      }
      this.browserFileInput.value = "";
      this.refreshAll();
    };
    this.browserFileInput.click();
  }

  private async rescanLibrary(): Promise<void> {
    this.library = await backend.scanLibrary();
    this.refreshAll();
    this.toast(`Indexed ${this.library.music.length} tracks & ${this.library.visuals.length} visuals`);
  }

  // --- Scenes & WebAssembly Plugins ---
  private async applyScene(scene: ScenePreset): Promise<void> {
    this.config.activeSceneId = scene.id;
    this.config.visual = { ...scene.visual };
    this.config.visualFx = { ...scene.visualFx };
    this.config.eqPresetId = scene.eqPresetId;
    this.config.eqBands = [...scene.eqBands] as [number, number, number, number, number];
    this.config.tapeCutoffHz = scene.tapeCutoffHz;
    this.config.ambience = { ...scene.ambience };
    this.config.audioSource = { ...scene.audioSource };

    this.applyVisualFromConfig();
    this.audio.setEqBands(this.config.eqBands);
    this.audio.setTapeCutoff(this.config.tapeCutoffHz);
    this.audio.setAmbienceMap(this.config.ambience);

    if (scene.audioSource.kind === "station") {
      await this.audio.selectStation(scene.audioSource.id, this.audio.getPlaying());
    }

    this.scheduleConfigSave();
    this.history = await backend.appendHistory(scene.name, scene.mood, "scene", scene.id);
    this.refreshAll();
    this.toast(`Scene: ${scene.name}`);
  }

  private async applyPluginScene(ps: PluginSceneContribution): Promise<void> {
    const canvasDef =
      BUILTIN_CANVAS_SCENES.find((s) => s.id === ps.visualId) ?? BUILTIN_CANVAS_SCENES[0];
    this.config.activeSceneId = ps.id;
    this.config.visual = {
      kind: "canvas",
      id: canvasDef.id,
      label: canvasDef.name,
      streamUrl: null,
    };
    this.config.eqPresetId = `plugin-${ps.id}`;
    this.config.eqBands = [...ps.eqBands] as [number, number, number, number, number];
    this.config.tapeCutoffHz = ps.tapeCutoffHz;
    this.config.ambience = { ...ps.ambience };

    this.visual.setSource(this.config.visual);
    this.audio.setEqBands(this.config.eqBands);
    this.audio.setTapeCutoff(this.config.tapeCutoffHz);
    this.audio.setAmbienceMap(this.config.ambience);

    this.scheduleConfigSave();
    this.history = await backend.appendHistory(ps.name, `Plugin · ${ps.mood}`, "scene", ps.id);
    this.refreshAll();
    this.toast(`Applied plugin scene: ${ps.name}`);
  }

  private async saveCurrentScene(name: string, mood: string): Promise<void> {
    const scene: ScenePreset = {
      id: `user-scene-${Date.now()}`,
      name,
      mood,
      description: `Custom mix: ${this.config.visual.label} + ${this.currentEqName()}`,
      builtin: false,
      visual: { ...this.config.visual },
      visualFx: { ...this.config.visualFx },
      eqPresetId: this.config.eqPresetId,
      eqBands: [...this.config.eqBands] as [number, number, number, number, number],
      tapeCutoffHz: this.config.tapeCutoffHz,
      ambience: { ...this.config.ambience },
      audioSource: { ...this.config.audioSource },
      accentColor: "#7dd3fc",
    };
    this.scenes = await backend.saveScene(scene);
    this.config.activeSceneId = scene.id;
    this.scheduleConfigSave();
    this.refreshAll();
    this.toast(`Saved scene "${name}"`);
  }

  private async deleteScene(sceneId: string): Promise<void> {
    this.scenes = await backend.deleteScene(sceneId);
    this.refreshAll();
    this.toast("Deleted scene");
  }

  private async togglePlugin(pluginId: string, enabled: boolean): Promise<void> {
    this.plugins = await backend.setPluginEnabled(pluginId, enabled);
    this.refreshAll();
    this.toast(`${enabled ? "Enabled" : "Disabled"} plugin ${pluginId}`);
  }

  private async reloadPlugins(): Promise<void> {
    this.plugins = await backend.reloadPlugins();
    this.refreshAll();
    this.toast(`Loaded ${this.plugins.plugins.length} plugins`);
  }

  private async installWasmPlugin(): Promise<void> {
    const files = await backend.pickFiles("Select WebAssembly plugin", ["wasm", "wat"]);
    if (files.length === 0) return;
    try {
      this.plugins = await backend.installWasmPlugin(files[0]);
      this.refreshAll();
      this.toast("Installed & loaded WebAssembly plugin");
    } catch (err) {
      this.toast(String(err));
    }
  }

  private async runPluginEqTransform(pluginId: string, intensity: number): Promise<void> {
    try {
      const res = await backend.runPluginEqTransform(pluginId, this.config.eqBands, intensity);
      this.config.eqBands = res.outputBands;
      this.config.tapeCutoffHz = res.suggestedTapeCutoffHz;
      this.config.eqPresetId = `dsp:${pluginId}`;
      this.audio.setEqBands(this.config.eqBands);
      this.audio.setTapeCutoff(this.config.tapeCutoffHz);
      this.scheduleConfigSave();
      this.refreshAll();
      this.toast(`${res.pluginName}: ${res.note}`);
    } catch (err) {
      this.toast(String(err));
    }
  }

  // --- History & Share ---
  private async replayHistory(entry: HistoryEntry): Promise<void> {
    if (entry.kind === "station") {
      await this.selectStation(entry.refId, false);
    } else if (entry.kind === "track") {
      const track = this.library.music.find((m) => m.id === entry.refId);
      if (track) await this.playLocalTrack(track, false);
      else this.toast("Local file is no longer in library");
    } else if (entry.kind === "scene") {
      const sc = this.scenes.find((s) => s.id === entry.refId);
      if (sc) await this.applyScene(sc);
      else {
        const ps = this.plugins.sceneContributions.find((s) => s.id === entry.refId);
        if (ps) await this.applyPluginScene(ps);
      }
    }
  }

  private async clearHistory(): Promise<void> {
    await backend.clearHistory();
    this.history = [];
    this.refreshPopover();
    this.toast("History cleared");
  }

  private async copyShareCode(): Promise<void> {
    const area = this.popoverHost.querySelector<HTMLTextAreaElement>("[data-share-out]");
    if (!area) return;
    try {
      await navigator.clipboard.writeText(area.value);
      this.toast("Scene code copied to clipboard");
    } catch {
      area.select();
      document.execCommand("copy");
      this.toast("Scene code copied");
    }
  }

  private async importShareCode(raw: string): Promise<void> {
    try {
      const jsonStr = raw.startsWith("lofi-scene-v1:")
        ? decodeURIComponent(escape(atob(raw.slice("lofi-scene-v1:".length))))
        : raw;
      const parsed = JSON.parse(jsonStr) as Partial<ScenePreset>;
      if (parsed.visual) this.config.visual = parsed.visual;
      if (parsed.visualFx) this.config.visualFx = { ...this.config.visualFx, ...parsed.visualFx };
      if (parsed.eqBands) this.config.eqBands = parsed.eqBands;
      if (parsed.tapeCutoffHz) this.config.tapeCutoffHz = parsed.tapeCutoffHz;
      if (parsed.ambience) this.config.ambience = parsed.ambience;
      if (parsed.audioSource) this.config.audioSource = parsed.audioSource;

      this.applyVisualFromConfig();
      this.audio.setEqBands(this.config.eqBands);
      this.audio.setTapeCutoff(this.config.tapeCutoffHz);
      this.audio.setAmbienceMap(this.config.ambience);
      this.scheduleConfigSave();
      this.refreshAll();
      this.toast("Imported atmosphere preset");
    } catch {
      this.toast("Invalid scene code or JSON");
    }
  }

  private exportSceneFile(): void {
    const data = JSON.stringify(
      {
        name: this.config.visual.label,
        visual: this.config.visual,
        visualFx: this.config.visualFx,
        eqBands: this.config.eqBands,
        tapeCutoffHz: this.config.tapeCutoffHz,
        ambience: this.config.ambience,
        audioSource: this.config.audioSource,
      },
      null,
      2
    );
    const blob = new Blob([data], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "atmosphere.lofi-scene.json";
    a.click();
    URL.revokeObjectURL(url);
    this.toast("Exported atmosphere.lofi-scene.json");
  }

  // --- Popovers, HUD & Timers ---
  private toggleModule(mod: ModuleId): void {
    if (this.popovers.getActiveModule() === mod) {
      this.closePopover();
    } else {
      this.openModule(mod);
    }
  }

  private openModule(mod: ModuleId): void {
    if (this.hudHidden) this.toggleHud();
    this.popovers.open(mod, this.popoverCtx());
    this.refreshDock();
  }

  private closePopover(): void {
    this.popovers.close();
    this.refreshDock();
  }

  private toggleHud(): void {
    this.hudHidden = !this.hudHidden;
    this.appRoot.classList.toggle("hud-hidden", this.hudHidden);
    this.restoreBtn.classList.toggle("hidden", !this.hudHidden);
    if (this.hudHidden) this.closePopover();
  }

  private startTimerLoop(): void {
    window.setInterval(() => {
      let changed = false;

      if (this.timers.focusRunning) {
        this.timers.focusRemainingSec = Math.max(0, this.timers.focusRemainingSec - 1);
        changed = true;
        if (this.timers.focusRemainingSec === 0) {
          this.timers.focusRunning = false;
          this.audio.playChime();
          if (this.timers.focusMode === "focus") {
            this.config.completedFocusSessions += 1;
            this.timers.focusMode = "break";
            this.timers.focusRemainingSec = (this.config.breakMinutes || 5) * 60;
            this.scheduleConfigSave();
            void backend.notifyFocusComplete(this.config.completedFocusSessions);
            this.toast("Focus session complete! Time for a short break.");
          } else {
            this.timers.focusMode = "focus";
            this.timers.focusRemainingSec = (this.config.focusMinutes || 25) * 60;
            this.toast("Break complete! Ready for the next focus block.");
          }
          this.refreshAll();
        }
      }

      if (this.timers.sleepRunning) {
        this.timers.sleepRemainingSec = Math.max(0, this.timers.sleepRemainingSec - 1);
        changed = true;
        // Smooth linear fade-out during the final 30 seconds
        if (this.timers.sleepRemainingSec <= 30) {
          this.audio.setSleepFade(this.timers.sleepRemainingSec / 30);
        }
        if (this.timers.sleepRemainingSec === 0) {
          this.timers.sleepRunning = false;
          this.audio.pause();
          this.audio.setSleepFade(1);
          this.refreshAll();
          this.toast("Sleep timer finished — playback paused");
        }
      }

      if (changed) {
        this.refreshTopHud();
        this.popovers.updateLiveTick(this.popoverCtx());
      }
    }, 1000);
  }

  private bindGlobalEvents(): void {
    this.restoreBtn.addEventListener("click", () => this.toggleHud());

    // Double-click background stage toggles Cinema Mode
    document.getElementById("stage")?.addEventListener("dblclick", () => this.toggleHud());

    // Click outside popover closes it
    document.addEventListener("mousedown", (e) => {
      if (!this.popovers.getActiveModule()) return;
      const target = e.target as Node;
      if (!this.popoverHost.contains(target) && !this.dockEl.contains(target)) {
        this.closePopover();
      }
    });

    // Global keyboard shortcuts
    document.addEventListener("keydown", (e) => {
      const tag = (e.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
        if (e.key === "Escape") this.closePopover();
        return;
      }

      if (e.code === "Space") {
        e.preventDefault();
        void this.togglePlay();
      } else if (e.key === "Escape") {
        if (this.popovers.getActiveModule()) this.closePopover();
        else if (this.hudHidden) this.toggleHud();
      } else if (e.key === "m" || e.key === "M") {
        this.toggleMute();
      } else if (e.key === "h" || e.key === "H") {
        this.toggleHud();
      } else if (e.key === "f" || e.key === "F") {
        void backend.toggleFullscreen();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        this.setVolume(this.config.volume + 0.05);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        this.setVolume(this.config.volume - 0.05);
      } else if (e.key === "n" || e.key === "N") {
        void this.stepTrack(1);
      } else if (e.key === "p" || e.key === "P") {
        void this.stepTrack(-1);
      } else {
        const mod = DOCK_MODULES.find((m) => m.shortcut === e.key);
        if (mod) this.toggleModule(mod.id);
      }
    });

    // Drag & Drop support (Browser + Tauri)
    this.setupDragAndDrop();
  }

  private setupDragAndDrop(): void {
    let dragDepth = 0;
    window.addEventListener("dragenter", (e) => {
      e.preventDefault();
      dragDepth++;
      this.dropOverlay.classList.remove("hidden");
    });
    window.addEventListener("dragover", (e) => e.preventDefault());
    window.addEventListener("dragleave", () => {
      dragDepth = Math.max(0, dragDepth - 1);
      if (dragDepth === 0) this.dropOverlay.classList.add("hidden");
    });
    window.addEventListener("drop", async (e) => {
      e.preventDefault();
      dragDepth = 0;
      this.dropOverlay.classList.add("hidden");

      if (isTauriRuntime()) return; // Handled via Tauri native drag-drop event below
      const files = Array.from(e.dataTransfer?.files || []);
      let added = 0;
      for (const file of files) {
        const item = backend.registerBrowserFile(file);
        if (item) {
          added++;
          if (item.kind === "image" || item.kind === "video") {
            this.config.visual = {
              kind: item.kind,
              id: item.filePath,
              label: item.title,
              streamUrl: item.streamUrl,
            };
            this.visual.setSource(this.config.visual);
          } else if (item.kind === "audio") {
            await this.playLocalTrack(item, true);
          }
        }
      }
      if (added > 0) {
        this.library = await backend.scanLibrary();
        this.refreshAll();
        this.toast(`Loaded ${added} dropped file(s)`);
      }
    });

    if (isTauriRuntime()) {
      void import("@tauri-apps/api/webview").then(({ getCurrentWebview }) => {
        void getCurrentWebview().onDragDropEvent(async (event) => {
          if (event.payload.type === "over") {
            this.dropOverlay.classList.remove("hidden");
          } else if (event.payload.type === "leave") {
            this.dropOverlay.classList.add("hidden");
          } else if (event.payload.type === "drop") {
            this.dropOverlay.classList.add("hidden");
            const paths = event.payload.paths;
            let loadedCount = 0;
            for (const p of paths) {
              const lower = p.toLowerCase();
              if (lower.endsWith(".wasm") || lower.endsWith(".wat")) {
                try {
                  this.plugins = await backend.installWasmPlugin(p);
                  loadedCount++;
                } catch (err) {
                  this.toast(String(err));
                }
              } else {
                try {
                  const item = await backend.registerLocalFile(p);
                  loadedCount++;
                  if (item.kind === "image" || item.kind === "video") {
                    this.config.visual = {
                      kind: item.kind,
                      id: item.filePath,
                      label: item.title,
                      streamUrl: item.streamUrl,
                    };
                    this.visual.setSource(this.config.visual);
                    this.scheduleConfigSave();
                  } else if (item.kind === "audio") {
                    await this.playLocalTrack(item, true);
                  }
                } catch (err) {
                  this.toast(String(err));
                }
              }
            }
            if (loadedCount > 0) {
              this.library = await backend.scanLibrary();
              this.refreshAll();
              this.toast(`Loaded ${loadedCount} local file(s)`);
            }
          }
        });
      });
    }
  }

  private scheduleConfigSave(): void {
    if (this.saveDebounce !== null) clearTimeout(this.saveDebounce);
    this.saveDebounce = window.setTimeout(() => {
      this.saveDebounce = null;
      void backend.saveConfig(this.config);
    }, 220);
  }

  private toast(message: string): void {
    const el = document.createElement("div");
    el.className = "toast";
    el.textContent = message;
    this.toastHost.appendChild(el);
    window.setTimeout(() => el.remove(), 3200);
  }
}

const app = new LofiApp();
void app.init();
