import type { AmbientChannelDef, EqPreset, LofiStation } from "../types";

export const EQ_BAND_FREQUENCIES = [60, 230, 910, 3600, 12000] as const;
export const EQ_BAND_LABELS = ["60Hz", "230Hz", "910Hz", "3.6k", "12k"] as const;

export const BUILTIN_EQ_PRESETS: EqPreset[] = [
  {
    id: "flat",
    name: "Flat EQ",
    bands: [0, 0, 0, 0, 0],
    tapeCutoffHz: 18500,
    source: "builtin",
  },
  {
    id: "warm-lofi",
    name: "Warm Lofi",
    bands: [3, 2, -0.5, -2, -3.5],
    tapeCutoffHz: 6800,
    source: "builtin",
  },
  {
    id: "bass-boost",
    name: "Bass Boost",
    bands: [5.5, 3, 0, -0.5, 0.5],
    tapeCutoffHz: 14500,
    source: "builtin",
  },
  {
    id: "late-night",
    name: "Late Night",
    bands: [4, 1.5, -1.5, -3, -4.5],
    tapeCutoffHz: 5200,
    source: "builtin",
  },
  {
    id: "radio-vinyl",
    name: "Radio Vinyl",
    bands: [-2, 2.5, 3, -1, -5],
    tapeCutoffHz: 4800,
    source: "builtin",
  },
  {
    id: "focus-clarity",
    name: "Focus Clarity",
    bands: [1, 0, -1, 2, 1.5],
    tapeCutoffHz: 16000,
    source: "builtin",
  },
];

export const BUILTIN_STATIONS: LofiStation[] = [
  {
    id: "focus-zone-relax-mellow",
    name: "focus-zone-relax-mellow",
    mood: "CHILLING",
    listeners: 8,
    bpm: 74,
    keyRootMidi: 60,
    description: "Warm electric piano chords and soft dusty boom-bap for calm evening flow.",
    accent: "#3b82f6",
    chords: [
      [53, 57, 60, 64, 67], // Fmaj9
      [52, 55, 59, 62, 67], // Em7(add11)
      [50, 53, 57, 60, 64], // Dm9
      [48, 52, 55, 59, 62], // Cmaj9
    ],
  },
  {
    id: "midnight-study-rhodes",
    name: "midnight-study-rhodes",
    mood: "FOCUSING",
    listeners: 14,
    bpm: 78,
    keyRootMidi: 57,
    description: "Steady mellow Rhodes loops with minimal percussion for deep concentration.",
    accent: "#8b5cf6",
    chords: [
      [57, 60, 64, 67, 71], // Am9
      [53, 57, 60, 64, 69], // Fmaj7
      [55, 59, 62, 65, 69], // G9sus
      [52, 55, 59, 62, 66], // Em9
    ],
  },
  {
    id: "tokyo-afterhours-dub",
    name: "tokyo-afterhours-dub",
    mood: "DRIFTING",
    listeners: 11,
    bpm: 70,
    keyRootMidi: 58,
    description: "Rain-soaked minor seventh harmonies and deep tape-filtered sub bass.",
    accent: "#ec4899",
    chords: [
      [51, 55, 58, 62, 65], // Ebmaj9
      [50, 53, 57, 60, 65], // Dm7
      [48, 51, 55, 58, 62], // Cm9
      [46, 50, 53, 57, 60], // Bbmaj9
    ],
  },
  {
    id: "sleepy-sunset-tape",
    name: "sleepy-sunset-tape",
    mood: "UNWINDING",
    listeners: 6,
    bpm: 64,
    keyRootMidi: 62,
    description: "Slow hazy tape keys and whisper-soft brushes made for winding down.",
    accent: "#f59e0b",
    chords: [
      [55, 59, 62, 66, 69], // Gmaj9
      [54, 57, 61, 64, 69], // F#m7
      [52, 55, 59, 62, 66], // Em9
      [50, 54, 57, 61, 64], // Dmaj9
    ],
  },
];

export const AMBIENT_CHANNELS: AmbientChannelDef[] = [
  { id: "rain", name: "Rain", category: "Weather", description: "Steady midnight rain on glass" },
  { id: "vinyl", name: "Vinyl", category: "Texture", description: "Warm dusty record crackle" },
  { id: "fireplace", name: "Fireplace", category: "Cozy", description: "Soft crackling hearth embers" },
  { id: "thunder", name: "Thunder", category: "Weather", description: "Distant low rolling rumbles" },
  { id: "wind", name: "Night Wind", category: "Nature", description: "Gentle breeze outside the window" },
  { id: "cafe", name: "Cafe", category: "Space", description: "Quiet muffled coffeehouse room" },
  { id: "forest", name: "Forest", category: "Nature", description: "Soft evening crickets & leaves" },
  { id: "ocean", name: "Ocean", category: "Nature", description: "Slow rhythmic nocturnal surf" },
];

interface ProceduralAmbientNode {
  gain: GainNode;
  cleanup: () => void;
}

interface CustomAmbientNode {
  audio: HTMLAudioElement;
  gain: GainNode;
  source: MediaElementAudioSourceNode;
}

function midiToHz(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}

export class LofiAudioEngine {
  private ctx: AudioContext | null = null;
  private eqNodes: BiquadFilterNode[] = [];
  private tapeFilter: BiquadFilterNode | null = null;
  private masterGain: GainNode | null = null;
  private sleepGain: GainNode | null = null;
  private synthBus: GainNode | null = null;
  private ambienceBus: GainNode | null = null;
  private mediaEl: HTMLAudioElement | null = null;
  private mediaSource: MediaElementAudioSourceNode | null = null;
  private noiseBuffer: AudioBuffer | null = null;
  private brownBuffer: AudioBuffer | null = null;

  private isPlaying = false;
  private mode: "station" | "local" | "stream" = "station";
  private currentStation: LofiStation = BUILTIN_STATIONS[0];
  private stepTimer: number | null = null;
  private stepIndex = 0;

  private volume = 0.72;
  private muted = false;
  private eqBands: [number, number, number, number, number] = [0, 0, 0, 0, 0];
  private tapeCutoffHz = 18500;
  private ambienceLevels: Record<string, number> = {};

  private activeProceduralAmbience = new Map<string, ProceduralAmbientNode>();
  private activeCustomAmbience = new Map<string, CustomAmbientNode>();

  public onTrackEnded: (() => void) | null = null;
  public onPlaybackError: ((msg: string) => void) | null = null;
  public onTimeUpdate: ((currentTime: number, duration: number) => void) | null = null;

  private ensureContext(): AudioContext {
    if (this.ctx) return this.ctx;
    const AudioCtx = window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    const ctx = new AudioCtx();
    this.ctx = ctx;

    // 5-band parametric EQ chain
    this.eqNodes = EQ_BAND_FREQUENCIES.map((freq, idx) => {
      const filter = ctx.createBiquadFilter();
      filter.type =
        idx === 0 ? "lowshelf" : idx === EQ_BAND_FREQUENCIES.length - 1 ? "highshelf" : "peaking";
      filter.frequency.value = freq;
      filter.Q.value = 1.05;
      filter.gain.value = this.eqBands[idx] ?? 0;
      return filter;
    });

    this.tapeFilter = ctx.createBiquadFilter();
    this.tapeFilter.type = "lowpass";
    this.tapeFilter.frequency.value = this.tapeCutoffHz;
    this.tapeFilter.Q.value = 0.7;

    const compressor = ctx.createDynamicsCompressor();
    compressor.threshold.value = -16;
    compressor.knee.value = 18;
    compressor.ratio.value = 3;
    compressor.attack.value = 0.01;
    compressor.release.value = 0.25;

    this.masterGain = ctx.createGain();
    this.masterGain.gain.value = this.muted ? 0 : this.volume;

    this.sleepGain = ctx.createGain();
    this.sleepGain.gain.value = 1;

    this.synthBus = ctx.createGain();
    this.synthBus.gain.value = 0.72;

    this.ambienceBus = ctx.createGain();
    this.ambienceBus.gain.value = 0.85;

    // Connect music bus through EQ -> Tape Filter -> Compressor -> Master -> Sleep -> Destination
    this.synthBus.connect(this.eqNodes[0]);
    for (let i = 0; i < this.eqNodes.length - 1; i++) {
      this.eqNodes[i].connect(this.eqNodes[i + 1]);
    }
    this.eqNodes[this.eqNodes.length - 1].connect(this.tapeFilter);
    this.tapeFilter.connect(compressor);
    // Ambience routes gently alongside music into Master Gain so volume & sleep fade affect everything
    this.ambienceBus.connect(this.masterGain);
    compressor.connect(this.masterGain);
    this.masterGain.connect(this.sleepGain);
    this.sleepGain.connect(ctx.destination);

    // Media element for local audio files and custom streams
    this.mediaEl = document.createElement("audio");
    this.mediaEl.crossOrigin = "anonymous";
    this.mediaEl.preload = "metadata";
    this.mediaEl.addEventListener("ended", () => {
      if (this.mode !== "station") {
        this.onTrackEnded?.();
      }
    });
    this.mediaEl.addEventListener("timeupdate", () => {
      if (this.mediaEl && this.mode !== "station") {
        this.onTimeUpdate?.(this.mediaEl.currentTime || 0, this.mediaEl.duration || 0);
      }
    });
    this.mediaEl.addEventListener("error", () => {
      if (this.mode !== "station" && this.isPlaying) {
        this.onPlaybackError?.("Unable to decode audio stream or local file");
      }
    });

    this.mediaSource = ctx.createMediaElementSource(this.mediaEl);
    this.mediaSource.connect(this.eqNodes[0]);

    this.createNoiseBuffers(ctx);
    return ctx;
  }

  private createNoiseBuffers(ctx: AudioContext): void {
    const length = ctx.sampleRate * 3;
    const pink = ctx.createBuffer(2, length, ctx.sampleRate);
    const brown = ctx.createBuffer(2, length, ctx.sampleRate);

    for (let ch = 0; ch < 2; ch++) {
      const pOut = pink.getChannelData(ch);
      const bOut = brown.getChannelData(ch);
      let b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0;
      let lastBrown = 0;
      for (let i = 0; i < length; i++) {
        const white = Math.random() * 2 - 1;
        b0 = 0.99886 * b0 + white * 0.0555179;
        b1 = 0.99332 * b1 + white * 0.0750759;
        b2 = 0.96900 * b2 + white * 0.1538520;
        b3 = 0.86650 * b3 + white * 0.3104856;
        b4 = 0.55000 * b4 + white * 0.5329522;
        b5 = -0.7616 * b5 - white * 0.0168980;
        pOut[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.05;
        b6 = white * 0.115926;

        lastBrown = (lastBrown + 0.02 * white) / 1.02;
        bOut[i] = lastBrown * 2.4;
      }
    }
    this.noiseBuffer = pink;
    this.brownBuffer = brown;
  }

  public getPlaying(): boolean {
    return this.isPlaying;
  }

  public async play(): Promise<void> {
    const ctx = this.ensureContext();
    if (ctx.state === "suspended") {
      await ctx.resume();
    }
    if (this.sleepGain) {
      this.sleepGain.gain.cancelScheduledValues(ctx.currentTime);
      this.sleepGain.gain.setValueAtTime(1, ctx.currentTime);
    }

    this.isPlaying = true;
    this.syncAllAmbience();

    if (this.mode === "station") {
      this.mediaEl?.pause();
      this.startStationSequencer();
    } else if (this.mediaEl) {
      this.stopStationSequencer();
      try {
        await this.mediaEl.play();
      } catch (err) {
        this.isPlaying = false;
        this.onPlaybackError?.(
          err instanceof Error ? err.message : "Failed to start audio playback"
        );
      }
    }
  }

  public pause(): void {
    this.isPlaying = false;
    this.stopStationSequencer();
    this.mediaEl?.pause();
    this.suspendAmbience(true);
  }

  public async togglePlay(): Promise<boolean> {
    if (this.isPlaying) {
      this.pause();
    } else {
      await this.play();
    }
    return this.isPlaying;
  }

  public async selectStation(stationId: string, autoPlay = true): Promise<LofiStation> {
    const station =
      BUILTIN_STATIONS.find((s) => s.id === stationId) ?? BUILTIN_STATIONS[0];
    this.currentStation = station;
    this.mode = "station";
    this.stepIndex = 0;
    this.mediaEl?.pause();
    if (autoPlay || this.isPlaying) {
      this.stopStationSequencer();
      await this.play();
    }
    return station;
  }

  public async playMediaUrl(url: string, kind: "local" | "stream", autoPlay = true): Promise<void> {
    const ctx = this.ensureContext();
    if (ctx.state === "suspended" && autoPlay) {
      await ctx.resume();
    }
    this.mode = kind;
    this.stopStationSequencer();
    if (this.mediaEl) {
      this.mediaEl.src = url;
      this.mediaEl.load();
    }
    if (autoPlay) {
      await this.play();
    }
  }

  public seekTo(seconds: number): void {
    if (this.mediaEl && this.mode !== "station" && Number.isFinite(seconds)) {
      this.mediaEl.currentTime = Math.max(0, seconds);
    }
  }

  public setVolume(value: number): void {
    this.volume = Math.max(0, Math.min(1, value));
    if (this.masterGain && this.ctx) {
      const target = this.muted ? 0 : this.volume;
      this.masterGain.gain.setTargetAtTime(target, this.ctx.currentTime, 0.03);
    }
  }

  public setMuted(muted: boolean): void {
    this.muted = muted;
    if (this.masterGain && this.ctx) {
      const target = this.muted ? 0 : this.volume;
      this.masterGain.gain.setTargetAtTime(target, this.ctx.currentTime, 0.03);
    }
  }

  public setEqBands(bands: [number, number, number, number, number]): void {
    this.eqBands = [...bands] as [number, number, number, number, number];
    if (this.ctx) {
      this.eqNodes.forEach((node, idx) => {
        node.gain.setTargetAtTime(this.eqBands[idx] ?? 0, this.ctx!.currentTime, 0.04);
      });
    }
  }

  public setTapeCutoff(hz: number): void {
    this.tapeCutoffHz = Math.max(1200, Math.min(20000, hz));
    if (this.tapeFilter && this.ctx) {
      this.tapeFilter.frequency.setTargetAtTime(this.tapeCutoffHz, this.ctx.currentTime, 0.05);
    }
  }

  public setSleepFade(multiplier: number): void {
    if (this.sleepGain && this.ctx) {
      this.sleepGain.gain.setTargetAtTime(
        Math.max(0, Math.min(1, multiplier)),
        this.ctx.currentTime,
        0.15
      );
    }
  }

  public setAmbienceLevel(id: string, level: number, customUrl?: string): void {
    const clamped = Math.max(0, Math.min(1, level));
    if (clamped <= 0.005) {
      delete this.ambienceLevels[id];
      this.stopAmbientChannel(id);
      return;
    }
    this.ambienceLevels[id] = clamped;
    if (!this.ctx) {
      // Wait until user initiates audio or initialize immediately if interacting with Sounds
      this.ensureContext();
    }
    if (this.ctx && this.ctx.state === "suspended") {
      void this.ctx.resume();
    }
    this.applyAmbientChannel(id, clamped, customUrl);
  }

  public setAmbienceMap(map: Record<string, number>): void {
    for (const id of [...this.activeProceduralAmbience.keys(), ...this.activeCustomAmbience.keys()]) {
      if (!map[id] || map[id] <= 0.005) {
        this.stopAmbientChannel(id);
      }
    }
    this.ambienceLevels = {};
    for (const [id, val] of Object.entries(map)) {
      if (val > 0.005) {
        this.ambienceLevels[id] = val;
      }
    }
    if (this.isPlaying) {
      this.syncAllAmbience();
    }
  }

  public clearAmbience(): void {
    this.ambienceLevels = {};
    for (const id of [...this.activeProceduralAmbience.keys()]) {
      this.stopAmbientChannel(id);
    }
    for (const id of [...this.activeCustomAmbience.keys()]) {
      this.stopAmbientChannel(id);
    }
  }

  public playChime(): void {
    const ctx = this.ensureContext();
    if (ctx.state === "suspended") void ctx.resume();
    const now = ctx.currentTime;
    [523.25, 659.25, 783.99].forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const g = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      g.gain.setValueAtTime(0.0001, now + i * 0.14);
      g.gain.exponentialRampToValueAtTime(0.12, now + i * 0.14 + 0.03);
      g.gain.exponentialRampToValueAtTime(0.0001, now + i * 0.14 + 1.1);
      osc.connect(g);
      g.connect(ctx.destination);
      osc.start(now + i * 0.14);
      osc.stop(now + i * 0.14 + 1.2);
    });
  }

  // --- Built-in Procedural Lofi Synthesizer ---
  private startStationSequencer(): void {
    this.stopStationSequencer();
    const eighthMs = (60 / this.currentStation.bpm / 2) * 1000;
    this.triggerStationStep();
    this.stepTimer = window.setInterval(() => {
      if (!this.isPlaying || this.mode !== "station") return;
      this.triggerStationStep();
    }, eighthMs);
  }

  private stopStationSequencer(): void {
    if (this.stepTimer !== null) {
      clearInterval(this.stepTimer);
      this.stepTimer = null;
    }
  }

  private triggerStationStep(): void {
    if (!this.ctx || !this.synthBus) return;
    const ctx = this.ctx;
    const now = ctx.currentTime + 0.02;
    const st = this.currentStation;
    const stepInBar = this.stepIndex % 8;
    const barIndex = Math.floor(this.stepIndex / 8) % st.chords.length;
    const chord = st.chords[barIndex];
    const swing = stepInBar % 2 === 1 ? 0.024 : 0;
    const t = now + swing;

    // 1. Warm Rhodes chord on beat 1 (and gentle re-articulation on beat 3.5)
    if (stepInBar === 0 || (stepInBar === 5 && Math.random() < 0.35)) {
      const dur = stepInBar === 0 ? (60 / st.bpm) * 3.7 : (60 / st.bpm) * 1.4;
      const vel = stepInBar === 0 ? 0.095 : 0.045;
      chord.forEach((midi, idx) => {
        const osc = ctx.createOscillator();
        const overtone = ctx.createOscillator();
        const filter = ctx.createBiquadFilter();
        const gain = ctx.createGain();

        const detuneCents = (Math.random() - 0.5) * 8; // Subtle analog tape drift
        osc.type = "triangle";
        osc.frequency.value = midiToHz(midi);
        osc.detune.value = detuneCents;

        overtone.type = "sine";
        overtone.frequency.value = midiToHz(midi) * 2;
        overtone.detune.value = detuneCents;

        const overtoneGain = ctx.createGain();
        overtoneGain.gain.setValueAtTime(0.18, t);
        overtoneGain.gain.exponentialRampToValueAtTime(0.01, t + 0.35);

        filter.type = "lowpass";
        filter.frequency.setValueAtTime(1600, t);
        filter.frequency.exponentialRampToValueAtTime(680, t + dur);

        const stagger = idx * 0.018;
        gain.gain.setValueAtTime(0.0001, t + stagger);
        gain.gain.linearRampToValueAtTime(vel, t + stagger + 0.045);
        gain.gain.exponentialRampToValueAtTime(vel * 0.45, t + stagger + dur * 0.55);
        gain.gain.exponentialRampToValueAtTime(0.0001, t + stagger + dur);

        osc.connect(filter);
        overtone.connect(overtoneGain);
        overtoneGain.connect(filter);
        filter.connect(gain);
        gain.connect(this.synthBus!);

        osc.start(t + stagger);
        overtone.start(t + stagger);
        osc.stop(t + stagger + dur + 0.05);
        overtone.stop(t + stagger + dur + 0.05);
      });
    }

    // 2. Warm sub bass on beat 1 & occasional syncopation
    if (stepInBar === 0 || (stepInBar === 6 && Math.random() < 0.5)) {
      const bassOsc = ctx.createOscillator();
      const bassGain = ctx.createGain();
      bassOsc.type = "sine";
      const root = chord[0] - 12;
      bassOsc.frequency.value = midiToHz(root);
      const bDur = stepInBar === 0 ? 1.5 : 0.65;
      bassGain.gain.setValueAtTime(0.0001, t);
      bassGain.gain.linearRampToValueAtTime(0.22, t + 0.04);
      bassGain.gain.exponentialRampToValueAtTime(0.0001, t + bDur);
      bassOsc.connect(bassGain);
      bassGain.connect(this.synthBus);
      bassOsc.start(t);
      bassOsc.stop(t + bDur + 0.04);
    }

    // 3. Soft dusty boom-bap kick (steps 0 and 5)
    if (stepInBar === 0 || (stepInBar === 5 && Math.random() < 0.72)) {
      const kick = ctx.createOscillator();
      const kGain = ctx.createGain();
      kick.type = "sine";
      kick.frequency.setValueAtTime(115, t);
      kick.frequency.exponentialRampToValueAtTime(42, t + 0.14);
      kGain.gain.setValueAtTime(0.28, t);
      kGain.gain.exponentialRampToValueAtTime(0.0001, t + 0.24);
      kick.connect(kGain);
      kGain.connect(this.synthBus);
      kick.start(t);
      kick.stop(t + 0.26);
    }

    // 4. Muffled brush snare on beats 2 & 4 (steps 2 and 6)
    if ((stepInBar === 2 || stepInBar === 6) && this.noiseBuffer) {
      const snare = ctx.createBufferSource();
      snare.buffer = this.noiseBuffer;
      const bp = ctx.createBiquadFilter();
      bp.type = "bandpass";
      bp.frequency.value = 1450;
      bp.Q.value = 0.9;
      const sGain = ctx.createGain();
      sGain.gain.setValueAtTime(0.11, t);
      sGain.gain.exponentialRampToValueAtTime(0.0001, t + 0.13);
      snare.connect(bp);
      bp.connect(sGain);
      sGain.connect(this.synthBus);
      snare.start(t, Math.random() * 1.5, 0.16);
    }

    // 5. Whisper hi-hat on 8th notes
    if (this.noiseBuffer && (stepInBar % 2 === 0 || Math.random() < 0.75)) {
      const hat = ctx.createBufferSource();
      hat.buffer = this.noiseBuffer;
      const hp = ctx.createBiquadFilter();
      hp.type = "highpass";
      hp.frequency.value = 6200;
      const hGain = ctx.createGain();
      const amp = stepInBar % 2 === 0 ? 0.036 : 0.02;
      hGain.gain.setValueAtTime(amp, t);
      hGain.gain.exponentialRampToValueAtTime(0.0001, t + 0.045);
      hat.connect(hp);
      hp.connect(hGain);
      hGain.connect(this.synthBus);
      hat.start(t, Math.random() * 2.0, 0.06);
    }

    // 6. Occasional dreamy pentatonic bell melody note
    if ((stepInBar === 3 || stepInBar === 7) && Math.random() < 0.45) {
      const note = chord[Math.floor(Math.random() * chord.length)] + 12;
      const bell = ctx.createOscillator();
      const bGain = ctx.createGain();
      bell.type = "sine";
      bell.frequency.value = midiToHz(note);
      bGain.gain.setValueAtTime(0.0001, t);
      bGain.gain.linearRampToValueAtTime(0.055, t + 0.03);
      bGain.gain.exponentialRampToValueAtTime(0.0001, t + 1.15);
      bell.connect(bGain);
      bGain.connect(this.synthBus);
      bell.start(t);
      bell.stop(t + 1.2);
    }

    this.stepIndex++;
  }

  // --- Procedural + Custom Local Ambient Mixer ---
  private syncAllAmbience(): void {
    for (const [id, level] of Object.entries(this.ambienceLevels)) {
      if (level > 0.005) {
        this.applyAmbientChannel(id, level);
      }
    }
    this.suspendAmbience(false);
  }

  private suspendAmbience(paused: boolean): void {
    if (!this.ctx || !this.ambienceBus) return;
    this.ambienceBus.gain.setTargetAtTime(paused ? 0 : 0.85, this.ctx.currentTime, 0.08);
    for (const custom of this.activeCustomAmbience.values()) {
      if (paused) custom.audio.pause();
      else void custom.audio.play();
    }
  }

  private applyAmbientChannel(id: string, level: number, customUrl?: string): void {
    const ctx = this.ensureContext();
    if (!this.ambienceBus) return;

    if (customUrl || this.activeCustomAmbience.has(id)) {
      let node = this.activeCustomAmbience.get(id);
      if (!node && customUrl) {
        const audio = document.createElement("audio");
        audio.crossOrigin = "anonymous";
        audio.loop = true;
        audio.src = customUrl;
        const source = ctx.createMediaElementSource(audio);
        const gain = ctx.createGain();
        gain.gain.value = level * 0.8;
        source.connect(gain);
        gain.connect(this.ambienceBus);
        void audio.play();
        node = { audio, gain, source };
        this.activeCustomAmbience.set(id, node);
      } else if (node) {
        node.gain.gain.setTargetAtTime(level * 0.8, ctx.currentTime, 0.05);
        if (node.audio.paused) void node.audio.play();
      }
      return;
    }

    const existing = this.activeProceduralAmbience.get(id);
    if (existing) {
      existing.gain.gain.setTargetAtTime(level * 0.65, ctx.currentTime, 0.06);
      return;
    }

    const created = this.createProceduralAmbient(id, level * 0.65);
    if (created) {
      this.activeProceduralAmbience.set(id, created);
    }
  }

  private stopAmbientChannel(id: string): void {
    const proc = this.activeProceduralAmbience.get(id);
    if (proc) {
      proc.cleanup();
      this.activeProceduralAmbience.delete(id);
    }
    const custom = this.activeCustomAmbience.get(id);
    if (custom) {
      custom.audio.pause();
      custom.source.disconnect();
      custom.gain.disconnect();
      this.activeCustomAmbience.delete(id);
    }
  }

  private createProceduralAmbient(id: string, initialGain: number): ProceduralAmbientNode | null {
    if (!this.ctx || !this.ambienceBus || !this.noiseBuffer || !this.brownBuffer) return null;
    const ctx = this.ctx;
    const master = ctx.createGain();
    master.gain.value = initialGain;
    master.connect(this.ambienceBus);

    const disposers: Array<() => void> = [];

    const makeLoop = (buf: AudioBuffer): AudioBufferSourceNode => {
      const src = ctx.createBufferSource();
      src.buffer = buf;
      src.loop = true;
      src.start();
      disposers.push(() => {
        try {
          src.stop();
          src.disconnect();
        } catch {
          // ignore
        }
      });
      return src;
    };

    switch (id) {
      case "rain": {
        const src = makeLoop(this.noiseBuffer);
        const bp = ctx.createBiquadFilter();
        bp.type = "bandpass";
        bp.frequency.value = 1150;
        bp.Q.value = 0.55;
        const hp = ctx.createBiquadFilter();
        hp.type = "highpass";
        hp.frequency.value = 260;
        src.connect(hp);
        hp.connect(bp);
        bp.connect(master);

        // Occasional window glass droplets
        const dropTimer = window.setInterval(() => {
          if (!this.ctx || Math.random() > 0.45) return;
          const t = ctx.currentTime;
          const osc = ctx.createOscillator();
          const dg = ctx.createGain();
          osc.type = "sine";
          osc.frequency.setValueAtTime(1400 + Math.random() * 1400, t);
          osc.frequency.exponentialRampToValueAtTime(600, t + 0.04);
          dg.gain.setValueAtTime(0.035, t);
          dg.gain.exponentialRampToValueAtTime(0.0001, t + 0.05);
          osc.connect(dg);
          dg.connect(master);
          osc.start(t);
          osc.stop(t + 0.06);
        }, 280);
        disposers.push(() => clearInterval(dropTimer));
        break;
      }

      case "vinyl": {
        const hiss = makeLoop(this.noiseBuffer);
        const hp = ctx.createBiquadFilter();
        hp.type = "highpass";
        hp.frequency.value = 3400;
        const hissGain = ctx.createGain();
        hissGain.gain.value = 0.16;
        hiss.connect(hp);
        hp.connect(hissGain);
        hissGain.connect(master);

        const popTimer = window.setInterval(() => {
          if (!this.ctx || !this.noiseBuffer || Math.random() > 0.52) return;
          const t = ctx.currentTime;
          const pop = ctx.createBufferSource();
          pop.buffer = this.noiseBuffer;
          const bp = ctx.createBiquadFilter();
          bp.type = "bandpass";
          bp.frequency.value = 2400 + Math.random() * 3200;
          bp.Q.value = 2.2;
          const pg = ctx.createGain();
          pg.gain.setValueAtTime(0.28 * Math.random() + 0.08, t);
          pg.gain.exponentialRampToValueAtTime(0.0001, t + 0.014);
          pop.connect(bp);
          bp.connect(pg);
          pg.connect(master);
          pop.start(t, Math.random() * 2, 0.02);
        }, 160);
        disposers.push(() => clearInterval(popTimer));
        break;
      }

      case "fireplace": {
        const rumble = makeLoop(this.brownBuffer);
        const lp = ctx.createBiquadFilter();
        lp.type = "lowpass";
        lp.frequency.value = 280;
        rumble.connect(lp);
        lp.connect(master);

        const crackleTimer = window.setInterval(() => {
          if (!this.ctx || !this.noiseBuffer || Math.random() > 0.55) return;
          const t = ctx.currentTime;
          const snap = ctx.createBufferSource();
          snap.buffer = this.noiseBuffer;
          const bp = ctx.createBiquadFilter();
          bp.type = "bandpass";
          bp.frequency.value = 1800 + Math.random() * 2500;
          bp.Q.value = 3.0;
          const sg = ctx.createGain();
          sg.gain.setValueAtTime(0.35 * Math.random() + 0.1, t);
          sg.gain.exponentialRampToValueAtTime(0.0001, t + 0.028);
          snap.connect(bp);
          bp.connect(sg);
          sg.connect(master);
          snap.start(t, Math.random() * 2, 0.04);
        }, 210);
        disposers.push(() => clearInterval(crackleTimer));
        break;
      }

      case "thunder": {
        const triggerRumble = () => {
          if (!this.ctx || !this.brownBuffer) return;
          const t = ctx.currentTime;
          const src = ctx.createBufferSource();
          src.buffer = this.brownBuffer;
          src.loop = true;
          const lp = ctx.createBiquadFilter();
          lp.type = "lowpass";
          lp.frequency.setValueAtTime(160, t);
          lp.frequency.exponentialRampToValueAtTime(55, t + 3.6);
          const tg = ctx.createGain();
          tg.gain.setValueAtTime(0.0001, t);
          tg.gain.linearRampToValueAtTime(0.85, t + 0.35);
          tg.gain.exponentialRampToValueAtTime(0.0001, t + 4.0);
          src.connect(lp);
          lp.connect(tg);
          tg.connect(master);
          src.start(t);
          src.stop(t + 4.2);
        };
        triggerRumble();
        const tTimer = window.setInterval(() => {
          if (Math.random() < 0.45) triggerRumble();
        }, 6500);
        disposers.push(() => clearInterval(tTimer));
        break;
      }

      case "wind": {
        const src = makeLoop(this.noiseBuffer);
        const bp = ctx.createBiquadFilter();
        bp.type = "bandpass";
        bp.frequency.value = 420;
        bp.Q.value = 2.4;
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        lfo.frequency.value = 0.12;
        lfoGain.gain.value = 210;
        lfo.connect(lfoGain);
        lfoGain.connect(bp.frequency);
        lfo.start();
        src.connect(bp);
        bp.connect(master);
        disposers.push(() => {
          try {
            lfo.stop();
          } catch {
            // ignore
          }
        });
        break;
      }

      case "cafe": {
        const src = makeLoop(this.brownBuffer);
        const bp = ctx.createBiquadFilter();
        bp.type = "bandpass";
        bp.frequency.value = 640;
        bp.Q.value = 0.8;
        src.connect(bp);
        bp.connect(master);
        break;
      }

      case "forest": {
        const src = makeLoop(this.noiseBuffer);
        const lp = ctx.createBiquadFilter();
        lp.type = "bandpass";
        lp.frequency.value = 720;
        lp.Q.value = 0.9;
        const bgGain = ctx.createGain();
        bgGain.gain.value = 0.32;
        src.connect(lp);
        lp.connect(bgGain);
        bgGain.connect(master);

        const chirpTimer = window.setInterval(() => {
          if (!this.ctx || Math.random() > 0.45) return;
          const t = ctx.currentTime;
          for (let i = 0; i < 3; i++) {
            const osc = ctx.createOscillator();
            const cg = ctx.createGain();
            osc.type = "sine";
            osc.frequency.value = 4250 + Math.random() * 140;
            cg.gain.setValueAtTime(0.0001, t + i * 0.065);
            cg.gain.linearRampToValueAtTime(0.03, t + i * 0.065 + 0.015);
            cg.gain.exponentialRampToValueAtTime(0.0001, t + i * 0.065 + 0.05);
            osc.connect(cg);
            cg.connect(master);
            osc.start(t + i * 0.065);
            osc.stop(t + i * 0.065 + 0.055);
          }
        }, 1250);
        disposers.push(() => clearInterval(chirpTimer));
        break;
      }

      case "ocean": {
        const src = makeLoop(this.brownBuffer);
        const lp = ctx.createBiquadFilter();
        lp.type = "lowpass";
        lp.frequency.value = 520;
        const swellGain = ctx.createGain();
        swellGain.gain.value = 0.55;
        const lfo = ctx.createOscillator();
        const lfoGain = ctx.createGain();
        lfo.frequency.value = 0.11;
        lfoGain.gain.value = 0.38;
        lfo.connect(lfoGain);
        lfoGain.connect(swellGain.gain);
        lfo.start();
        src.connect(lp);
        lp.connect(swellGain);
        swellGain.connect(master);
        disposers.push(() => {
          try {
            lfo.stop();
          } catch {
            // ignore
          }
        });
        break;
      }

      default:
        return null;
    }

    return {
      gain: master,
      cleanup: () => {
        disposers.forEach((fn) => fn());
        master.disconnect();
      },
    };
  }
}
