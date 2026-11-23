use crate::models::{
    EqPreset, PluginEqTransformResult, PluginKind, PluginMetadata, PluginSceneContribution,
};
use crate::plugins::{LofiPlugin, PluginError, PluginEvent};
use std::collections::HashMap;

/// Built-in native plugin implementing the exact same `LofiPlugin` trait as
/// dynamically loaded WebAssembly modules.
pub struct CoreAtmospherePlugin;

impl CoreAtmospherePlugin {
    pub fn new() -> Self {
        Self
    }
}

impl LofiPlugin for CoreAtmospherePlugin {
    fn id(&self) -> &str {
        "core-atmosphere"
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: self.id().to_string(),
            name: "Core Atmosphere".to_string(),
            version: "1.0.0".to_string(),
            author: "Lofi Desktop".to_string(),
            description: "Native Rust acoustic presets and cozy desk scenes linked via the unified LofiPlugin trait.".to_string(),
            kind: PluginKind::Native,
            file_path: None,
            capabilities: vec![
                "eq-presets".to_string(),
                "eq-dsp".to_string(),
                "scene-presets".to_string(),
            ],
            enabled: true,
        }
    }

    fn eq_presets(&self) -> Vec<EqPreset> {
        vec![
            EqPreset {
                id: "core-cassette-haze".to_string(),
                name: "Cassette Haze".to_string(),
                bands: [2.5, 1.8, -0.8, -2.5, -4.2],
                tape_cutoff_hz: 5600.0,
                source: "plugin:core-atmosphere".to_string(),
            },
        ]
    }

    fn scene_contributions(&self) -> Vec<PluginSceneContribution> {
        let mut ambience = HashMap::new();
        ambience.insert("rain".to_string(), 0.35);
        ambience.insert("cafe".to_string(), 0.25);

        vec![PluginSceneContribution {
            id: "plugin-starry-attic".to_string(),
            name: "Starry Attic Loft".to_string(),
            mood: "DREAMING".to_string(),
            description: "Quiet midnight skylight with distant city hum and velvety low-pass warmth.".to_string(),
            visual_id: "midnight-starlight".to_string(),
            eq_bands: [2.5, 1.8, -0.8, -2.5, -4.2],
            tape_cutoff_hz: 5600.0,
            ambience,
            accent_color: "#818cf8".to_string(),
            plugin_id: self.id().to_string(),
        }]
    }

    fn transform_eq(
        &self,
        bands: [f32; 5],
        intensity: f32,
    ) -> Result<PluginEqTransformResult, PluginError> {
        let k = intensity.clamp(0.0, 1.0);
        let target = [2.2, 1.6, -0.6, -2.2, -3.8];
        let mut out = [0.0f32; 5];
        for i in 0..5 {
            out[i] = ((bands[i] + target[i] * k).clamp(-12.0, 12.0) * 10.0).round() / 10.0;
        }
        let cutoff = 18000.0 - k * 12200.0;
        Ok(PluginEqTransformResult {
            plugin_id: self.id().to_string(),
            plugin_name: "Core Atmosphere".to_string(),
            input_bands: bands,
            output_bands: out,
            suggested_tape_cutoff_hz: cutoff.clamp(2400.0, 20000.0),
            note: format!("Applied native cassette warmth ({:.0}%)", k * 100.0),
        })
    }

    fn on_event(&self, _event: &PluginEvent) -> Result<i32, PluginError> {
        Ok(1)
    }
}
