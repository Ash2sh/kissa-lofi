import type { ModuleId } from "../types";
import { icons } from "./icons";

export interface DockModuleDef {
  id: ModuleId;
  label: string;
  icon: string;
  shortcut: string;
}

export const DOCK_MODULES: DockModuleDef[] = [
  { id: "eq", label: "EQ", icon: icons.eq, shortcut: "1" },
  { id: "music", label: "Music", icon: icons.music, shortcut: "2" },
  { id: "visuals", label: "Visuals", icon: icons.visuals, shortcut: "3" },
  { id: "sounds", label: "Sounds", icon: icons.sounds, shortcut: "4" },
  { id: "scenes", label: "Scenes", icon: icons.scenes, shortcut: "5" },
  { id: "history", label: "History", icon: icons.history, shortcut: "6" },
  { id: "share", label: "Share", icon: icons.share, shortcut: "7" },
  { id: "keys", label: "Keys", icon: icons.keys, shortcut: "8" },
  { id: "focus", label: "Focus", icon: icons.focus, shortcut: "9" },
  { id: "sleep", label: "Sleep", icon: icons.sleep, shortcut: "0" },
];

export interface DockViewState {
  mood: string;
  title: string;
  subtitle: string;
  statusLine: string;
  playing: boolean;
  volume: number;
  muted: boolean;
  activeModule: ModuleId | null;
  scenesDot: boolean;
  soundsActive: boolean;
  focusActive: boolean;
  sleepActive: boolean;
}

export interface DockCallbacks {
  onTogglePlay: () => void;
  onToggleMute: () => void;
  onVolumeChange: (vol: number) => void;
  onToggleModule: (mod: ModuleId) => void;
  onTrackClick: () => void;
}

export class DockController {
  private root: HTMLElement;
  private callbacks: DockCallbacks;
  private initialized = false;

  constructor(root: HTMLElement, callbacks: DockCallbacks) {
    this.root = root;
    this.callbacks = callbacks;
  }

  public render(state: DockViewState): void {
    if (!this.initialized) {
      this.mount();
      this.initialized = true;
    }

    const thumb = this.root.querySelector<HTMLElement>("[data-dock-thumb]");
    const moodEl = this.root.querySelector<HTMLElement>("[data-dock-mood]");
    const titleEl = this.root.querySelector<HTMLElement>("[data-dock-title]");
    const subEl = this.root.querySelector<HTMLElement>("[data-dock-sub]");
    const statusEl = this.root.querySelector<HTMLElement>("[data-dock-status]");
    const playBtn = this.root.querySelector<HTMLButtonElement>("[data-dock-play]");
    const muteBtn = this.root.querySelector<HTMLButtonElement>("[data-dock-mute]");
    const volSlider = this.root.querySelector<HTMLInputElement>("[data-dock-volume]");

    if (thumb) thumb.classList.toggle("playing", state.playing);
    if (moodEl) moodEl.textContent = state.mood;
    if (titleEl) {
      titleEl.textContent = state.title;
      titleEl.title = state.title;
    }
    if (subEl) subEl.textContent = state.subtitle;
    if (statusEl) {
      statusEl.textContent = state.statusLine;
      statusEl.title = state.statusLine;
    }

    if (playBtn) {
      playBtn.innerHTML = state.playing ? icons.pause : icons.play;
      playBtn.title = state.playing ? "Pause (Space)" : "Play (Space)";
      playBtn.setAttribute("aria-label", state.playing ? "Pause" : "Play");
    }

    if (muteBtn) {
      const icon =
        state.muted || state.volume <= 0.005
          ? icons.volumeMute
          : state.volume < 0.45
          ? icons.volumeLow
          : icons.volumeHigh;
      muteBtn.innerHTML = icon;
      muteBtn.title = state.muted ? "Unmute (M)" : "Mute (M)";
    }

    if (volSlider) {
      const pct = Math.round((state.muted ? 0 : state.volume) * 100);
      volSlider.value = String(pct);
      volSlider.style.setProperty("--val", `${pct}%`);
    }

    const modButtons = this.root.querySelectorAll<HTMLButtonElement>("[data-module-id]");
    modButtons.forEach((btn) => {
      const id = btn.dataset.moduleId as ModuleId;
      btn.classList.toggle("active", state.activeModule === id);

      let badge = btn.querySelector<HTMLElement>(".mod-badge");
      const showGreen = id === "scenes" && state.scenesDot;
      const showBlue =
        (id === "sounds" && state.soundsActive) ||
        (id === "focus" && state.focusActive) ||
        (id === "sleep" && state.sleepActive);

      if (showGreen || showBlue) {
        if (!badge) {
          badge = document.createElement("span");
          badge.className = "mod-badge";
          btn.appendChild(badge);
        }
        badge.classList.toggle("mod-badge--blue", showBlue && !showGreen);
      } else if (badge) {
        badge.remove();
      }
    });
  }

  private mount(): void {
    this.root.innerHTML = `
      <div class="dock-track" data-dock-track title="Open Music library">
        <div class="dock-thumb" data-dock-thumb>
          ${icons.musicNoteSolid}
        </div>
        <div class="dock-meta">
          <div class="dock-mood">
            ${icons.broadcast}
            <span data-dock-mood>CHILLING</span>
          </div>
          <div class="dock-title" data-dock-title>focus-zone-relax-mellow</div>
          <div class="dock-sub">
            ${icons.users}
            <span data-dock-sub>8 listening</span>
          </div>
          <div class="dock-status" data-dock-status>Flat EQ · No ambience</div>
        </div>
      </div>

      <div class="dock-transport">
        <button type="button" class="play-btn" data-dock-play aria-label="Play">
          ${icons.play}
        </button>
        <div class="volume-group">
          <button type="button" class="icon-btn" data-dock-mute aria-label="Toggle Mute">
            ${icons.volumeHigh}
          </button>
          <input
            type="range"
            class="volume-slider"
            data-dock-volume
            min="0"
            max="100"
            value="72"
            aria-label="Master Volume"
          />
        </div>
      </div>

      <nav class="dock-modules" aria-label="Modules">
        ${DOCK_MODULES.map(
          (m) => `
          <button
            type="button"
            class="mod-btn"
            data-module-id="${m.id}"
            title="${m.label} (${m.shortcut})"
          >
            ${m.icon}
            <span>${m.label}</span>
          </button>
        `
        ).join("")}
      </nav>
    `;

    this.root
      .querySelector("[data-dock-track]")
      ?.addEventListener("click", () => this.callbacks.onTrackClick());
    this.root
      .querySelector("[data-dock-play]")
      ?.addEventListener("click", () => this.callbacks.onTogglePlay());
    this.root
      .querySelector("[data-dock-mute]")
      ?.addEventListener("click", () => this.callbacks.onToggleMute());

    const volSlider = this.root.querySelector<HTMLInputElement>("[data-dock-volume]");
    volSlider?.addEventListener("input", () => {
      const val = Number(volSlider.value) / 100;
      volSlider.style.setProperty("--val", `${volSlider.value}%`);
      this.callbacks.onVolumeChange(val);
    });

    this.root.querySelectorAll<HTMLButtonElement>("[data-module-id]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const id = btn.dataset.moduleId as ModuleId;
        this.callbacks.onToggleModule(id);
      });
    });
  }
}
