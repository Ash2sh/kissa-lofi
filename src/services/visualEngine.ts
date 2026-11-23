import type { CanvasSceneDef, VisualFxSettings, VisualSource } from "../types";

export const BUILTIN_CANVAS_SCENES: CanvasSceneDef[] = [
  {
    id: "rainy-window-cat",
    name: "Rainy Window Cat",
    subtitle: "Open night window, city lights & rain",
    previewGradient: "linear-gradient(135deg, #1b1742 0%, #3d3584 52%, #110d24 100%)",
  },
  {
    id: "neon-rain-skyline",
    name: "Neon Rain Skyline",
    subtitle: "Rainy midnight towers & soft neon glow",
    previewGradient: "linear-gradient(135deg, #0d132b 0%, #2a184d 55%, #4a1c4f 100%)",
  },
  {
    id: "midnight-starlight",
    name: "Midnight Starlight",
    subtitle: "Attic skylight & drifting shooting stars",
    previewGradient: "linear-gradient(135deg, #06091a 0%, #14224a 55%, #1d1a3d 100%)",
  },
  {
    id: "warm-study-dusk",
    name: "Warm Study Dusk",
    subtitle: "Golden dusk clouds & drifting dust motes",
    previewGradient: "linear-gradient(135deg, #27152e 0%, #6d3b47 52%, #c97a53 100%)",
  },
];

interface Raindrop {
  x: number;
  y: number;
  len: number;
  speed: number;
  alpha: number;
  width: number;
}

interface Star {
  x: number;
  y: number;
  r: number;
  phase: number;
  speed: number;
}

interface CityWindow {
  bx: number;
  by: number;
  bw: number;
  bh: number;
  lit: boolean;
  warm: number;
  flickerPhase: number;
}

export class VisualEngine {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private video: HTMLVideoElement;
  private image: HTMLImageElement;
  private dimLayer: HTMLDivElement;

  private source: VisualSource = {
    kind: "canvas",
    id: "rainy-window-cat",
    label: "Rainy Window Cat",
    streamUrl: null,
  };
  private fx: VisualFxSettings = {
    rainOverlay: 0.55,
    vignette: 0.35,
    filmGrain: 0.1,
    dim: 0.04,
    warmTint: 0.08,
    blur: 0,
    fpsCap: 30,
    pauseWhenHidden: true,
  };

  private width = 1280;
  private height = 720;
  private dpr = 1;
  private rafId: number | null = null;
  private lastFrameMs = 0;
  private elapsedSec = 0;
  private hidden = false;

  private raindrops: Raindrop[] = [];
  private stars: Star[] = [];
  private cityWindows: CityWindow[] = [];
  private grainPatternCanvas: HTMLCanvasElement;

  constructor(
    canvas: HTMLCanvasElement,
    video: HTMLVideoElement,
    image: HTMLImageElement,
    dimLayer: HTMLDivElement
  ) {
    this.canvas = canvas;
    const context = canvas.getContext("2d", { alpha: true });
    if (!context) {
      throw new Error("2D Canvas context unavailable");
    }
    this.ctx = context;
    this.video = video;
    this.image = image;
    this.dimLayer = dimLayer;

    this.grainPatternCanvas = this.buildGrainTile();
    this.initParticles();
    this.handleResize();

    window.addEventListener("resize", this.handleResize);
    document.addEventListener("visibilitychange", this.handleVisibility);
  }

  public start(): void {
    if (this.rafId !== null) return;
    this.lastFrameMs = performance.now();
    this.rafId = requestAnimationFrame(this.loop);
  }

  public destroy(): void {
    if (this.rafId !== null) {
      cancelAnimationFrame(this.rafId);
      this.rafId = null;
    }
    window.removeEventListener("resize", this.handleResize);
    document.removeEventListener("visibilitychange", this.handleVisibility);
  }

  public setSource(source: VisualSource): void {
    this.source = { ...source };
    this.applyMediaElements();
  }

  public setFx(fx: VisualFxSettings): void {
    this.fx = { ...fx };
    this.applyMediaCssFilters();
  }

  private applyMediaElements(): void {
    if (this.source.kind === "video" && this.source.streamUrl) {
      this.image.classList.add("hidden");
      this.image.removeAttribute("src");
      if (this.video.src !== this.source.streamUrl) {
        this.video.src = this.source.streamUrl;
        this.video.load();
      }
      this.video.classList.remove("hidden");
      void this.video.play().catch(() => {});
    } else if (this.source.kind === "image" && this.source.streamUrl) {
      this.video.pause();
      this.video.classList.add("hidden");
      this.video.removeAttribute("src");
      this.image.src = this.source.streamUrl;
      this.image.classList.remove("hidden");
    } else {
      this.video.pause();
      this.video.classList.add("hidden");
      this.video.removeAttribute("src");
      this.image.classList.add("hidden");
      this.image.removeAttribute("src");
    }
    this.applyMediaCssFilters();
  }

  private applyMediaCssFilters(): void {
    const blurPx = this.fx.blur > 0 ? `blur(${this.fx.blur.toFixed(1)}px)` : "";
    const filter = blurPx || "none";
    this.video.style.filter = filter;
    this.image.style.filter = filter;
    this.dimLayer.style.background = `rgba(6, 7, 14, ${Math.max(0, Math.min(0.75, this.fx.dim)).toFixed(3)})`;
  }

  private handleResize = (): void => {
    // Cap DPR at 1.5 for ultra-low GPU fill-rate on 4K displays while remaining razor sharp
    this.dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    this.width = window.innerWidth;
    this.height = window.innerHeight;
    this.canvas.width = Math.max(1, Math.floor(this.width * this.dpr));
    this.canvas.height = Math.max(1, Math.floor(this.height * this.dpr));
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0);
  };

  private handleVisibility = (): void => {
    this.hidden = document.hidden;
    if (this.fx.pauseWhenHidden) {
      if (this.hidden) {
        this.video.pause();
      } else if (this.source.kind === "video" && this.source.streamUrl) {
        void this.video.play().catch(() => {});
      }
    }
  };

  private initParticles(): void {
    this.raindrops = [];
    for (let i = 0; i < 190; i++) {
      const depth = Math.random();
      this.raindrops.push({
        x: Math.random(),
        y: Math.random(),
        len: 0.03 + depth * 0.075,
        speed: 0.55 + depth * 0.95,
        alpha: 0.1 + depth * 0.28,
        width: 0.65 + depth * 0.9,
      });
    }

    this.stars = [];
    for (let i = 0; i < 120; i++) {
      this.stars.push({
        x: Math.random(),
        y: Math.random() * 0.72,
        r: 0.6 + Math.random() * 1.6,
        phase: Math.random() * Math.PI * 2,
        speed: 0.6 + Math.random() * 1.8,
      });
    }

    // Pre-seeded warm apartment windows for consistent cozy skyline
    this.cityWindows = [];
    const buildings = [
      // Left mid-rise tower (visible through window left side)
      { x: 0.295, y: 0.41, w: 0.085, h: 0.52, cols: 3, rows: 6 },
      // Right distant apartment block (behind cat)
      { x: 0.72, y: 0.28, w: 0.145, h: 0.66, cols: 4, rows: 7 },
    ];
    for (const b of buildings) {
      const cw = b.w / (b.cols * 1.65);
      const ch = b.h / (b.rows * 1.85);
      for (let r = 0; r < b.rows; r++) {
        for (let c = 0; c < b.cols; c++) {
          const lit = ((r * 7 + c * 13) % 5) !== 0 && (r > 0 || c > 0);
          this.cityWindows.push({
            bx: b.x + 0.012 + c * (b.w / b.cols) * 0.9,
            by: b.y + 0.03 + r * (b.h / b.rows) * 0.88,
            bw: cw,
            bh: ch,
            lit,
            warm: 0.72 + ((r * 3 + c * 5) % 4) * 0.07,
            flickerPhase: r * 1.3 + c * 2.1,
          });
        }
      }
    }
  }

  private buildGrainTile(): HTMLCanvasElement {
    const c = document.createElement("canvas");
    c.width = 96;
    c.height = 96;
    const g = c.getContext("2d");
    if (g) {
      const img = g.createImageData(96, 96);
      for (let i = 0; i < img.data.length; i += 4) {
        const v = Math.random() * 255;
        img.data[i] = v;
        img.data[i + 1] = v;
        img.data[i + 2] = v;
        img.data[i + 3] = 28;
      }
      g.putImageData(img, 0, 0);
    }
    return c;
  }

  private loop = (now: number): void => {
    this.rafId = requestAnimationFrame(this.loop);

    if (this.hidden && this.fx.pauseWhenHidden) {
      this.lastFrameMs = now;
      return;
    }

    const minDelta = 1000 / Math.max(10, Math.min(60, this.fx.fpsCap || 30));
    const deltaMs = now - this.lastFrameMs;
    if (deltaMs < minDelta) return;

    const dt = Math.min(deltaMs / 1000, 0.1);
    this.lastFrameMs = now;
    this.elapsedSec += dt;

    this.render(dt);
  };

  private render(dt: number): void {
    const ctx = this.ctx;
    const w = this.width;
    const h = this.height;

    ctx.clearRect(0, 0, w, h);

    if (this.source.kind === "canvas") {
      switch (this.source.id) {
        case "neon-rain-skyline":
          this.renderNeonSkyline(ctx, w, h, dt);
          break;
        case "midnight-starlight":
          this.renderMidnightStarlight(ctx, w, h);
          break;
        case "warm-study-dusk":
          this.renderWarmDusk(ctx, w, h);
          break;
        case "rainy-window-cat":
        default:
          this.renderRainyWindowCat(ctx, w, h, dt);
          break;
      }
    } else {
      // Local photo/video mode: render atmospheric rain overlay on canvas if enabled
      if (this.fx.rainOverlay > 0.01) {
        this.renderRainStreaks(ctx, w, h, dt, this.fx.rainOverlay, 0, w);
      }
    }

    this.renderPostFx(ctx, w, h);
  }

  // --- Scene 1: Rainy Window Cat (Faithful recreation of the attached reference screenshot) ---
  private renderRainyWindowCat(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    dt: number
  ): void {
    const t = this.elapsedSec;
    const winLeft = w * 0.255;
    const winRight = w * 0.815;
    const winW = winRight - winLeft;

    // 1. Twilight Indigo-Violet Rainy Sky inside the window opening
    const skyGrad = ctx.createLinearGradient(winLeft, 0, winLeft, h);
    skyGrad.addColorStop(0, "#1d1a4b");
    skyGrad.addColorStop(0.28, "#312d78");
    skyGrad.addColorStop(0.62, "#514a9e");
    skyGrad.addColorStop(0.84, "#796cc4");
    skyGrad.addColorStop(1, "#4a3f88");
    ctx.fillStyle = skyGrad;
    ctx.fillRect(0, 0, w, h);

    // Soft bright moonlight break behind upper-left clouds
    const moonGlow = ctx.createRadialGradient(
      w * 0.36,
      h * 0.06,
      10,
      w * 0.36,
      h * 0.09,
      w * 0.26
    );
    moonGlow.addColorStop(0, "rgba(218, 224, 245, 0.42)");
    moonGlow.addColorStop(0.45, "rgba(148, 154, 215, 0.22)");
    moonGlow.addColorStop(1, "rgba(49, 45, 120, 0)");
    ctx.fillStyle = moonGlow;
    ctx.fillRect(winLeft, 0, winW, h * 0.6);

    // Drifting dark indigo storm cloud bank across upper sky
    ctx.save();
    ctx.fillStyle = "rgba(28, 26, 76, 0.62)";
    const drift = Math.sin(t * 0.08) * 18;
    ctx.beginPath();
    ctx.moveTo(winLeft, 0);
    ctx.lineTo(winRight, 0);
    ctx.lineTo(winRight, h * 0.24);
    ctx.quadraticCurveTo(w * 0.68 + drift, h * 0.31, w * 0.56 + drift, h * 0.22);
    ctx.quadraticCurveTo(w * 0.48 + drift, h * 0.29, w * 0.42 + drift, h * 0.14);
    ctx.quadraticCurveTo(w * 0.34, h * 0.21, winLeft, h * 0.16);
    ctx.closePath();
    ctx.fill();
    ctx.restore();

    // Soft horizon mist glow behind city buildings
    const mistGrad = ctx.createRadialGradient(
      w * 0.53,
      h * 0.72,
      w * 0.02,
      w * 0.53,
      h * 0.72,
      w * 0.3
    );
    mistGrad.addColorStop(0, "rgba(182, 165, 240, 0.44)");
    mistGrad.addColorStop(0.55, "rgba(128, 112, 206, 0.20)");
    mistGrad.addColorStop(1, "rgba(70, 58, 136, 0)");
    ctx.fillStyle = mistGrad;
    ctx.fillRect(winLeft, h * 0.35, winW, h * 0.65);

    // 2. Hanging overhead street cables & warm bokeh string lights
    const sway = Math.sin(t * 0.85) * 4;
    ctx.strokeStyle = "rgba(16, 14, 34, 0.78)";
    ctx.lineWidth = 2;
    const wires = [
      { y1: h * 0.03, cy: h * 0.14 + sway, y2: h * 0.16 },
      { y1: h * 0.09, cy: h * 0.21 + sway * 0.8, y2: h * 0.19 },
      { y1: h * 0.0, cy: h * 0.06 + sway * 0.5, y2: h * 0.03 },
    ];
    for (const wire of wires) {
      ctx.beginPath();
      ctx.moveTo(winLeft - 10, wire.y1);
      ctx.quadraticCurveTo(w * 0.54, wire.cy, winRight + 10, wire.y2);
      ctx.stroke();
    }

    // Soft glowing street/fairy light bokeh dots
    const bokeh = [
      { x: 0.42, y: 0.23, r: 4.5, col: "255, 240, 195" },
      { x: 0.495, y: 0.14, r: 7, col: "255, 236, 175" },
      { x: 0.515, y: 0.115, r: 4, col: "245, 242, 220" },
      { x: 0.69, y: 0.04, r: 6.5, col: "255, 244, 210" },
      { x: 0.785, y: 0.175, r: 6, col: "255, 240, 195" },
    ];
    for (const b of bokeh) {
      const pulse = 0.78 + 0.22 * Math.sin(t * 1.4 + b.x * 15);
      const bx = w * b.x;
      const by = h * b.y + sway * 0.35;
      const g = ctx.createRadialGradient(bx, by, 0.5, bx, by, b.r * 3.2);
      g.addColorStop(0, `rgba(${b.col}, ${(0.95 * pulse).toFixed(2)})`);
      g.addColorStop(0.35, `rgba(${b.col}, ${(0.42 * pulse).toFixed(2)})`);
      g.addColorStop(1, `rgba(${b.col}, 0)`);
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(bx, by, b.r * 3.2, 0, Math.PI * 2);
      ctx.fill();
    }

    // 3. Distant City Building Silhouettes
    // Center mid-distance dark tower
    ctx.fillStyle = "#26214c";
    ctx.fillRect(w * 0.425, h * 0.54, w * 0.092, h * 0.46);
    ctx.fillRect(w * 0.44, h * 0.495, w * 0.062, h * 0.06);
    ctx.fillRect(w * 0.49, h * 0.485, w * 0.012, h * 0.015);
    ctx.fillRect(w * 0.41, h * 0.62, w * 0.12, h * 0.38);

    // Left foreground-mid apartment building
    ctx.fillStyle = "#19152f";
    ctx.fillRect(w * 0.28, h * 0.39, w * 0.088, h * 0.61);
    ctx.fillRect(w * 0.305, h * 0.435, w * 0.075, h * 0.565);
    ctx.fillRect(w * 0.275, h * 0.375, w * 0.108, h * 0.022); // roofline ledge

    // Right apartment building (behind the cat)
    ctx.fillStyle = "#16132b";
    ctx.fillRect(w * 0.715, h * 0.29, w * 0.12, h * 0.71);
    ctx.fillRect(w * 0.735, h * 0.265, w * 0.048, h * 0.035);
    // Tiny roof rails
    ctx.strokeStyle = "#1c1836";
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.moveTo(w * 0.742, h * 0.265);
    ctx.lineTo(w * 0.742, h * 0.235);
    ctx.moveTo(w * 0.765, h * 0.265);
    ctx.lineTo(w * 0.765, h * 0.238);
    ctx.stroke();

    // Warmly lit apartment windows with subtle glow
    for (const cw of this.cityWindows) {
      if (!cw.lit) continue;
      const x = w * cw.bx;
      const y = h * cw.by;
      const ww = w * cw.bw;
      const wh = h * cw.bh;
      const flicker = 0.92 + 0.08 * Math.sin(t * 0.7 + cw.flickerPhase);

      ctx.save();
      ctx.shadowColor = `rgba(255, 224, 145, ${(0.55 * flicker).toFixed(2)})`;
      ctx.shadowBlur = 12;
      ctx.fillStyle = `rgba(255, 232, 168, ${(cw.warm * flicker).toFixed(2)})`;
      ctx.fillRect(x, y, ww, wh);
      ctx.restore();

      // Window cross muntins
      ctx.strokeStyle = "rgba(22, 18, 42, 0.72)";
      ctx.lineWidth = 1.1;
      ctx.beginPath();
      ctx.moveTo(x + ww * 0.5, y);
      ctx.lineTo(x + ww * 0.5, y + wh);
      ctx.moveTo(x, y + wh * 0.5);
      ctx.lineTo(x + ww, y + wh * 0.5);
      ctx.stroke();
    }

    // 4. Rain streaks falling through the outdoor sky (behind cat & window frame)
    const rainStrength = Math.max(0.15, this.fx.rainOverlay);
    this.renderRainStreaks(ctx, w, h, dt, rainStrength, winLeft, winRight);

    // 5. Potted Leafy Plant on the left-center windowsill
    this.drawWindowsillPlant(ctx, w, h, t);

    // 6. Black Cat Silhouette perched on the right windowsill gazing left
    this.drawCatSilhouette(ctx, w, h, t);

    // 7. Dark Cozy Room Interior & Open Casement Window Frames (Left & Right)
    this.drawWindowFrame(ctx, w, h, winLeft, winRight);
  }

  private drawWindowsillPlant(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    t: number
  ): void {
    const cx = w * 0.415;
    const potTopY = h * 0.79;
    const potW = w * 0.165;
    const potH = h * 0.22;
    const breeze = Math.sin(t * 1.3) * 2.5;

    // Lush layered foliage dome
    const leafColors = ["#1c3222", "#2c4e2e", "#3d6b37", "#5b8944", "#81a858"];
    for (let i = 0; i < 42; i++) {
      const ratio = i / 41;
      const angle = Math.PI + ratio * Math.PI;
      const spreadX = Math.cos(angle) * potW * (0.48 + ((i * 7) % 5) * 0.045);
      const liftY = Math.sin(angle) * h * (0.095 + ((i * 11) % 6) * 0.015);
      const lx = cx + spreadX + breeze * (0.4 + (i % 3) * 0.25);
      const ly = potTopY + 10 + liftY;
      const rx = 10 + (i % 4) * 5;
      const ry = 5 + (i % 3) * 3;
      const rot = (ratio - 0.5) * 1.4 + Math.sin(t * 1.1 + i) * 0.04;

      ctx.save();
      ctx.translate(lx, ly);
      ctx.rotate(rot);
      ctx.fillStyle = leafColors[i % leafColors.length];
      ctx.beginPath();
      ctx.ellipse(0, 0, rx, ry, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();
    }

    // Terracotta pot body
    const potGrad = ctx.createLinearGradient(cx - potW * 0.5, potTopY, cx + potW * 0.5, potTopY);
    potGrad.addColorStop(0, "#4c1c1c");
    potGrad.addColorStop(0.55, "#6b2824");
    potGrad.addColorStop(1, "#351215");
    ctx.fillStyle = potGrad;
    ctx.beginPath();
    ctx.moveTo(cx - potW * 0.48, potTopY);
    ctx.lineTo(cx + potW * 0.48, potTopY);
    ctx.lineTo(cx + potW * 0.37, potTopY + potH);
    ctx.lineTo(cx - potW * 0.37, potTopY + potH);
    ctx.closePath();
    ctx.fill();

    // Pot rim highlight
    ctx.fillStyle = "#7a3028";
    ctx.fillRect(cx - potW * 0.51, potTopY - 4, potW * 1.02, 12);
    ctx.fillStyle = "rgba(18, 8, 12, 0.35)";
    ctx.fillRect(cx - potW * 0.48, potTopY + 8, potW * 0.96, 6);
  }

  private drawCatSilhouette(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    t: number
  ): void {
    const breath = Math.sin(t * 1.2) * 1.6;
    const earTwitch = Math.sin(t * 0.45) > 0.96 ? Math.sin(t * 18) * 2.5 : 0;

    ctx.save();
    ctx.fillStyle = "#0b0a17";

    ctx.beginPath();
    // Start at bottom left of cat chest near bottom edge
    ctx.moveTo(w * 0.642, h);
    // Curve up along chest & front leg
    ctx.bezierCurveTo(
      w * 0.638,
      h * 0.85,
      w * 0.665,
      h * 0.72,
      w * 0.642,
      h * 0.61 + breath
    );
    // Neck & chin facing left
    ctx.bezierCurveTo(
      w * 0.635,
      h * 0.56 + breath,
      w * 0.646,
      h * 0.515 + breath,
      w * 0.64,
      h * 0.495 + breath
    );
    // Chin / snout profile looking left
    ctx.bezierCurveTo(
      w * 0.612,
      h * 0.488 + breath,
      w * 0.614,
      h * 0.458 + breath,
      w * 0.62,
      h * 0.44 + breath
    );
    // Nose bridge to forehead
    ctx.quadraticCurveTo(
      w * 0.616,
      h * 0.415 + breath,
      w * 0.622,
      h * 0.385 + breath
    );
    // Left (front) pointed ear
    ctx.lineTo(w * 0.612 + earTwitch, h * 0.275 + breath);
    ctx.quadraticCurveTo(
      w * 0.648,
      h * 0.305 + breath,
      w * 0.665,
      h * 0.335 + breath
    );
    // Back ear slight peak
    ctx.lineTo(w * 0.684, h * 0.305 + breath);
    ctx.quadraticCurveTo(
      w * 0.715,
      h * 0.34 + breath,
      w * 0.725,
      h * 0.41 + breath
    );
    // Back of neck sloping down into seated back/haunch
    ctx.bezierCurveTo(
      w * 0.745,
      h * 0.49 + breath,
      w * 0.785,
      h * 0.62,
      w * 0.805,
      h * 0.8
    );
    ctx.lineTo(w * 0.818, h);
    ctx.closePath();
    ctx.fill();

    // Subtle cool violet rim-light along cat's left profile from the rainy sky
    ctx.strokeStyle = "rgba(155, 145, 228, 0.24)";
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.moveTo(w * 0.615 + earTwitch, h * 0.28 + breath);
    ctx.lineTo(w * 0.623, h * 0.388 + breath);
    ctx.quadraticCurveTo(w * 0.617, h * 0.43 + breath, w * 0.62, h * 0.46 + breath);
    ctx.stroke();

    // Fine silhouette whiskers
    ctx.strokeStyle = "rgba(14, 12, 26, 0.85)";
    ctx.lineWidth = 1;
    const wx = w * 0.618;
    const wy = h * 0.462 + breath;
    for (let i = -1; i <= 1; i++) {
      ctx.beginPath();
      ctx.moveTo(wx, wy + i * 3);
      ctx.lineTo(wx - 15, wy + i * 6 + 1);
      ctx.stroke();
    }
    ctx.restore();
  }

  private drawWindowFrame(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    winLeft: number,
    winRight: number
  ): void {
    // Left dark room wall
    const leftWall = ctx.createLinearGradient(0, 0, winLeft, 0);
    leftWall.addColorStop(0, "#0b0a18");
    leftWall.addColorStop(0.85, "#110f24");
    leftWall.addColorStop(1, "#181531");
    ctx.fillStyle = leftWall;
    ctx.fillRect(0, 0, winLeft * 0.86, h);

    // Subtle framed artwork on left interior wall (as seen in reference screenshot)
    ctx.strokeStyle = "#191730";
    ctx.lineWidth = 10;
    ctx.strokeRect(-14, -10, w * 0.085, h * 0.21);
    ctx.fillStyle = "#131127";
    ctx.fillRect(0, 0, w * 0.068, h * 0.185);

    // Open left casement window door (angled inward)
    ctx.fillStyle = "#16142e";
    ctx.fillRect(winLeft * 0.86, 0, winLeft * 0.14, h);
    ctx.fillStyle = "rgba(110, 120, 210, 0.14)";
    ctx.fillRect(winLeft * 0.96, 0, 3, h);
    // Subtle reflection inside open left sash
    const sashGradL = ctx.createLinearGradient(winLeft * 0.88, 0, winLeft * 0.99, h);
    sashGradL.addColorStop(0, "rgba(112, 118, 198, 0.16)");
    sashGradL.addColorStop(0.5, "rgba(60, 62, 128, 0.08)");
    sashGradL.addColorStop(1, "rgba(255, 218, 145, 0.09)");
    ctx.fillStyle = sashGradL;
    ctx.fillRect(winLeft * 0.89, h * 0.04, winLeft * 0.08, h * 0.92);

    // Right dark room wall & open right casement sash
    const rightWall = ctx.createLinearGradient(winRight, 0, w, 0);
    rightWall.addColorStop(0, "#14122a");
    rightWall.addColorStop(0.25, "#0d0c1d");
    rightWall.addColorStop(1, "#080712");
    ctx.fillStyle = rightWall;
    ctx.fillRect(winRight, 0, w - winRight, h);

    // Right window glass sash reflection
    ctx.fillStyle = "rgba(105, 115, 200, 0.12)";
    ctx.fillRect(winRight + 10, h * 0.04, w * 0.048, h * 0.92);
    ctx.strokeStyle = "#1f1c3b";
    ctx.lineWidth = 4;
    ctx.strokeRect(winRight + 8, h * 0.03, w * 0.052, h * 0.94);

    // Window handle detail on right frame
    ctx.fillStyle = "#201d3b";
    ctx.fillRect(winRight + w * 0.068, h * 0.48, 8, h * 0.1);
  }

  // --- Scene 2: Neon Rain Skyline ---
  private renderNeonSkyline(
    ctx: CanvasRenderingContext2D,
    w: number,
    h: number,
    dt: number
  ): void {
    const t = this.elapsedSec;
    const sky = ctx.createLinearGradient(0, 0, 0, h);
    sky.addColorStop(0, "#090c1e");
    sky.addColorStop(0.45, "#19153c");
    sky.addColorStop(0.78, "#3a1c4b");
    sky.addColorStop(1, "#120d26");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, w, h);

    // Distant neon horizon aura
    const glow = ctx.createRadialGradient(w * 0.52, h * 0.72, 20, w * 0.52, h * 0.72, w * 0.55);
    glow.addColorStop(0, "rgba(236, 72, 153, 0.28)");
    glow.addColorStop(0.45, "rgba(56, 189, 248, 0.14)");
    glow.addColorStop(1, "rgba(12, 12, 30, 0)");
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, w, h);

    // Layered skyline towers
    const towers = [
      { x: 0.03, w: 0.1, h: 0.48, col: "#121329", accent: "#38bdf8" },
      { x: 0.14, w: 0.08, h: 0.62, col: "#171434", accent: "#f472b6" },
      { x: 0.24, w: 0.12, h: 0.42, col: "#111026", accent: "#fbbf24" },
      { x: 0.39, w: 0.09, h: 0.68, col: "#161433", accent: "#38bdf8" },
      { x: 0.50, w: 0.13, h: 0.54, col: "#100f25", accent: "#f472b6" },
      { x: 0.66, w: 0.1, h: 0.64, col: "#181438", accent: "#a78bfa" },
      { x: 0.78, w: 0.09, h: 0.49, col: "#111026", accent: "#38bdf8" },
      { x: 0.89, w: 0.1, h: 0.58, col: "#15122e", accent: "#f472b6" },
    ];

    for (let i = 0; i < towers.length; i++) {
      const b = towers[i];
      const bx = b.x * w;
      const bw = b.w * w;
      const bh = b.h * h;
      const by = h - bh;
      ctx.fillStyle = b.col;
      ctx.fillRect(bx, by, bw, bh);

      // Neon rooftop strip
      const pulse = 0.65 + 0.35 * Math.sin(t * 1.2 + i);
      ctx.save();
      ctx.fillStyle = b.accent;
      ctx.globalAlpha = 0.75 * pulse;
      ctx.shadowColor = b.accent;
      ctx.shadowBlur = 16;
      ctx.fillRect(bx + 6, by + 6, bw - 12, 3);
      ctx.restore();

      // Tiny lit window dots
      ctx.fillStyle = "rgba(255, 232, 180, 0.55)";
      for (let r = 0; r < 10; r++) {
        for (let c = 0; c < 4; c++) {
          if (((r * 5 + c * 3 + i * 7) % 4) !== 0) continue;
          ctx.fillRect(bx + 10 + c * (bw / 4.6), by + 22 + r * 24, 7, 10);
        }
      }
    }

    this.renderRainStreaks(ctx, w, h, dt, Math.max(0.25, this.fx.rainOverlay), 0, w);
  }

  // --- Scene 3: Midnight Starlight ---
  private renderMidnightStarlight(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    const t = this.elapsedSec;
    const sky = ctx.createLinearGradient(0, 0, 0, h);
    sky.addColorStop(0, "#050816");
    sky.addColorStop(0.55, "#0e1938");
    sky.addColorStop(1, "#1e1a42");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, w, h);

    // Subtle nebula glow
    const neb = ctx.createRadialGradient(w * 0.65, h * 0.3, 10, w * 0.65, h * 0.3, w * 0.45);
    neb.addColorStop(0, "rgba(129, 140, 248, 0.2)");
    neb.addColorStop(0.5, "rgba(56, 189, 248, 0.08)");
    neb.addColorStop(1, "rgba(5, 8, 22, 0)");
    ctx.fillStyle = neb;
    ctx.fillRect(0, 0, w, h);

    // Twinkling stars
    for (const s of this.stars) {
      const a = 0.25 + 0.75 * (0.5 + 0.5 * Math.sin(t * s.speed + s.phase));
      ctx.fillStyle = `rgba(226, 236, 255, ${a.toFixed(2)})`;
      ctx.beginPath();
      ctx.arc(s.x * w, s.y * h, s.r, 0, Math.PI * 2);
      ctx.fill();
    }

    // Crescent moon + halo
    const mx = w * 0.26;
    const my = h * 0.24;
    const halo = ctx.createRadialGradient(mx, my, 12, mx, my, 120);
    halo.addColorStop(0, "rgba(224, 238, 255, 0.32)");
    halo.addColorStop(1, "rgba(224, 238, 255, 0)");
    ctx.fillStyle = halo;
    ctx.beginPath();
    ctx.arc(mx, my, 120, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = "#eef4ff";
    ctx.beginPath();
    ctx.arc(mx, my, 24, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#0a132c";
    ctx.beginPath();
    ctx.arc(mx + 9, my - 5, 21, 0, Math.PI * 2);
    ctx.fill();

    // Occasional gentle shooting star
    const cycle = t % 9;
    if (cycle < 1.1) {
      const p = cycle / 1.1;
      const sx = w * (0.78 - p * 0.32);
      const sy = h * (0.12 + p * 0.2);
      const grad = ctx.createLinearGradient(sx, sy, sx + 75, sy - 46);
      grad.addColorStop(0, `rgba(255, 255, 255, ${(1 - p) * 0.85})`);
      grad.addColorStop(1, "rgba(255, 255, 255, 0)");
      ctx.strokeStyle = grad;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(sx, sy);
      ctx.lineTo(sx + 75, sy - 46);
      ctx.stroke();
    }

    // Soft dark mountain silhouette
    ctx.fillStyle = "#090b1b";
    ctx.beginPath();
    ctx.moveTo(0, h);
    ctx.lineTo(0, h * 0.78);
    ctx.quadraticCurveTo(w * 0.22, h * 0.66, w * 0.45, h * 0.77);
    ctx.quadraticCurveTo(w * 0.72, h * 0.63, w, h * 0.76);
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fill();

    if (this.fx.rainOverlay > 0.05) {
      this.renderRainStreaks(ctx, w, h, 0.016, this.fx.rainOverlay * 0.6, 0, w);
    }
  }

  // --- Scene 4: Warm Study Dusk ---
  private renderWarmDusk(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    const t = this.elapsedSec;
    const sky = ctx.createLinearGradient(0, 0, 0, h);
    sky.addColorStop(0, "#1d1229");
    sky.addColorStop(0.42, "#512b42");
    sky.addColorStop(0.75, "#a45348");
    sky.addColorStop(1, "#e29a62");
    ctx.fillStyle = sky;
    ctx.fillRect(0, 0, w, h);

    // Setting sun glow
    const sun = ctx.createRadialGradient(w * 0.5, h * 0.72, 10, w * 0.5, h * 0.72, w * 0.35);
    sun.addColorStop(0, "rgba(255, 232, 178, 0.68)");
    sun.addColorStop(0.35, "rgba(251, 146, 60, 0.26)");
    sun.addColorStop(1, "rgba(251, 146, 60, 0)");
    ctx.fillStyle = sun;
    ctx.fillRect(0, 0, w, h);

    // Soft drifting dusk clouds
    ctx.fillStyle = "rgba(42, 20, 44, 0.36)";
    for (let i = 0; i < 4; i++) {
      const cx = ((w * (0.2 + i * 0.26) + t * (4 + i * 1.5)) % (w + 300)) - 150;
      const cy = h * (0.22 + (i % 3) * 0.13);
      ctx.beginPath();
      ctx.ellipse(cx, cy, 150, 26, 0, 0, Math.PI * 2);
      ctx.fill();
    }

    // Warm floating dust motes
    for (let i = 0; i < 36; i++) {
      const mx = ((i * 97.3 + Math.sin(t * 0.4 + i) * 32) % w + w) % w;
      const my = ((i * 53.7 - t * (6 + (i % 4) * 2)) % h + h) % h;
      ctx.fillStyle = `rgba(255, 232, 185, ${(0.22 + (i % 4) * 0.08).toFixed(2)})`;
      ctx.beginPath();
      ctx.arc(mx, my, 1.4 + (i % 3) * 0.7, 0, Math.PI * 2);
      ctx.fill();
    }

    // Distant warm rooftop silhouettes
    ctx.fillStyle = "#180d1d";
    ctx.fillRect(0, h * 0.82, w, h * 0.18);
    ctx.beginPath();
    ctx.moveTo(w * 0.06, h * 0.82);
    ctx.lineTo(w * 0.16, h * 0.71);
    ctx.lineTo(w * 0.26, h * 0.82);
    ctx.moveTo(w * 0.72, h * 0.82);
    ctx.lineTo(w * 0.83, h * 0.69);
    ctx.lineTo(w * 0.94, h * 0.82);
    ctx.fill();
  }

  private renderRainStreaks(
    ctx: CanvasRenderingContext2D,
    _w: number,
    h: number,
    dt: number,
    intensity: number,
    minX: number,
    maxX: number
  ): void {
    const count = Math.floor(this.raindrops.length * Math.min(1, intensity));
    const span = maxX - minX;
    ctx.save();
    ctx.lineCap = "round";

    for (let i = 0; i < count; i++) {
      const d = this.raindrops[i];
      d.y += d.speed * dt * (0.75 + intensity * 0.45);
      d.x += d.speed * dt * 0.085;
      if (d.y > 1.1) {
        d.y = -0.1;
        d.x = Math.random();
      }
      if (d.x > 1.05) d.x -= 1.1;

      const x = minX + d.x * span;
      const y = d.y * h;
      const dx = d.len * h * 0.11;
      const dy = d.len * h;

      ctx.strokeStyle = `rgba(196, 206, 245, ${(d.alpha * intensity).toFixed(3)})`;
      ctx.lineWidth = d.width;
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x + dx, y + dy);
      ctx.stroke();
    }
    ctx.restore();
  }

  private renderPostFx(ctx: CanvasRenderingContext2D, w: number, h: number): void {
    if (this.fx.warmTint > 0.01) {
      ctx.fillStyle = `rgba(255, 166, 92, ${(this.fx.warmTint * 0.18).toFixed(3)})`;
      ctx.fillRect(0, 0, w, h);
    }

    if (this.fx.vignette > 0.01) {
      const vig = ctx.createRadialGradient(
        w * 0.5,
        h * 0.5,
        Math.min(w, h) * 0.25,
        w * 0.5,
        h * 0.5,
        Math.max(w, h) * 0.72
      );
      vig.addColorStop(0, "rgba(0, 0, 0, 0)");
      vig.addColorStop(1, `rgba(4, 5, 12, ${(this.fx.vignette * 0.72).toFixed(3)})`);
      ctx.fillStyle = vig;
      ctx.fillRect(0, 0, w, h);
    }

    if (this.fx.filmGrain > 0.02) {
      ctx.save();
      ctx.globalAlpha = this.fx.filmGrain * 0.45;
      const pat = ctx.createPattern(this.grainPatternCanvas, "repeat");
      if (pat) {
        ctx.translate(Math.random() * 24 - 12, Math.random() * 24 - 12);
        ctx.fillStyle = pat;
        ctx.fillRect(-24, -24, w + 48, h + 48);
      }
      ctx.restore();
    }
  }
}
