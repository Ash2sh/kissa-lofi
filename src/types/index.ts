export interface LibraryDirectories {
  appDataDir: string;
  defaultMusicDir: string;
  defaultVisualsDir: string;
  defaultSoundsDir: string;
  defaultPluginsDir: string;
  customMusicDirs: string[];
  customVisualDirs: string[];
  customSoundDirs: string[];
  customPluginDirs: string[];
}

export type MediaKind = "audio" | "image" | "video" | "ambient";

export interface MediaItem {
  id: string;
  title: string;
  subtitle: string;
  filePath: string;
  streamUrl: string;
  kind: MediaKind;
  extension: string;
  sizeBytes: number;
  modifiedUnix: number;
}

export interface LibrarySnapshot {
  music: MediaItem[];
  visuals: MediaItem[];
  ambientSounds: MediaItem[];
  directories: LibraryDirectories;
}

export interface EqPreset {
  id: string;
  name: string;
  bands: [number, number, number, number, number];
  tapeCutoffHz: number;
  source: string;
}

export interface VisualFxSettings {
  rainOverlay: number;
  vignette: number;
  filmGrain: number;
  dim: number;
  warmTint: number;
  blur: number;
  fpsCap: number;
  pauseWhenHidden: boolean;
}

export interface VisualSource {
  kind: "canvas" | "image" | "video";
  id: string;
  label: string;
  streamUrl?: string | null;
}

export interface AudioSourceRef {
  kind: "station" | "local" | "stream";
  id: string;
  label: string;
}

export interface ScenePreset {
  id: string;
  name: string;
  mood: string;
  description: string;
  builtin: boolean;
  visual: VisualSource;
  visualFx: VisualFxSettings;
  eqPresetId: string;
  eqBands: [number, number, number, number, number];
  tapeCutoffHz: number;
  ambience: Record<string, number>;
  audioSource: AudioSourceRef;
  accentColor: string;
}

export interface HistoryEntry {
  id: string;
  title: string;
  subtitle: string;
  kind: "track" | "station" | "scene";
  refId: string;
  timestampUnix: number;
}

export interface FocusTask {
  id: string;
  text: string;
  done: boolean;
}

export interface CustomStream {
  id: string;
  name: string;
  mood: string;
  url: string;
}

export interface AppConfig {
  volume: number;
  muted: boolean;
  activeSceneId: string;
  visual: VisualSource;
  visualFx: VisualFxSettings;
  eqPresetId: string;
  eqBands: [number, number, number, number, number];
  tapeCutoffHz: number;
  ambience: Record<string, number>;
  audioSource: AudioSourceRef;
  shuffle: boolean;
  repeatMode: "all" | "one" | "off";
  customMusicDirs: string[];
  customVisualDirs: string[];
  customSoundDirs: string[];
  customPluginDirs: string[];
  allowedFiles: string[];
  customEqPresets: EqPreset[];
  customStreams: CustomStream[];
  focusMinutes: number;
  breakMinutes: number;
  completedFocusSessions: number;
  focusTasks: FocusTask[];
  disabledPlugins: string[];
  hudAutoHide: boolean;
}

export type PluginKind = "native" | "wasm";

export interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  kind: PluginKind;
  filePath?: string | null;
  capabilities: string[];
  enabled: boolean;
}

export interface PluginSceneContribution {
  id: string;
  name: string;
  mood: string;
  description: string;
  visualId: string;
  eqBands: [number, number, number, number, number];
  tapeCutoffHz: number;
  ambience: Record<string, number>;
  accentColor: string;
  pluginId: string;
}

export interface PluginEqTransformResult {
  pluginId: string;
  pluginName: string;
  inputBands: [number, number, number, number, number];
  outputBands: [number, number, number, number, number];
  suggestedTapeCutoffHz: number;
  note: string;
}

export interface PluginRegistrySnapshot {
  plugins: PluginMetadata[];
  eqPresets: EqPreset[];
  sceneContributions: PluginSceneContribution[];
  loadErrors: string[];
}

export interface SystemMetrics {
  uptimeSecs: number;
  loadedPlugins: number;
  wasmPlugins: number;
  indexedTracks: number;
  indexedVisuals: number;
}

export type ModuleId =
  | "eq"
  | "music"
  | "visuals"
  | "sounds"
  | "scenes"
  | "history"
  | "share"
  | "keys"
  | "focus"
  | "sleep";

export interface LofiStation {
  id: string;
  name: string;
  mood: string;
  listeners: number;
  bpm: number;
  keyRootMidi: number;
  description: string;
  accent: string;
  chords: number[][];
}

export interface AmbientChannelDef {
  id: string;
  name: string;
  category: string;
  description: string;
}

export interface CanvasSceneDef {
  id: string;
  name: string;
  subtitle: string;
  previewGradient: string;
}
