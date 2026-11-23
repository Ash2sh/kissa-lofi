import {
  AMBIENT_CHANNELS,
  BUILTIN_EQ_PRESETS,
  BUILTIN_STATIONS,
  EQ_BAND_LABELS,
} from "../services/audioEngine";
import { BUILTIN_CANVAS_SCENES } from "../services/visualEngine";
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
  VisualFxSettings,
  VisualSource,
} from "../types";
import { icons } from "./icons";

export interface TimerViewState {
  focusRunning: boolean;
  focusMode: "focus" | "break";
  focusRemainingSec: number;
  sleepRunning: boolean;
  sleepRemainingSec: number;
  sleepTotalSec: number;
}

export interface PlaybackProgress {
  currentTime: number;
  duration: number;
}

export interface PopoverContext {
  config: AppConfig;
  library: LibrarySnapshot;
  scenes: ScenePreset[];
  history: HistoryEntry[];
  plugins: PluginRegistrySnapshot;
  timers: TimerViewState;
  progress: PlaybackProgress;
  isTauri: boolean;
}

export interface PopoverActions {
  onClose: () => void;
  // EQ
  onSetEqBand: (index: number, db: number) => void;
  onSetTapeCutoff: (hz: number) => void;
  onApplyEqPreset: (preset: EqPreset) => void;
  onResetEq: () => void;
  onSaveCustomEqPreset: (name: string) => void;
  onRunPluginEqTransform: (pluginId: string, intensity: number) => void;
  // Music
  onSelectStation: (stationId: string) => void;
  onPlayLocalTrack: (item: MediaItem) => void;
  onPlayStream: (id: string, name: string, mood: string, url: string) => void;
  onAddCustomStream: (name: string, mood: string, url: string) => void;
  onRemoveCustomStream: (id: string) => void;
  onNextTrack: () => void;
  onPrevTrack: () => void;
  onToggleShuffle: () => void;
  onCycleRepeat: () => void;
  onSeek: (sec: number) => void;
  // Local Files & Folders
  onAddDirectory: (category: "music" | "visuals" | "sounds" | "plugins") => void;
  onRemoveDirectory: (category: "music" | "visuals" | "sounds" | "plugins", path: string) => void;
  onPickLocalFiles: (category: "music" | "visuals" | "sounds") => void;
  onRescanLibrary: () => void;
  // Visuals
  onSelectVisual: (source: VisualSource) => void;
  onUpdateVisualFx: (partial: Partial<VisualFxSettings>) => void;
  // Sounds
  onSetAmbience: (id: string, level: number, customUrl?: string) => void;
  onClearAmbience: () => void;
  // Scenes & Plugins
  onApplyScene: (scene: ScenePreset) => void;
  onApplyPluginScene: (scene: PluginSceneContribution) => void;
  onSaveCurrentScene: (name: string, mood: string) => void;
  onDeleteScene: (sceneId: string) => void;
  onTogglePlugin: (pluginId: string, enabled: boolean) => void;
  onReloadPlugins: () => void;
  onInstallWasmPlugin: () => void;
  // History
  onReplayHistory: (entry: HistoryEntry) => void;
  onClearHistory: () => void;
  // Share
  onCopyShareCode: () => void;
  onImportShareCode: (raw: string) => void;
  onExportSceneFile: () => void;
  // Keys / View
  onToggleHud: () => void;
  onToggleFullscreen: () => void;
  // Focus
  onStartPauseFocus: () => void;
  onResetFocus: (mode?: "focus" | "break", minutes?: number) => void;
  onAddFocusTask: (text: string) => void;
  onToggleFocusTask: (id: string) => void;
  onDeleteFocusTask: (id: string) => void;
  // Sleep
  onStartSleep: (minutes: number) => void;
  onCancelSleep: () => void;
}

function esc(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function formatClock(totalSec: number): string {
  const s = Math.max(0, Math.floor(totalSec));
  const m = Math.floor(s / 60);
  const rem = s % 60;
  return `${String(m).padStart(2, "0")}:${String(rem).padStart(2, "0")}`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatRelativeTime(unixSec: number): string {
  const diff = Math.max(0, Math.floor(Date.now() / 1000) - unixSec);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return `${Math.floor(diff / 86400)}d ago`;
}

export class PopoverController {
  private host: HTMLElement;
  private actions: PopoverActions;
  private activeModule: ModuleId | null = null;
  private subTabs: Record<string, string> = {
    music: "stations",
    visuals: "builtin",
    scenes: "scenes",
  };
  private musicSearch = "";
  private wasmIntensity = 0.75;

  constructor(host: HTMLElement, actions: PopoverActions) {
    this.host = host;
    this.actions = actions;
  }

  public getActiveModule(): ModuleId | null {
    return this.activeModule;
  }

  public open(moduleId: ModuleId, ctx: PopoverContext): void {
    this.activeModule = moduleId;
    this.host.classList.remove("hidden");
    this.render(ctx);
  }

  public close(): void {
    this.activeModule = null;
    this.host.classList.add("hidden");
    this.host.innerHTML = "";
  }

  public render(ctx: PopoverContext): void {
    if (!this.activeModule) return;

    const mod = this.activeModule;
    switch (mod) {
      case "eq":
        this.renderEq(ctx);
        break;
      case "music":
        this.renderMusic(ctx);
        break;
      case "visuals":
        this.renderVisuals(ctx);
        break;
      case "sounds":
        this.renderSounds(ctx);
        break;
      case "scenes":
        this.renderScenes(ctx);
        break;
      case "history":
        this.renderHistory(ctx);
        break;
      case "share":
        this.renderShare(ctx);
        break;
      case "keys":
        this.renderKeys();
        break;
      case "focus":
        this.renderFocus(ctx);
        break;
      case "sleep":
        this.renderSleep(ctx);
        break;
    }
  }

  /** Lightweight tick update for live timers/seekbar without destroying inputs */
  public updateLiveTick(ctx: PopoverContext): void {
    if (!this.activeModule) return;
    if (this.activeModule === "focus") {
      const clock = this.host.querySelector<HTMLElement>("[data-focus-clock]");
      if (clock) clock.textContent = formatClock(ctx.timers.focusRemainingSec);
    } else if (this.activeModule === "sleep") {
      const clock = this.host.querySelector<HTMLElement>("[data-sleep-clock]");
      if (clock) {
        clock.textContent = ctx.timers.sleepRunning
          ? formatClock(ctx.timers.sleepRemainingSec)
          : "Off";
      }
    } else if (this.activeModule === "music") {
      const cur = this.host.querySelector<HTMLElement>("[data-track-cur]");
      const dur = this.host.querySelector<HTMLElement>("[data-track-dur]");
      const seek = this.host.querySelector<HTMLInputElement>("[data-track-seek]");
      if (cur) cur.textContent = formatClock(ctx.progress.currentTime);
      if (dur) dur.textContent = formatClock(ctx.progress.duration);
      if (seek && ctx.progress.duration > 0) {
        seek.max = String(Math.floor(ctx.progress.duration));
        seek.value = String(Math.floor(ctx.progress.currentTime));
      }
    }
  }

  private wrapPanel(
    icon: string,
    title: string,
    subtitle: string,
    bodyHtml: string,
    headerExtra = ""
  ): void {
    this.host.innerHTML = `
      <div class="panel-header">
        <div class="panel-title-wrap">
          ${icon}
          <div>
            <div class="panel-title">${esc(title)}</div>
            <div class="panel-subtitle">${esc(subtitle)}</div>
          </div>
        </div>
        <div class="panel-header-actions">
          ${headerExtra}
          <button type="button" class="icon-btn" data-close-panel title="Close (Esc)">
            ${icons.close}
          </button>
        </div>
      </div>
      <div class="panel-body">${bodyHtml}</div>
    `;
    this.host
      .querySelector("[data-close-panel]")
      ?.addEventListener("click", () => this.actions.onClose());
  }

  // --- 1. EQ Module ---
  private renderEq(ctx: PopoverContext): void {
    const allPresets: EqPreset[] = [
      ...BUILTIN_EQ_PRESETS,
      ...ctx.plugins.eqPresets,
      ...ctx.config.customEqPresets,
    ];
    const wasmDspPlugins = ctx.plugins.plugins.filter(
      (p) => p.enabled && p.capabilities.includes("eq-dsp")
    );

    const body = `
      <div class="chip-row">
        ${allPresets
          .map(
            (p) => `
          <button
            type="button"
            class="chip ${ctx.config.eqPresetId === p.id ? "active" : ""} ${p.source.startsWith("plugin:wasm") ? "chip--wasm" : ""}"
            data-eq-preset="${esc(p.id)}"
          >${esc(p.name)}</button>
        `
          )
          .join("")}
      </div>

      <div class="eq-sliders-grid">
        ${ctx.config.eqBands
          .map(
            (db, i) => `
          <div class="eq-band">
            <span class="eq-band-db" data-eq-db="${i}">${db >= 0 ? "+" : ""}${db.toFixed(1)}dB</span>
            <div class="eq-vertical-wrap">
              <input
                type="range"
                class="eq-vertical-slider"
                min="-12"
                max="12"
                step="0.5"
                value="${db}"
                data-eq-band="${i}"
              />
            </div>
            <span class="eq-band-freq">${EQ_BAND_LABELS[i]}</span>
          </div>
        `
          )
          .join("")}
      </div>

      <div class="slider-row">
        <span>Tape Low-Pass</span>
        <input
          type="range"
          min="1600"
          max="20000"
          step="100"
          value="${Math.round(ctx.config.tapeCutoffHz)}"
          data-tape-cutoff
        />
        <span class="slider-val" data-tape-val>${(ctx.config.tapeCutoffHz / 1000).toFixed(1)} kHz</span>
      </div>

      ${
        wasmDspPlugins.length > 0
          ? `
        <div class="section-label">Dynamic Wasm &amp; Rust DSP Transforms</div>
        <div class="slider-row">
          <span>DSP Intensity</span>
          <input type="range" min="10" max="100" value="${Math.round(this.wasmIntensity * 100)}" data-wasm-intensity />
          <span class="slider-val" data-wasm-intensity-val>${Math.round(this.wasmIntensity * 100)}%</span>
        </div>
        <div class="chip-row">
          ${wasmDspPlugins
            .map(
              (p) => `
            <button type="button" class="btn btn--sm btn--accent" data-run-wasm-eq="${esc(p.id)}">
              ${icons.cpu}
              <span>Apply ${esc(p.name)} (${p.kind.toUpperCase()})</span>
            </button>
          `
            )
            .join("")}
        </div>
      `
          : ""
      }

      <div class="row-gap">
        <input type="text" class="field-input" placeholder="Save current curve as preset..." data-new-eq-name />
        <button type="button" class="btn" data-save-eq>Save</button>
      </div>
    `;

    this.wrapPanel(
      icons.eq,
      "Equalizer & Tape Filter",
      "5-band parametric Web Audio EQ + Wasm DSP hooks",
      body,
      `<button type="button" class="btn btn--sm" data-reset-eq>Reset Flat</button>`
    );

    this.host.querySelector("[data-reset-eq]")?.addEventListener("click", () => {
      this.actions.onResetEq();
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-eq-preset]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const preset = allPresets.find((p) => p.id === btn.dataset.eqPreset);
        if (preset) this.actions.onApplyEqPreset(preset);
      });
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-eq-band]").forEach((slider) => {
      slider.addEventListener("input", () => {
        const idx = Number(slider.dataset.eqBand);
        const val = Number(slider.value);
        const lbl = this.host.querySelector(`[data-eq-db="${idx}"]`);
        if (lbl) lbl.textContent = `${val >= 0 ? "+" : ""}${val.toFixed(1)}dB`;
        this.actions.onSetEqBand(idx, val);
      });
    });

    const tapeSlider = this.host.querySelector<HTMLInputElement>("[data-tape-cutoff]");
    tapeSlider?.addEventListener("input", () => {
      const hz = Number(tapeSlider.value);
      const valEl = this.host.querySelector("[data-tape-val]");
      if (valEl) valEl.textContent = `${(hz / 1000).toFixed(1)} kHz`;
      this.actions.onSetTapeCutoff(hz);
    });

    const wasmInt = this.host.querySelector<HTMLInputElement>("[data-wasm-intensity]");
    wasmInt?.addEventListener("input", () => {
      this.wasmIntensity = Number(wasmInt.value) / 100;
      const valEl = this.host.querySelector("[data-wasm-intensity-val]");
      if (valEl) valEl.textContent = `${wasmInt.value}%`;
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-run-wasm-eq]").forEach((btn) => {
      btn.addEventListener("click", () => {
        if (btn.dataset.runWasmEq) {
          this.actions.onRunPluginEqTransform(btn.dataset.runWasmEq, this.wasmIntensity);
        }
      });
    });

    this.host.querySelector("[data-save-eq]")?.addEventListener("click", () => {
      const input = this.host.querySelector<HTMLInputElement>("[data-new-eq-name]");
      const name = input?.value.trim();
      if (name) {
        this.actions.onSaveCustomEqPreset(name);
      }
    });
  }

  // --- 2. Music Module ---
  private renderMusic(ctx: PopoverContext): void {
    const tab = this.subTabs.music || "stations";
    const filteredLocal = ctx.library.music.filter((m) =>
      m.title.toLowerCase().includes(this.musicSearch.toLowerCase())
    );

    const localSeekHtml =
      ctx.config.audioSource.kind === "local"
        ? `
        <div class="list-row">
          <div class="row-gap">
            <button type="button" class="icon-btn" data-prev-track title="Previous (P)">${icons.prev}</button>
            <button type="button" class="icon-btn" data-next-track title="Next (N)">${icons.next}</button>
            <button type="button" class="icon-btn ${ctx.config.shuffle ? "active" : ""}" data-toggle-shuffle title="Shuffle">${icons.shuffle}</button>
            <button type="button" class="btn btn--sm" data-cycle-repeat title="Repeat mode">Repeat: ${esc(ctx.config.repeatMode)}</button>
          </div>
          <div class="row-gap" style="flex:1; max-width: 260px;">
            <span class="slider-val" data-track-cur>${formatClock(ctx.progress.currentTime)}</span>
            <input type="range" style="flex:1; accent-color:#7dd3fc;" min="0" max="${Math.max(1, Math.floor(ctx.progress.duration))}" value="${Math.floor(ctx.progress.currentTime)}" data-track-seek />
            <span class="slider-val" data-track-dur>${formatClock(ctx.progress.duration)}</span>
          </div>
        </div>
      `
        : "";

    let tabBody = "";
    if (tab === "stations") {
      tabBody = `
        <div class="card-grid-2">
          ${BUILTIN_STATIONS.map((s) => {
            const active =
              ctx.config.audioSource.kind === "station" && ctx.config.audioSource.id === s.id;
            return `
              <button type="button" class="item-card ${active ? "active" : ""}" data-station-id="${esc(s.id)}">
                <div class="row-between">
                  <span class="badge" style="background:${s.accent}25;color:${s.accent};border:1px solid ${s.accent}55">${esc(s.mood)}</span>
                  <span class="list-row-sub">${s.bpm} BPM</span>
                </div>
                <div class="list-row-title">${esc(s.name)}</div>
                <div class="list-row-sub" style="white-space:normal">${esc(s.description)}</div>
              </button>
            `;
          }).join("")}
        </div>
      `;
    } else if (tab === "local") {
      tabBody = `
        ${localSeekHtml}
        <div class="row-gap">
          <input type="text" class="field-input" placeholder="Search local music library..." value="${esc(this.musicSearch)}" data-music-search />
          <button type="button" class="btn" data-add-music-files>${icons.plus}<span>Files</span></button>
          ${ctx.isTauri ? `<button type="button" class="btn" data-add-music-dir>${icons.folder}<span>Folder</span></button>` : ""}
          <button type="button" class="icon-btn" data-rescan title="Rescan local folders">${icons.refresh}</button>
        </div>

        <div class="path-pill" title="${esc(ctx.library.directories.defaultMusicDir)}">
          Local folder: ${esc(ctx.library.directories.defaultMusicDir)}
        </div>

        ${
          ctx.library.directories.customMusicDirs.length > 0
            ? `<div class="chip-row">
                ${ctx.library.directories.customMusicDirs
                  .map(
                    (d) => `<span class="chip">${esc(d)} <button type="button" class="icon-btn" style="width:16px;height:16px" data-rm-music-dir="${esc(d)}">&times;</button></span>`
                  )
                  .join("")}
              </div>`
            : ""
        }

        ${
          filteredLocal.length === 0
            ? `<div class="empty-state">No local audio tracks indexed yet.<br/>Drop .mp3 / .flac / .wav / .ogg files onto the window or add a local music folder.</div>`
            : `<div class="list-stack">
                ${filteredLocal
                  .map((t) => {
                    const active =
                      ctx.config.audioSource.kind === "local" && ctx.config.audioSource.id === t.id;
                    return `
                      <div class="list-row list-row--clickable ${active ? "active" : ""}" data-local-track="${esc(t.id)}">
                        <div class="list-row-main">
                          <div class="list-row-title">${esc(t.title)}</div>
                          <div class="list-row-sub">${esc(t.subtitle)} · .${esc(t.extension)} · ${formatBytes(t.sizeBytes)}</div>
                        </div>
                        <span class="badge badge--native">${active ? "Playing" : "Play"}</span>
                      </div>
                    `;
                  })
                  .join("")}
              </div>`
        }
      `;
    } else {
      tabBody = `
        <div class="list-stack">
          <div class="row-gap">
            <input type="text" class="field-input" placeholder="Stream name (e.g. Lofi Radio)" data-stream-name />
            <input type="text" class="field-input" placeholder="Audio stream URL (mp3/ogg/aac)" data-stream-url />
            <button type="button" class="btn btn--primary" data-add-stream>Add</button>
          </div>
          ${
            ctx.config.customStreams.length === 0
              ? `<div class="empty-state">Add any direct internet radio or local Icecast audio stream URL.</div>`
              : ctx.config.customStreams
                  .map((st) => {
                    const active =
                      ctx.config.audioSource.kind === "stream" &&
                      ctx.config.audioSource.id === st.id;
                    return `
                      <div class="list-row list-row--clickable ${active ? "active" : ""}" data-play-stream="${esc(st.id)}">
                        <div class="list-row-main">
                          <div class="list-row-title">${esc(st.name)}</div>
                          <div class="list-row-sub">${esc(st.url)}</div>
                        </div>
                        <button type="button" class="icon-btn" data-rm-stream="${esc(st.id)}" title="Remove">${icons.trash}</button>
                      </div>
                    `;
                  })
                  .join("")
          }
        </div>
      `;
    }

    const body = `
      <div class="tabs">
        <button type="button" class="tab-btn ${tab === "stations" ? "active" : ""}" data-music-tab="stations">Lofi Stations</button>
        <button type="button" class="tab-btn ${tab === "local" ? "active" : ""}" data-music-tab="local">Local Library (${ctx.library.music.length})</button>
        <button type="button" class="tab-btn ${tab === "streams" ? "active" : ""}" data-music-tab="streams">Custom Streams</button>
      </div>
      ${tabBody}
    `;

    this.wrapPanel(
      icons.music,
      "Music & Local Library",
      "Continuous procedural lofi beats, local audio folders & streams",
      body
    );

    this.host.querySelectorAll<HTMLButtonElement>("[data-music-tab]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.subTabs.music = btn.dataset.musicTab || "stations";
        this.render(ctx);
      });
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-station-id]").forEach((btn) => {
      btn.addEventListener("click", () => {
        if (btn.dataset.stationId) this.actions.onSelectStation(btn.dataset.stationId);
      });
    });

    this.host.querySelector("[data-music-search]")?.addEventListener("input", (e) => {
      this.musicSearch = (e.target as HTMLInputElement).value;
      this.render(ctx);
      const input = this.host.querySelector<HTMLInputElement>("[data-music-search]");
      if (input) {
        input.focus();
        input.setSelectionRange(input.value.length, input.value.length);
      }
    });

    this.host.querySelector("[data-add-music-files]")?.addEventListener("click", () => {
      this.actions.onPickLocalFiles("music");
    });
    this.host.querySelector("[data-add-music-dir]")?.addEventListener("click", () => {
      this.actions.onAddDirectory("music");
    });
    this.host.querySelector("[data-rescan]")?.addEventListener("click", () => {
      this.actions.onRescanLibrary();
    });
    this.host.querySelectorAll<HTMLElement>("[data-rm-music-dir]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        if (btn.dataset.rmMusicDir) {
          this.actions.onRemoveDirectory("music", btn.dataset.rmMusicDir);
        }
      });
    });

    this.host.querySelectorAll<HTMLElement>("[data-local-track]").forEach((row) => {
      row.addEventListener("click", () => {
        const item = ctx.library.music.find((m) => m.id === row.dataset.localTrack);
        if (item) this.actions.onPlayLocalTrack(item);
      });
    });

    this.host.querySelector("[data-prev-track]")?.addEventListener("click", () => this.actions.onPrevTrack());
    this.host.querySelector("[data-next-track]")?.addEventListener("click", () => this.actions.onNextTrack());
    this.host.querySelector("[data-toggle-shuffle]")?.addEventListener("click", () => this.actions.onToggleShuffle());
    this.host.querySelector("[data-cycle-repeat]")?.addEventListener("click", () => this.actions.onCycleRepeat());
    this.host.querySelector<HTMLInputElement>("[data-track-seek]")?.addEventListener("input", (e) => {
      this.actions.onSeek(Number((e.target as HTMLInputElement).value));
    });

    this.host.querySelector("[data-add-stream]")?.addEventListener("click", () => {
      const name = this.host.querySelector<HTMLInputElement>("[data-stream-name]")?.value.trim() || "Custom Stream";
      const url = this.host.querySelector<HTMLInputElement>("[data-stream-url]")?.value.trim() || "";
      if (url) this.actions.onAddCustomStream(name, "LIVE STREAM", url);
    });
    this.host.querySelectorAll<HTMLElement>("[data-play-stream]").forEach((row) => {
      row.addEventListener("click", () => {
        const st = ctx.config.customStreams.find((s) => s.id === row.dataset.playStream);
        if (st) this.actions.onPlayStream(st.id, st.name, st.mood, st.url);
      });
    });
    this.host.querySelectorAll<HTMLElement>("[data-rm-stream]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        if (btn.dataset.rmStream) this.actions.onRemoveCustomStream(btn.dataset.rmStream);
      });
    });
  }

  // --- 3. Visuals Module ---
  private renderVisuals(ctx: PopoverContext): void {
    const tab = this.subTabs.visuals || "builtin";
    const fx = ctx.config.visualFx;

    let tabBody = "";
    if (tab === "builtin") {
      tabBody = `
        <div class="card-grid-2">
          ${BUILTIN_CANVAS_SCENES.map((sc) => {
            const active = ctx.config.visual.kind === "canvas" && ctx.config.visual.id === sc.id;
            return `
              <button type="button" class="item-card ${active ? "active" : ""}" data-canvas-scene="${esc(sc.id)}">
                <div class="visual-preview-box" style="background:${sc.previewGradient}">
                  <span class="visual-tag">Canvas · Live</span>
                </div>
                <div class="list-row-title">${esc(sc.name)}</div>
                <div class="list-row-sub">${esc(sc.subtitle)}</div>
              </button>
            `;
          }).join("")}
        </div>
      `;
    } else if (tab === "local") {
      tabBody = `
        <div class="row-gap">
          <button type="button" class="btn btn--primary" data-pick-visual>${icons.plus}<span>Pick Photo / Video</span></button>
          ${ctx.isTauri ? `<button type="button" class="btn" data-add-visual-dir>${icons.folder}<span>Add Folder</span></button>` : ""}
          <button type="button" class="icon-btn" data-rescan-visuals title="Rescan">${icons.refresh}</button>
        </div>
        <div class="path-pill" title="${esc(ctx.library.directories.defaultVisualsDir)}">
          Visuals folder: ${esc(ctx.library.directories.defaultVisualsDir)}
        </div>
        ${
          ctx.library.visuals.length === 0
            ? `<div class="empty-state">No local photos or videos yet.<br/>Pick or drag-and-drop any .mp4, .webm, .jpg, .png, or .webp wallpaper onto the window.</div>`
            : `<div class="card-grid-2">
                ${ctx.library.visuals
                  .map((v) => {
                    const active = ctx.config.visual.id === v.filePath;
                    return `
                      <button type="button" class="item-card ${active ? "active" : ""}" data-local-visual="${esc(v.id)}">
                        <div class="visual-preview-box">
                          ${v.kind === "image" ? `<img src="${esc(v.streamUrl)}" alt="" loading="lazy" />` : ""}
                          <span class="visual-tag">${v.kind === "video" ? "Video · Loop" : "Photo"} · .${esc(v.extension)}</span>
                        </div>
                        <div class="list-row-title">${esc(v.title)}</div>
                        <div class="list-row-sub">${esc(v.subtitle)} · ${formatBytes(v.sizeBytes)}</div>
                      </button>
                    `;
                  })
                  .join("")}
              </div>`
        }
      `;
    } else {
      tabBody = `
        <div class="list-stack">
          <div class="slider-row">
            <span>Rain Overlay</span>
            <input type="range" min="0" max="100" value="${Math.round(fx.rainOverlay * 100)}" data-fx="rainOverlay" />
            <span class="slider-val">${Math.round(fx.rainOverlay * 100)}%</span>
          </div>
          <div class="slider-row">
            <span>Vignette</span>
            <input type="range" min="0" max="100" value="${Math.round(fx.vignette * 100)}" data-fx="vignette" />
            <span class="slider-val">${Math.round(fx.vignette * 100)}%</span>
          </div>
          <div class="slider-row">
            <span>Film Grain</span>
            <input type="range" min="0" max="100" value="${Math.round(fx.filmGrain * 100)}" data-fx="filmGrain" />
            <span class="slider-val">${Math.round(fx.filmGrain * 100)}%</span>
          </div>
          <div class="slider-row">
            <span>Dimmer</span>
            <input type="range" min="0" max="75" value="${Math.round(fx.dim * 100)}" data-fx="dim" />
            <span class="slider-val">${Math.round(fx.dim * 100)}%</span>
          </div>
          <div class="slider-row">
            <span>Warm Tint</span>
            <input type="range" min="0" max="100" value="${Math.round(fx.warmTint * 100)}" data-fx="warmTint" />
            <span class="slider-val">${Math.round(fx.warmTint * 100)}%</span>
          </div>
          <div class="slider-row">
            <span>Media Blur</span>
            <input type="range" min="0" max="12" step="0.5" value="${fx.blur}" data-fx-raw="blur" />
            <span class="slider-val">${fx.blur.toFixed(1)}px</span>
          </div>
        </div>

        <div class="row-between" style="padding-top:6px; border-top:1px solid rgba(255,255,255,0.07)">
          <div>
            <div class="list-row-title">Ultra-Low Resource Cap</div>
            <div class="list-row-sub">Limits Canvas render FPS &amp; pauses when window is minimized</div>
          </div>
          <div class="chip-row">
            ${[15, 30, 60]
              .map(
                (fps) =>
                  `<button type="button" class="chip ${fx.fpsCap === fps ? "active" : ""}" data-fps-cap="${fps}">${fps} FPS</button>`
              )
              .join("")}
          </div>
        </div>
      `;
    }

    const body = `
      <div class="tabs">
        <button type="button" class="tab-btn ${tab === "builtin" ? "active" : ""}" data-vis-tab="builtin">Canvas Scenes</button>
        <button type="button" class="tab-btn ${tab === "local" ? "active" : ""}" data-vis-tab="local">Local Photos &amp; Videos (${ctx.library.visuals.length})</button>
        <button type="button" class="tab-btn ${tab === "fx" ? "active" : ""}" data-vis-tab="fx">Atmosphere &amp; Power</button>
      </div>
      ${tabBody}
    `;

    this.wrapPanel(
      icons.visuals,
      "Visual Backgrounds",
      "HTML5 <video> & Canvas rendering engine with local wallpaper support",
      body
    );

    this.host.querySelectorAll<HTMLButtonElement>("[data-vis-tab]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.subTabs.visuals = btn.dataset.visTab || "builtin";
        this.render(ctx);
      });
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-canvas-scene]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const sc = BUILTIN_CANVAS_SCENES.find((s) => s.id === btn.dataset.canvasScene);
        if (sc) {
          this.actions.onSelectVisual({
            kind: "canvas",
            id: sc.id,
            label: sc.name,
            streamUrl: null,
          });
        }
      });
    });

    this.host.querySelector("[data-pick-visual]")?.addEventListener("click", () => {
      this.actions.onPickLocalFiles("visuals");
    });
    this.host.querySelector("[data-add-visual-dir]")?.addEventListener("click", () => {
      this.actions.onAddDirectory("visuals");
    });
    this.host.querySelector("[data-rescan-visuals]")?.addEventListener("click", () => {
      this.actions.onRescanLibrary();
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-local-visual]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const item = ctx.library.visuals.find((v) => v.id === btn.dataset.localVisual);
        if (item) {
          this.actions.onSelectVisual({
            kind: item.kind === "video" ? "video" : "image",
            id: item.filePath,
            label: item.title,
            streamUrl: item.streamUrl,
          });
        }
      });
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-fx]").forEach((slider) => {
      slider.addEventListener("input", () => {
        const key = slider.dataset.fx as keyof VisualFxSettings;
        const val = Number(slider.value) / 100;
        const valEl = slider.nextElementSibling;
        if (valEl) valEl.textContent = `${slider.value}%`;
        this.actions.onUpdateVisualFx({ [key]: val });
      });
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-fx-raw]").forEach((slider) => {
      slider.addEventListener("input", () => {
        const key = slider.dataset.fxRaw as keyof VisualFxSettings;
        const val = Number(slider.value);
        const valEl = slider.nextElementSibling;
        if (valEl) valEl.textContent = `${val.toFixed(1)}px`;
        this.actions.onUpdateVisualFx({ [key]: val });
      });
    });

    this.host.querySelectorAll<HTMLButtonElement>("[data-fps-cap]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.actions.onUpdateVisualFx({ fpsCap: Number(btn.dataset.fpsCap) });
      });
    });
  }

  // --- 4. Sounds (Ambient Mixer) Module ---
  private renderSounds(ctx: PopoverContext): void {
    const activeCount = Object.values(ctx.config.ambience).filter((v) => v > 0.01).length;

    const body = `
      <div class="card-grid-2">
        ${AMBIENT_CHANNELS.map((ch) => {
          const level = ctx.config.ambience[ch.id] ?? 0;
          const active = level > 0.01;
          return `
            <div class="sound-tile ${active ? "active" : ""}">
              <div class="row-between">
                <div>
                  <div class="list-row-title">${esc(ch.name)}</div>
                  <div class="list-row-sub">${esc(ch.description)}</div>
                </div>
                <button type="button" class="chip ${active ? "active" : ""}" data-sound-toggle="${esc(ch.id)}">
                  ${active ? `${Math.round(level * 100)}%` : "Off"}
                </button>
              </div>
              <input
                type="range"
                min="0"
                max="100"
                value="${Math.round(level * 100)}"
                style="width:100%; accent-color:#7dd3fc;"
                data-sound-slider="${esc(ch.id)}"
              />
            </div>
          `;
        }).join("")}
      </div>

      ${
        ctx.library.ambientSounds.length > 0
          ? `
        <div class="section-label">Local Ambient Loops</div>
        <div class="card-grid-2">
          ${ctx.library.ambientSounds
            .map((item) => {
              const level = ctx.config.ambience[item.id] ?? 0;
              const active = level > 0.01;
              return `
                <div class="sound-tile ${active ? "active" : ""}">
                  <div class="row-between">
                    <div class="list-row-title">${esc(item.title)}</div>
                    <span class="slider-val">${Math.round(level * 100)}%</span>
                  </div>
                  <input
                    type="range"
                    min="0"
                    max="100"
                    value="${Math.round(level * 100)}"
                    style="width:100%; accent-color:#7dd3fc;"
                    data-custom-sound-slider="${esc(item.id)}"
                    data-custom-sound-url="${esc(item.streamUrl)}"
                  />
                </div>
              `;
            })
            .join("")}
        </div>
      `
          : ""
      }

      <div class="row-between">
        <div class="row-gap">
          <button type="button" class="btn btn--sm" data-add-sound-file>${icons.plus}<span>Add Ambient Audio</span></button>
          ${ctx.isTauri ? `<button type="button" class="btn btn--sm" data-add-sound-dir>${icons.folder}<span>Add Folder</span></button>` : ""}
        </div>
        ${activeCount > 0 ? `<button type="button" class="btn btn--sm btn--danger" data-clear-ambience>Mute All Ambience</button>` : ""}
      </div>
    `;

    this.wrapPanel(
      icons.sounds,
      "Ambient Soundscapes",
      "Layer procedural atmospheric soundscapes or local audio loops",
      body
    );

    this.host.querySelectorAll<HTMLButtonElement>("[data-sound-toggle]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.dataset.soundToggle!;
        const current = ctx.config.ambience[id] ?? 0;
        this.actions.onSetAmbience(id, current > 0.01 ? 0 : 0.45);
      });
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-sound-slider]").forEach((slider) => {
      slider.addEventListener("input", () => {
        const id = slider.dataset.soundSlider!;
        this.actions.onSetAmbience(id, Number(slider.value) / 100);
      });
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-custom-sound-slider]").forEach((slider) => {
      slider.addEventListener("input", () => {
        const id = slider.dataset.customSoundSlider!;
        const url = slider.dataset.customSoundUrl!;
        this.actions.onSetAmbience(id, Number(slider.value) / 100, url);
      });
    });

    this.host.querySelector("[data-clear-ambience]")?.addEventListener("click", () => {
      this.actions.onClearAmbience();
    });
    this.host.querySelector("[data-add-sound-file]")?.addEventListener("click", () => {
      this.actions.onPickLocalFiles("sounds");
    });
    this.host.querySelector("[data-add-sound-dir]")?.addEventListener("click", () => {
      this.actions.onAddDirectory("sounds");
    });
  }

  // --- 5. Scenes & Wasm Plugins Module ---
  private renderScenes(ctx: PopoverContext): void {
    const tab = this.subTabs.scenes || "scenes";
    const wasmCount = ctx.plugins.plugins.filter((p) => p.kind === "wasm" && p.enabled).length;

    let tabBody = "";
    if (tab === "scenes") {
      tabBody = `
        <div class="card-grid-2">
          ${ctx.scenes
            .map((sc) => {
              const active = ctx.config.activeSceneId === sc.id;
              return `
                <div class="item-card ${active ? "active" : ""}" data-apply-scene="${esc(sc.id)}">
                  <div class="row-between">
                    <span class="badge" style="background:${sc.accentColor}25;color:${sc.accentColor};border:1px solid ${sc.accentColor}55">${esc(sc.mood)}</span>
                    ${
                      !sc.builtin
                        ? `<button type="button" class="icon-btn" style="width:22px;height:22px" data-del-scene="${esc(sc.id)}" title="Delete scene">${icons.trash}</button>`
                        : `<span class="list-row-sub">Built-in</span>`
                    }
                  </div>
                  <div class="list-row-title">${esc(sc.name)}</div>
                  <div class="list-row-sub" style="white-space:normal">${esc(sc.description)}</div>
                </div>
              `;
            })
            .join("")}
        </div>

        ${
          ctx.plugins.sceneContributions.length > 0
            ? `
          <div class="section-label">Plugin-Contributed Scenes (Rust Trait / Wasm)</div>
          <div class="card-grid-2">
            ${ctx.plugins.sceneContributions
              .map(
                (ps) => `
              <button type="button" class="item-card ${ctx.config.activeSceneId === ps.id ? "active" : ""}" data-plugin-scene="${esc(ps.id)}">
                <div class="row-between">
                  <span class="badge badge--wasm">${esc(ps.mood)}</span>
                  <span class="list-row-sub">${esc(ps.pluginId)}</span>
                </div>
                <div class="list-row-title">${esc(ps.name)}</div>
                <div class="list-row-sub" style="white-space:normal">${esc(ps.description)}</div>
              </button>
            `
              )
              .join("")}
          </div>
        `
            : ""
        }

        <div class="row-gap">
          <input type="text" class="field-input" placeholder="Save current mix as a new scene..." data-new-scene-name />
          <input type="text" class="field-input" style="max-width:130px" placeholder="Mood tag" value="CUSTOM" data-new-scene-mood />
          <button type="button" class="btn btn--primary" data-save-scene>Save</button>
        </div>
      `;
    } else {
      tabBody = `
        <div class="row-between">
          <div class="row-gap">
            ${ctx.isTauri ? `<button type="button" class="btn btn--primary" data-install-wasm>${icons.plus}<span>Install .wasm / .wat</span></button>` : ""}
            ${ctx.isTauri ? `<button type="button" class="btn" data-add-plugin-dir>${icons.folder}<span>Plugin Folder</span></button>` : ""}
            <button type="button" class="btn" data-reload-plugins>${icons.refresh}<span>Reload Runtime</span></button>
          </div>
          <span class="badge badge--wasm">${wasmCount} Wasm Active</span>
        </div>

        <div class="path-pill" title="${esc(ctx.library.directories.defaultPluginsDir)}">
          Wasm plugins dir: ${esc(ctx.library.directories.defaultPluginsDir)}
        </div>

        <div class="list-stack">
          ${ctx.plugins.plugins
            .map(
              (p) => `
            <div class="list-row">
              <div class="list-row-main">
                <div class="row-gap">
                  <span class="list-row-title">${esc(p.name)}</span>
                  <span class="badge ${p.kind === "wasm" ? "badge--wasm" : "badge--native"}">${p.kind.toUpperCase()} v${esc(p.version)}</span>
                </div>
                <div class="list-row-sub" style="white-space:normal">${esc(p.description)}</div>
                <div class="list-row-sub">Capabilities: ${esc(p.capabilities.join(", "))}</div>
              </div>
              <div class="row-gap">
                ${
                  p.enabled && p.capabilities.includes("eq-dsp")
                    ? `<button type="button" class="btn btn--sm btn--accent" data-plugin-dsp="${esc(p.id)}">Run DSP</button>`
                    : ""
                }
                <button type="button" class="chip ${p.enabled ? "active" : ""}" data-toggle-plugin="${esc(p.id)}" data-plugin-next="${!p.enabled}">
                  ${p.enabled ? "Enabled" : "Disabled"}
                </button>
              </div>
            </div>
          `
            )
            .join("")}
        </div>

        ${
          ctx.plugins.loadErrors.length > 0
            ? `<div class="empty-state" style="color:#fca5a5; text-align:left">
                Plugin load errors:<br/>${ctx.plugins.loadErrors.map(esc).join("<br/>")}
              </div>`
            : ""
        }
      `;
    }

    const body = `
      <div class="tabs">
        <button type="button" class="tab-btn ${tab === "scenes" ? "active" : ""}" data-sc-tab="scenes">Atmosphere Scenes</button>
        <button type="button" class="tab-btn ${tab === "plugins" ? "active" : ""}" data-sc-tab="plugins">Wasm Plugins (${ctx.plugins.plugins.length})</button>
      </div>
      ${tabBody}
    `;

    this.wrapPanel(
      icons.scenes,
      "Scenes & WebAssembly Plugins",
      "One-click atmospheric presets & dynamic Rust/Wasm plugin runtime",
      body
    );

    this.host.querySelectorAll<HTMLButtonElement>("[data-sc-tab]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.subTabs.scenes = btn.dataset.scTab || "scenes";
        this.render(ctx);
      });
    });

    this.host.querySelectorAll<HTMLElement>("[data-apply-scene]").forEach((card) => {
      card.addEventListener("click", () => {
        const sc = ctx.scenes.find((s) => s.id === card.dataset.applyScene);
        if (sc) this.actions.onApplyScene(sc);
      });
    });

    this.host.querySelectorAll<HTMLElement>("[data-del-scene]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        if (btn.dataset.delScene) this.actions.onDeleteScene(btn.dataset.delScene);
      });
    });

    this.host.querySelectorAll<HTMLElement>("[data-plugin-scene]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const ps = ctx.plugins.sceneContributions.find((s) => s.id === btn.dataset.pluginScene);
        if (ps) this.actions.onApplyPluginScene(ps);
      });
    });

    this.host.querySelector("[data-save-scene]")?.addEventListener("click", () => {
      const name = this.host.querySelector<HTMLInputElement>("[data-new-scene-name]")?.value.trim();
      const mood =
        this.host.querySelector<HTMLInputElement>("[data-new-scene-mood]")?.value.trim() ||
        "CUSTOM";
      if (name) this.actions.onSaveCurrentScene(name, mood.toUpperCase());
    });

    this.host.querySelector("[data-install-wasm]")?.addEventListener("click", () => {
      this.actions.onInstallWasmPlugin();
    });
    this.host.querySelector("[data-add-plugin-dir]")?.addEventListener("click", () => {
      this.actions.onAddDirectory("plugins");
    });
    this.host.querySelector("[data-reload-plugins]")?.addEventListener("click", () => {
      this.actions.onReloadPlugins();
    });
    this.host.querySelectorAll<HTMLElement>("[data-toggle-plugin]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.actions.onTogglePlugin(
          btn.dataset.togglePlugin!,
          btn.dataset.pluginNext === "true"
        );
      });
    });
    this.host.querySelectorAll<HTMLElement>("[data-plugin-dsp]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.actions.onRunPluginEqTransform(btn.dataset.pluginDsp!, 0.75);
      });
    });
  }

  // --- 6. History Module ---
  private renderHistory(ctx: PopoverContext): void {
    const body =
      ctx.history.length === 0
        ? `<div class="empty-state">Your recent stations, local tracks, and applied scenes will appear here locally.</div>`
        : `<div class="list-stack">
            ${ctx.history
              .map(
                (h) => `
              <div class="list-row list-row--clickable" data-hist-id="${esc(h.id)}">
                <div class="list-row-main">
                  <div class="list-row-title">${esc(h.title)}</div>
                  <div class="list-row-sub">${esc(h.kind.toUpperCase())} · ${esc(h.subtitle)}</div>
                </div>
                <span class="list-row-sub">${formatRelativeTime(h.timestampUnix)}</span>
              </div>
            `
              )
              .join("")}
          </div>`;

    this.wrapPanel(
      icons.history,
      "Listening History",
      "Stored privately on your machine",
      body,
      ctx.history.length > 0
        ? `<button type="button" class="btn btn--sm btn--danger" data-clear-history>Clear</button>`
        : ""
    );

    this.host.querySelector("[data-clear-history]")?.addEventListener("click", () => {
      this.actions.onClearHistory();
    });
    this.host.querySelectorAll<HTMLElement>("[data-hist-id]").forEach((row) => {
      row.addEventListener("click", () => {
        const entry = ctx.history.find((h) => h.id === row.dataset.histId);
        if (entry) this.actions.onReplayHistory(entry);
      });
    });
  }

  // --- 7. Share Module ---
  private renderShare(ctx: PopoverContext): void {
    const payload = {
      v: 1,
      name: ctx.config.visual.label,
      visual: ctx.config.visual.kind === "canvas" ? ctx.config.visual : undefined,
      visualFx: ctx.config.visualFx,
      eqPresetId: ctx.config.eqPresetId,
      eqBands: ctx.config.eqBands,
      tapeCutoffHz: ctx.config.tapeCutoffHz,
      ambience: ctx.config.ambience,
      audioSource: ctx.config.audioSource.kind === "station" ? ctx.config.audioSource : undefined,
    };
    const encoded = `lofi-scene-v1:${btoa(unescape(encodeURIComponent(JSON.stringify(payload))))}`;

    const body = `
      <div class="section-label">Export Current Atmosphere</div>
      <textarea class="field-textarea" readonly data-share-out>${esc(encoded)}</textarea>
      <div class="row-gap">
        <button type="button" class="btn btn--primary" data-copy-share>${icons.copy}<span>Copy Scene Code</span></button>
        <button type="button" class="btn" data-export-json>Export .lofi-scene.json</button>
      </div>

      <div class="section-label" style="margin-top:4px">Import Shared Scene Code or JSON</div>
      <textarea class="field-textarea" placeholder="Paste lofi-scene-v1:... or JSON preset here" data-share-in></textarea>
      <div class="row-gap">
        <button type="button" class="btn btn--accent" data-import-share>Apply Imported Scene</button>
      </div>
    `;

    this.wrapPanel(
      icons.share,
      "Share & Import Presets",
      "Local-first portable atmosphere codes (no cloud required)",
      body
    );

    this.host.querySelector("[data-copy-share]")?.addEventListener("click", () => {
      this.actions.onCopyShareCode();
    });
    this.host.querySelector("[data-export-json]")?.addEventListener("click", () => {
      this.actions.onExportSceneFile();
    });
    this.host.querySelector("[data-import-share]")?.addEventListener("click", () => {
      const raw = this.host.querySelector<HTMLTextAreaElement>("[data-share-in]")?.value.trim();
      if (raw) this.actions.onImportShareCode(raw);
    });
  }

  // --- 8. Keys (Shortcuts) Module ---
  private renderKeys(): void {
    const shortcuts = [
      ["Space", "Play / Pause"],
      ["M", "Mute / Unmute"],
      ["↑ / ↓", "Volume Up / Down"],
      ["N / P", "Next / Previous Track"],
      ["1 – 9, 0", "Toggle Dock Modules"],
      ["H", "Hide / Show HUD (Cinema)"],
      ["F", "Toggle Fullscreen"],
      ["Esc", "Close Active Panel"],
    ];

    const body = `
      <div class="keys-grid">
        ${shortcuts
          .map(
            ([k, desc]) => `
          <div class="key-row">
            <span>${esc(desc)}</span>
            <kbd>${esc(k)}</kbd>
          </div>
        `
          )
          .join("")}
      </div>
      <div class="row-gap">
        <button type="button" class="btn" data-key-hud>${icons.eyeOff}<span>Enter Cinema Mode (H)</span></button>
        <button type="button" class="btn" data-key-fs>${icons.fullscreen}<span>Toggle Fullscreen (F)</span></button>
      </div>
    `;

    this.wrapPanel(icons.keys, "Keyboard Shortcuts", "Quick distraction-free control", body);

    this.host.querySelector("[data-key-hud]")?.addEventListener("click", () => this.actions.onToggleHud());
    this.host.querySelector("[data-key-fs]")?.addEventListener("click", () => this.actions.onToggleFullscreen());
  }

  // --- 9. Focus (Pomodoro + Tasks) Module ---
  private renderFocus(ctx: PopoverContext): void {
    const t = ctx.timers;

    const body = `
      <div class="timer-display-card">
        <div class="chip-row">
          <button type="button" class="chip ${t.focusMode === "focus" ? "active" : ""}" data-focus-preset="focus:25">Focus 25m</button>
          <button type="button" class="chip" data-focus-preset="focus:50">Deep 50m</button>
          <button type="button" class="chip ${t.focusMode === "break" ? "active" : ""}" data-focus-preset="break:5">Short Break 5m</button>
          <button type="button" class="chip" data-focus-preset="break:15">Long Break 15m</button>
        </div>
        <div class="timer-clock" data-focus-clock>${formatClock(t.focusRemainingSec)}</div>
        <div class="list-row-sub">
          ${t.focusMode === "focus" ? "Focus session" : "Break time"} · Completed today: ${ctx.config.completedFocusSessions}
        </div>
        <div class="row-gap">
          <button type="button" class="btn btn--primary" data-focus-toggle>
            ${t.focusRunning ? "Pause Timer" : "Start Focus"}
          </button>
          <button type="button" class="btn" data-focus-reset>Reset</button>
        </div>
      </div>

      <div class="section-label">Session Goals</div>
      <div class="row-gap">
        <input type="text" class="field-input" placeholder="Add a focus task..." data-task-input />
        <button type="button" class="btn" data-task-add>Add</button>
      </div>
      <div class="list-stack">
        ${ctx.config.focusTasks
          .map(
            (task) => `
          <div class="list-row">
            <label class="row-gap" style="cursor:pointer; flex:1; min-width:0;">
              <input type="checkbox" ${task.done ? "checked" : ""} data-task-check="${esc(task.id)}" />
              <span class="list-row-title" style="${task.done ? "text-decoration:line-through;opacity:0.5" : ""}">${esc(task.text)}</span>
            </label>
            <button type="button" class="icon-btn" data-task-del="${esc(task.id)}">${icons.trash}</button>
          </div>
        `
          )
          .join("")}
      </div>
    `;

    this.wrapPanel(icons.focus, "Focus Timer", "Pomodoro rhythm with gentle chime & session checklist", body);

    this.host.querySelector("[data-focus-toggle]")?.addEventListener("click", () => {
      this.actions.onStartPauseFocus();
    });
    this.host.querySelector("[data-focus-reset]")?.addEventListener("click", () => {
      this.actions.onResetFocus();
    });
    this.host.querySelectorAll<HTMLButtonElement>("[data-focus-preset]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const [mode, mins] = btn.dataset.focusPreset!.split(":");
        this.actions.onResetFocus(mode as "focus" | "break", Number(mins));
      });
    });

    const addTask = () => {
      const input = this.host.querySelector<HTMLInputElement>("[data-task-input]");
      const text = input?.value.trim();
      if (text) this.actions.onAddFocusTask(text);
    };
    this.host.querySelector("[data-task-add]")?.addEventListener("click", addTask);
    this.host.querySelector<HTMLInputElement>("[data-task-input]")?.addEventListener("keydown", (e) => {
      if (e.key === "Enter") addTask();
    });

    this.host.querySelectorAll<HTMLInputElement>("[data-task-check]").forEach((cb) => {
      cb.addEventListener("change", () => this.actions.onToggleFocusTask(cb.dataset.taskCheck!));
    });
    this.host.querySelectorAll<HTMLButtonElement>("[data-task-del]").forEach((btn) => {
      btn.addEventListener("click", () => this.actions.onDeleteFocusTask(btn.dataset.taskDel!));
    });
  }

  // --- 10. Sleep Timer Module ---
  private renderSleep(ctx: PopoverContext): void {
    const t = ctx.timers;

    const body = `
      <div class="timer-display-card">
        <div class="timer-clock" data-sleep-clock>
          ${t.sleepRunning ? formatClock(t.sleepRemainingSec) : "Off"}
        </div>
        <div class="list-row-sub">
          ${
            t.sleepRunning
              ? "Audio & ambience will smoothly fade out over the final 30 seconds."
              : "Choose a duration to gently fade out playback while you fall asleep."
          }
        </div>
        <div class="chip-row">
          ${[15, 30, 45, 60, 90]
            .map(
              (m) =>
                `<button type="button" class="btn" data-sleep-min="${m}">${m} min</button>`
            )
            .join("")}
        </div>
        <div class="row-gap" style="width:100%; max-width:260px;">
          <input type="number" class="field-input" min="1" max="360" placeholder="Custom minutes" data-sleep-custom />
          <button type="button" class="btn btn--primary" data-sleep-start-custom>Start</button>
          ${t.sleepRunning ? `<button type="button" class="btn btn--danger" data-sleep-cancel>Cancel</button>` : ""}
        </div>
      </div>
    `;

    this.wrapPanel(icons.sleep, "Sleep Timer", "Gradual volume fade-out & automatic pause", body);

    this.host.querySelectorAll<HTMLButtonElement>("[data-sleep-min]").forEach((btn) => {
      btn.addEventListener("click", () => this.actions.onStartSleep(Number(btn.dataset.sleepMin)));
    });
    this.host.querySelector("[data-sleep-start-custom]")?.addEventListener("click", () => {
      const val = Number(this.host.querySelector<HTMLInputElement>("[data-sleep-custom]")?.value);
      if (val >= 1 && val <= 360) this.actions.onStartSleep(val);
    });
    this.host.querySelector("[data-sleep-cancel]")?.addEventListener("click", () => {
      this.actions.onCancelSleep();
    });
  }
}
