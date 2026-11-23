;; Example Lofi Desktop WebAssembly Plugin (ABI v1)
;; Drop this .wat file (or its compiled .wasm) into your local plugins/ directory.
(module
  (memory (export "memory") 1)
  (data (i32.const 1024) "{\"id\":\"wasm-rainy-lofi-example\",\"name\":\"Rainy Cassette Example\",\"version\":\"1.0.0\",\"author\":\"Community\",\"description\":\"Example WebAssembly Text (.wat) plugin that boosts cozy sub-bass and rolls off harsh highs.\",\"capabilities\":[\"eq-dsp\",\"eq-presets\"],\"eqPresets\":[{\"id\":\"wasm-rainy-example-eq\",\"name\":\"Rainy Cassette (WAT)\",\"bands\":[3.2,1.6,-0.5,-2.4,-4.8],\"tapeCutoffHz\":5000}],\"transformNote\":\"Applied Rainy Cassette curve from .wat plugin\"}")

  (func (export "lofi_abi_version") (result i32)
    (i32.const 1)
  )

  (func (export "lofi_metadata_ptr") (result i32)
    (i32.const 1024)
  )

  (func (export "lofi_metadata_len") (result i32)
    (i32.const 442)
  )

  (func (export "lofi_result_ptr") (result i32)
    (i32.const 256)
  )

  (func (export "lofi_transform_eq")
    (param $b0 f32) (param $b1 f32) (param $b2 f32) (param $b3 f32) (param $b4 f32) (param $k f32)
    (result f32)
    (f32.store (i32.const 256) (f32.add (local.get $b0) (f32.mul (f32.const 3.2) (local.get $k))))
    (f32.store (i32.const 260) (f32.add (local.get $b1) (f32.mul (f32.const 1.6) (local.get $k))))
    (f32.store (i32.const 264) (f32.add (local.get $b2) (f32.mul (f32.const -0.5) (local.get $k))))
    (f32.store (i32.const 268) (f32.add (local.get $b3) (f32.mul (f32.const -2.4) (local.get $k))))
    (f32.store (i32.const 272) (f32.add (local.get $b4) (f32.mul (f32.const -4.8) (local.get $k))))
    (f32.sub (f32.const 18500.0) (f32.mul (f32.const 13500.0) (local.get $k)))
  )

  (func (export "lofi_on_event") (param $kind i32) (param $value i32) (result i32)
    (i32.add (local.get $kind) (local.get $value))
  )
)
