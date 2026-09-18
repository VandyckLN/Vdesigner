# Plano de Implementação — Motor e Editor (Vdesigner, parte 1 de 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Entregar um aplicativo Windows instalável que abre uma imagem, aplica redimensionamento e filtros de qualidade com prévia ao vivo, e exporta no formato escolhido.

**Architecture:** Workspace Cargo com a crate pura `vdesigner-core` guardando toda a lógica de imagem, e uma camada Tauri fina que só traduz comandos da interface React em chamadas ao motor. Uma operação é a estrutura de dados `Job`, executada pelo mesmo caminho de código tanto na prévia reduzida quanto na exportação em resolução cheia.

**Tech Stack:** Rust 1.82, Tauri 2, React 18, TypeScript 5, Vite 5. Bibliotecas: `image`, `fast_image_resize`, `imageproc`, `resvg`, `webp`, `ravif`, `thiserror`.

**Spec:** `docs/superpowers/specs/2026-09-17-vdesigner-design.md`

## Global Constraints

- Plataforma alvo: Windows 10 versão 1809 ou superior, 64 bits.
- Rust: edição 2021, versão mínima 1.82. Fixada em `rust-toolchain.toml`.
- Node: 20 LTS ou superior.
- Licença: MIT. Toda dependência nova entra em `THIRD-PARTY.md` na mesma tarefa que a introduz.
- `vdesigner-core` não pode depender de `tauri`, de `windows`, nem de qualquer crate de interface. Essa regra é verificada por teste na Tarefa 1.
- Nenhuma decisão sobre imagem em `src-tauri/` ou em `ui/`. Essas camadas só transportam dados.
- Todo erro público do motor é a enum `CoreError`. Nada de `unwrap` ou `expect` em código de produção; apenas em testes.
- Nomes de tipos, funções e comentários em inglês. Textos de interface em português do Brasil.
- Cada tarefa termina com commit. Mensagem em inglês, prefixo convencional (`feat:`, `test:`, `chore:`, `fix:`, `docs:`).

---

## Estrutura de Arquivos

```
Cargo.toml                          workspace
rust-toolchain.toml                 fixa a versão do Rust
.github/workflows/ci.yml            fmt, clippy, test em windows-latest
THIRD-PARTY.md                      licenças das dependências

crates/core/Cargo.toml
crates/core/src/lib.rs              reexporta a API pública
crates/core/src/error.rs            CoreError
crates/core/src/decode.rs           bytes -> DynamicImage
crates/core/src/encode.rs           DynamicImage -> bytes
crates/core/src/resize.rs           Lanczos3 e modos de encaixe
crates/core/src/filters.rs          sharpen, denoise, ajustes
crates/core/src/svg.rs              rasterização de SVG
crates/core/src/job.rs              Job, Step, run_job, run_preview
crates/core/tests/common/mod.rs     geradores de imagem para teste
crates/core/tests/fixtures/         arquivos de amostra versionados

src-tauri/Cargo.toml
src-tauri/tauri.conf.json
src-tauri/src/main.rs               bootstrap
src-tauri/src/commands.rs           comandos expostos à interface
src-tauri/src/session.rs            imagem carregada na memória
src-tauri/src/export.rs             escrita em disco e proteção de sobrescrita

ui/package.json
ui/src/main.tsx
ui/src/App.tsx
ui/src/api.ts                       tipos espelhando o motor, invoke tipado
ui/src/screens/Editor.tsx
ui/src/components/PreviewPane.tsx
ui/src/components/ResizeControls.tsx
ui/src/components/QualityControls.tsx
ui/src/components/ExportBar.tsx
```

Responsabilidade por arquivo: cada módulo do motor resolve uma etapa do pipeline e nada mais. `job.rs` é o único que conhece a ordem das etapas. `commands.rs` é o único que conhece o Tauri.

---

### Task 1: Workspace, crate do motor e decodificação

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `THIRD-PARTY.md`
- Create: `.github/workflows/ci.yml`
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`, `crates/core/src/error.rs`, `crates/core/src/decode.rs`
- Test: `crates/core/tests/decode_test.rs`, `crates/core/tests/common/mod.rs`

**Interfaces:**
- Consumes: nada.
- Produces: `CoreError`, `InputFormat`, `decode(bytes: &[u8]) -> Result<DynamicImage, CoreError>`, `detect_format(bytes: &[u8]) -> Result<InputFormat, CoreError>`.

- [ ] **Step 1: Criar o esqueleto do workspace**

`Cargo.toml` na raiz:

```toml
[workspace]
members = ["crates/core"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.82"
license = "MIT"

[workspace.dependencies]
image = { version = "0.25", default-features = false, features = ["jpeg", "png", "gif", "bmp", "tiff", "webp", "ico"] }
thiserror = "2"
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.82"
components = ["rustfmt", "clippy"]
```

`.gitignore`:

```
/target
/ui/node_modules
/ui/dist
/src-tauri/target
```

`crates/core/Cargo.toml`:

```toml
[package]
name = "vdesigner-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
image.workspace = true
thiserror.workspace = true
```

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/common/mod.rs`:

```rust
use image::{DynamicImage, Rgba, RgbaImage};

/// Builds a deterministic gradient image, so tests never depend on binary fixtures.
pub fn gradient(width: u32, height: u32) -> DynamicImage {
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x * 255 / width.max(1)) as u8;
        let g = (y * 255 / height.max(1)) as u8;
        *pixel = Rgba([r, g, 128, 255]);
    }
    DynamicImage::ImageRgba8(img)
}

/// Encodes an image with the `image` crate, used only to produce test input bytes.
pub fn as_png(img: &DynamicImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("png encoding in test helper must not fail");
    bytes
}
```

`crates/core/tests/decode_test.rs`:

```rust
mod common;

use vdesigner_core::{decode, detect_format, CoreError, InputFormat};

#[test]
fn detects_png_from_magic_bytes() {
    let bytes = common::as_png(&common::gradient(8, 8));
    assert_eq!(detect_format(&bytes).unwrap(), InputFormat::Png);
}

#[test]
fn decodes_png_preserving_dimensions() {
    let bytes = common::as_png(&common::gradient(32, 16));
    let img = decode(&bytes).unwrap();
    assert_eq!((img.width(), img.height()), (32, 16));
}

#[test]
fn rejects_unknown_bytes_with_unsupported_error() {
    let err = decode(b"not an image at all").unwrap_err();
    assert!(matches!(err, CoreError::UnsupportedFormat));
}

#[test]
fn rejects_truncated_png_with_decode_error() {
    let bytes = common::as_png(&common::gradient(8, 8));
    let truncated = &bytes[..bytes.len() / 2];
    assert!(matches!(decode(truncated), Err(CoreError::Decode(_))));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core`
Expected: FAIL na compilação, com `unresolved import vdesigner_core::decode`.

- [ ] **Step 4: Implementar o erro**

`crates/core/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("formato de arquivo não suportado")]
    UnsupportedFormat,

    #[error("falha ao decodificar a imagem: {0}")]
    Decode(String),

    #[error("falha ao codificar a imagem: {0}")]
    Encode(String),

    #[error("parâmetro inválido: {0}")]
    InvalidParameter(String),
}
```

- [ ] **Step 5: Implementar a detecção e a decodificação**

`crates/core/src/decode.rs`:

```rust
use crate::error::CoreError;
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Tiff,
}

/// Identifies the format from the leading bytes, never from the file extension.
pub fn detect_format(bytes: &[u8]) -> Result<InputFormat, CoreError> {
    match image::guess_format(bytes) {
        Ok(image::ImageFormat::Jpeg) => Ok(InputFormat::Jpeg),
        Ok(image::ImageFormat::Png) => Ok(InputFormat::Png),
        Ok(image::ImageFormat::WebP) => Ok(InputFormat::WebP),
        Ok(image::ImageFormat::Gif) => Ok(InputFormat::Gif),
        Ok(image::ImageFormat::Bmp) => Ok(InputFormat::Bmp),
        Ok(image::ImageFormat::Tiff) => Ok(InputFormat::Tiff),
        _ => Err(CoreError::UnsupportedFormat),
    }
}

/// Decodes raster input into RGBA8, the single working representation of the engine.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage, CoreError> {
    let format = detect_format(bytes)?;
    let reader = image::ImageReader::with_format(
        std::io::Cursor::new(bytes),
        to_image_format(format),
    );
    let decoded = reader
        .decode()
        .map_err(|e| CoreError::Decode(e.to_string()))?;
    Ok(DynamicImage::ImageRgba8(decoded.to_rgba8()))
}

fn to_image_format(format: InputFormat) -> image::ImageFormat {
    match format {
        InputFormat::Jpeg => image::ImageFormat::Jpeg,
        InputFormat::Png => image::ImageFormat::Png,
        InputFormat::WebP => image::ImageFormat::WebP,
        InputFormat::Gif => image::ImageFormat::Gif,
        InputFormat::Bmp => image::ImageFormat::Bmp,
        InputFormat::Tiff => image::ImageFormat::Tiff,
    }
}
```

`crates/core/src/lib.rs`:

```rust
//! Pure image engine. Knows nothing about Tauri, about Windows, or about any UI.

mod decode;
mod error;

pub use decode::{decode, detect_format, InputFormat};
pub use error::CoreError;
```

- [ ] **Step 6: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 4 testes.

- [ ] **Step 7: Adicionar o teste que protege a pureza da crate**

`crates/core/tests/purity_test.rs`:

```rust
/// The engine must never gain a UI or platform dependency. This test reads the
/// manifest as text, which is enough to catch an accidental addition in review.
#[test]
fn core_manifest_has_no_ui_or_platform_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in ["tauri", "windows", "winapi", "wry"] {
        assert!(
            !manifest.contains(forbidden),
            "vdesigner-core não pode depender de `{forbidden}`"
        );
    }
}
```

Run: `cargo test -p vdesigner-core`
Expected: PASS, 5 testes.

- [ ] **Step 8: Adicionar a integração contínua**

`.github/workflows/ci.yml`:

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

jobs:
  rust:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.82
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
```

- [ ] **Step 9: Registrar as licenças**

`THIRD-PARTY.md`:

```markdown
# Dependências de terceiros

| Pacote | Licença | Uso |
|--------|---------|-----|
| image | MIT OR Apache-2.0 | decodificação e codificação de formatos raster |
| thiserror | MIT OR Apache-2.0 | definição de erros |
```

- [ ] **Step 10: Rodar a verificação completa e commitar**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all`
Expected: tudo passa, sem aviso.

```bash
git add Cargo.toml rust-toolchain.toml .gitignore THIRD-PARTY.md .github crates
git commit -m "feat: add cargo workspace and raster image decoding"
```

---

### Task 2: Codificação de saída

**Files:**
- Create: `crates/core/src/encode.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, `Cargo.toml`, `THIRD-PARTY.md`
- Test: `crates/core/tests/encode_test.rs`

**Interfaces:**
- Consumes: `decode`, `CoreError` da Tarefa 1.
- Produces: `OutputFormat`, `EncodeSpec { format, quality, lossless }`, `encode(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError>`.

Nota técnica para quem implementa: o crate `image` 0.25 decodifica WebP mas não codifica, por isso entra o crate `webp`, que embrulha a libwebp em C. AVIF é codificado por `ravif`, que é Rust puro. PNG, JPEG, TIFF e ICO ficam com o `image`.

- [ ] **Step 1: Declarar as dependências novas**

Em `Cargo.toml` da raiz, dentro de `[workspace.dependencies]`:

```toml
webp = "0.3"
ravif = "0.11"
rgb = "0.8"
```

Em `crates/core/Cargo.toml`, dentro de `[dependencies]`:

```toml
webp.workspace = true
ravif.workspace = true
rgb.workspace = true
```

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/encode_test.rs`:

```rust
mod common;

use vdesigner_core::{decode, detect_format, encode, CoreError, EncodeSpec, InputFormat, OutputFormat};

fn spec(format: OutputFormat, quality: u8) -> EncodeSpec {
    EncodeSpec { format, quality, lossless: false }
}

#[test]
fn encodes_webp_that_decodes_back_to_the_same_size() {
    let img = common::gradient(64, 48);
    let bytes = encode(&img, &spec(OutputFormat::WebP, 82)).unwrap();
    assert_eq!(detect_format(&bytes).unwrap(), InputFormat::WebP);
    let round_trip = decode(&bytes).unwrap();
    assert_eq!((round_trip.width(), round_trip.height()), (64, 48));
}

#[test]
fn encodes_png_losslessly() {
    let img = common::gradient(16, 16);
    let bytes = encode(&img, &spec(OutputFormat::Png, 100)).unwrap();
    let round_trip = decode(&bytes).unwrap();
    assert_eq!(round_trip.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn lower_quality_produces_smaller_jpeg() {
    let img = common::gradient(256, 256);
    let high = encode(&img, &spec(OutputFormat::Jpeg, 95)).unwrap();
    let low = encode(&img, &spec(OutputFormat::Jpeg, 40)).unwrap();
    assert!(low.len() < high.len(), "qualidade menor deve gerar arquivo menor");
}

#[test]
fn encodes_avif() {
    let img = common::gradient(32, 32);
    let bytes = encode(&img, &spec(OutputFormat::Avif, 60)).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn encodes_ico_and_tiff() {
    let img = common::gradient(32, 32);
    assert!(!encode(&img, &spec(OutputFormat::Ico, 100)).unwrap().is_empty());
    assert!(!encode(&img, &spec(OutputFormat::Tiff, 100)).unwrap().is_empty());
}

#[test]
fn rejects_ico_larger_than_256_pixels() {
    let img = common::gradient(512, 512);
    let err = encode(&img, &spec(OutputFormat::Ico, 100)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn rejects_quality_above_one_hundred() {
    let img = common::gradient(8, 8);
    let err = encode(&img, &spec(OutputFormat::Jpeg, 120)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core --test encode_test`
Expected: FAIL na compilação, `unresolved import vdesigner_core::encode`.

- [ ] **Step 4: Implementar a codificação**

`crates/core/src/encode.rs`:

```rust
use crate::error::CoreError;
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    WebP,
    Avif,
    Png,
    Jpeg,
    Tiff,
    Ico,
}

#[derive(Debug, Clone, Copy)]
pub struct EncodeSpec {
    pub format: OutputFormat,
    /// 1 to 100. Ignored by the lossless formats.
    pub quality: u8,
    /// Only honoured by WebP, which supports both modes.
    pub lossless: bool,
}

/// The ICO container stores at most 256 pixels per side.
const ICO_MAX_SIDE: u32 = 256;

pub fn encode(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    if spec.quality == 0 || spec.quality > 100 {
        return Err(CoreError::InvalidParameter(format!(
            "qualidade deve ficar entre 1 e 100, recebido {}",
            spec.quality
        )));
    }

    match spec.format {
        OutputFormat::WebP => encode_webp(img, spec),
        OutputFormat::Avif => encode_avif(img, spec),
        OutputFormat::Ico => {
            if img.width() > ICO_MAX_SIDE || img.height() > ICO_MAX_SIDE {
                return Err(CoreError::InvalidParameter(format!(
                    "ICO aceita no máximo {ICO_MAX_SIDE} pixels por lado, recebido {}x{}",
                    img.width(),
                    img.height()
                )));
            }
            encode_with_image_crate(img, image::ImageFormat::Ico)
        }
        OutputFormat::Png => encode_with_image_crate(img, image::ImageFormat::Png),
        OutputFormat::Tiff => encode_with_image_crate(img, image::ImageFormat::Tiff),
        OutputFormat::Jpeg => encode_jpeg(img, spec.quality),
    }
}

fn encode_with_image_crate(
    img: &DynamicImage,
    format: image::ImageFormat,
) -> Result<Vec<u8>, CoreError> {
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), format)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(bytes)
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, CoreError> {
    // JPEG has no alpha channel, so the image is flattened over white first.
    let rgb = DynamicImage::ImageRgb8(img.to_rgb8());
    let mut bytes = Vec::new();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::Cursor::new(&mut bytes), quality);
    encoder
        .encode_image(&rgb)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(bytes)
}

fn encode_webp(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    let rgba = img.to_rgba8();
    let encoder = webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height());
    let memory = if spec.lossless {
        encoder.encode_lossless()
    } else {
        encoder.encode(spec.quality as f32)
    };
    Ok(memory.to_vec())
}

fn encode_avif(img: &DynamicImage, spec: &EncodeSpec) -> Result<Vec<u8>, CoreError> {
    let rgba = img.to_rgba8();
    let pixels: Vec<rgb::RGBA8> = rgba
        .pixels()
        .map(|p| rgb::RGBA8::new(p.0[0], p.0[1], p.0[2], p.0[3]))
        .collect();
    let buffer = ravif::Img::new(pixels.as_slice(), img.width() as usize, img.height() as usize);
    let encoded = ravif::Encoder::new()
        .with_quality(spec.quality as f32)
        .with_speed(6)
        .encode_rgba(buffer)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(encoded.avif_file)
}
```

Em `crates/core/src/lib.rs`, acrescentar:

```rust
mod encode;
pub use encode::{encode, EncodeSpec, OutputFormat};
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 12 testes.

- [ ] **Step 6: Atualizar as licenças e commitar**

Acrescentar em `THIRD-PARTY.md`:

```markdown
| webp | BSD-3-Clause (libwebp) | codificação WebP |
| ravif | BSD-3-Clause | codificação AVIF |
| rgb | MIT | conversão de pixels para o ravif |
```

```bash
git add Cargo.toml THIRD-PARTY.md crates/core
git commit -m "feat: add output encoding for webp, avif, png, jpeg, tiff and ico"
```

---

### Task 3: Redimensionamento sem distorção

**Files:**
- Create: `crates/core/src/resize.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, `Cargo.toml`, `THIRD-PARTY.md`
- Test: `crates/core/tests/resize_test.rs`

**Interfaces:**
- Consumes: `CoreError` da Tarefa 1.
- Produces: `FitMode { Contain, Cover, Stretch }`, `ResizeSpec { width, height, fit, pad_color }`, `resize(img: &DynamicImage, spec: &ResizeSpec) -> Result<DynamicImage, CoreError>`.

- [ ] **Step 1: Declarar a dependência**

Em `Cargo.toml` da raiz:

```toml
fast_image_resize = { version = "5", features = ["image"] }
```

Em `crates/core/Cargo.toml`:

```toml
fast_image_resize.workspace = true
```

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/resize_test.rs`:

```rust
mod common;

use vdesigner_core::{resize, CoreError, FitMode, ResizeSpec};

fn spec(width: Option<u32>, height: Option<u32>, fit: FitMode) -> ResizeSpec {
    ResizeSpec { width, height, fit, pad_color: [0, 0, 0, 0] }
}

#[test]
fn width_only_keeps_the_aspect_ratio() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(200), None, FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (200, 100));
}

#[test]
fn height_only_keeps_the_aspect_ratio() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(None, Some(50), FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 50));
}

#[test]
fn contain_pads_to_the_exact_target_without_cropping() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));

    // The source is wider than tall, so the top row must be transparent padding.
    let rgba = out.to_rgba8();
    assert_eq!(rgba.get_pixel(50, 0).0[3], 0, "topo deve ser preenchimento");
    assert_ne!(rgba.get_pixel(50, 50).0[3], 0, "centro deve ser imagem");
}

#[test]
fn cover_fills_the_target_and_crops_the_overflow() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Cover)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));

    let rgba = out.to_rgba8();
    for pixel in rgba.pixels() {
        assert_eq!(pixel.0[3], 255, "cover não deve deixar preenchimento");
    }
}

#[test]
fn stretch_distorts_on_purpose() {
    let img = common::gradient(400, 200);
    let out = resize(&img, &spec(Some(100), Some(100), FitMode::Stretch)).unwrap();
    assert_eq!((out.width(), out.height()), (100, 100));
}

#[test]
fn rejects_a_spec_without_any_dimension() {
    let img = common::gradient(10, 10);
    let err = resize(&img, &spec(None, None, FitMode::Contain)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn rejects_zero_as_a_dimension() {
    let img = common::gradient(10, 10);
    let err = resize(&img, &spec(Some(0), None, FitMode::Contain)).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}

#[test]
fn upscaling_is_allowed() {
    let img = common::gradient(10, 10);
    let out = resize(&img, &spec(Some(40), None, FitMode::Contain)).unwrap();
    assert_eq!((out.width(), out.height()), (40, 40));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core --test resize_test`
Expected: FAIL na compilação.

- [ ] **Step 4: Implementar o redimensionamento**

`crates/core/src/resize.rs`:

```rust
use crate::error::CoreError;
use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// Fits inside the target and pads the remaining area. Never crops.
    Contain,
    /// Fills the target and crops the overflow, centred.
    Cover,
    /// Ignores the aspect ratio. Only reachable when the user unlocks it.
    Stretch,
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeSpec {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: FitMode,
    /// RGBA padding colour, used by `Contain` only.
    pub pad_color: [u8; 4],
}

pub fn resize(img: &DynamicImage, spec: &ResizeSpec) -> Result<DynamicImage, CoreError> {
    validate(spec)?;

    let (src_w, src_h) = (img.width(), img.height());
    let (target_w, target_h) = target_size(src_w, src_h, spec);

    match spec.fit {
        FitMode::Stretch => scale(img, target_w, target_h),
        FitMode::Contain => {
            // With a single dimension given, the target already has the source ratio,
            // so scaling alone produces the answer and no padding is needed.
            let (inner_w, inner_h) = fit_inside(src_w, src_h, target_w, target_h);
            let scaled = scale(img, inner_w, inner_h)?;
            if (inner_w, inner_h) == (target_w, target_h) {
                return Ok(scaled);
            }
            Ok(pad_centered(&scaled, target_w, target_h, spec.pad_color))
        }
        FitMode::Cover => {
            let (outer_w, outer_h) = fill_outside(src_w, src_h, target_w, target_h);
            let scaled = scale(img, outer_w, outer_h)?;
            Ok(crop_centered(&scaled, target_w, target_h))
        }
    }
}

fn validate(spec: &ResizeSpec) -> Result<(), CoreError> {
    if spec.width.is_none() && spec.height.is_none() {
        return Err(CoreError::InvalidParameter(
            "informe ao menos largura ou altura".into(),
        ));
    }
    for value in [spec.width, spec.height].into_iter().flatten() {
        if value == 0 {
            return Err(CoreError::InvalidParameter(
                "as dimensões devem ser maiores que zero".into(),
            ));
        }
    }
    if spec.fit == FitMode::Stretch && (spec.width.is_none() || spec.height.is_none()) {
        return Err(CoreError::InvalidParameter(
            "o modo Stretch exige largura e altura".into(),
        ));
    }
    Ok(())
}

/// Resolves the requested box. A missing dimension is derived from the source ratio.
fn target_size(src_w: u32, src_h: u32, spec: &ResizeSpec) -> (u32, u32) {
    match (spec.width, spec.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, scale_dimension(w, src_h, src_w)),
        (None, Some(h)) => (scale_dimension(h, src_w, src_h), h),
        (None, None) => (src_w, src_h),
    }
}

fn scale_dimension(known: u32, other_src: u32, known_src: u32) -> u32 {
    let value = (known as u64 * other_src as u64) / known_src.max(1) as u64;
    value.max(1) as u32
}

fn fit_inside(src_w: u32, src_h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let ratio = f64::min(box_w as f64 / src_w as f64, box_h as f64 / src_h as f64);
    (
        ((src_w as f64 * ratio).round() as u32).max(1),
        ((src_h as f64 * ratio).round() as u32).max(1),
    )
}

fn fill_outside(src_w: u32, src_h: u32, box_w: u32, box_h: u32) -> (u32, u32) {
    let ratio = f64::max(box_w as f64 / src_w as f64, box_h as f64 / src_h as f64);
    (
        ((src_w as f64 * ratio).round() as u32).max(box_w),
        ((src_h as f64 * ratio).round() as u32).max(box_h),
    )
}

/// Lanczos3 rescale. This is the only place that touches the resampling library.
fn scale(img: &DynamicImage, width: u32, height: u32) -> Result<DynamicImage, CoreError> {
    let src = img.to_rgba8();
    let source = FirImage::from_vec_u8(img.width(), img.height(), src.into_raw(), PixelType::U8x4)
        .map_err(|e| CoreError::InvalidParameter(e.to_string()))?;
    let mut destination = FirImage::new(width, height, PixelType::U8x4);

    Resizer::new()
        .resize(
            &source,
            &mut destination,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3)),
        )
        .map_err(|e| CoreError::InvalidParameter(e.to_string()))?;

    let buffer = RgbaImage::from_raw(width, height, destination.into_vec())
        .ok_or_else(|| CoreError::Encode("buffer redimensionado inválido".into()))?;
    Ok(DynamicImage::ImageRgba8(buffer))
}

fn pad_centered(img: &DynamicImage, width: u32, height: u32, color: [u8; 4]) -> DynamicImage {
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba(color));
    let x = (width.saturating_sub(img.width())) / 2;
    let y = (height.saturating_sub(img.height())) / 2;
    image::imageops::overlay(&mut canvas, &img.to_rgba8(), x as i64, y as i64);
    DynamicImage::ImageRgba8(canvas)
}

fn crop_centered(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    let x = (img.width().saturating_sub(width)) / 2;
    let y = (img.height().saturating_sub(height)) / 2;
    img.crop_imm(x, y, width, height)
}
```

Em `crates/core/src/lib.rs`, acrescentar:

```rust
mod resize;
pub use resize::{resize, FitMode, ResizeSpec};
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 20 testes.

- [ ] **Step 6: Atualizar licenças e commitar**

Acrescentar em `THIRD-PARTY.md`:

```markdown
| fast_image_resize | MIT OR Apache-2.0 | redimensionamento Lanczos3 com SIMD |
```

```bash
git add Cargo.toml THIRD-PARTY.md crates/core
git commit -m "feat: add lanczos3 resize with contain, cover and stretch modes"
```

---

### Task 4: Filtros de qualidade

**Files:**
- Create: `crates/core/src/filters.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, `Cargo.toml`, `THIRD-PARTY.md`
- Test: `crates/core/tests/filters_test.rs`

**Interfaces:**
- Consumes: `CoreError` da Tarefa 1.
- Produces: `SharpenSpec { amount, radius }`, `DenoiseSpec { radius }`, `AdjustSpec { brightness, contrast, saturation }`, e as funções `sharpen`, `denoise`, `adjust`, todas com assinatura `(&DynamicImage, &Spec) -> Result<DynamicImage, CoreError>`.

- [ ] **Step 1: Declarar a dependência**

Em `Cargo.toml` da raiz:

```toml
imageproc = "0.25"
```

Em `crates/core/Cargo.toml`:

```toml
imageproc.workspace = true
```

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/filters_test.rs`:

```rust
mod common;

use image::{DynamicImage, Rgba, RgbaImage};
use vdesigner_core::{adjust, denoise, sharpen, AdjustSpec, CoreError, DenoiseSpec, SharpenSpec};

/// Mean absolute difference between neighbouring pixels along a row.
/// Sharpening raises it; denoising lowers it. This gives the tests a real
/// property to assert instead of comparing against a magic byte blob.
fn local_contrast(img: &DynamicImage) -> f64 {
    let rgba = img.to_rgba8();
    let mut total = 0f64;
    let mut count = 0u64;
    for y in 0..rgba.height() {
        for x in 1..rgba.width() {
            let a = rgba.get_pixel(x - 1, y).0[0] as f64;
            let b = rgba.get_pixel(x, y).0[0] as f64;
            total += (a - b).abs();
            count += 1;
        }
    }
    total / count.max(1) as f64
}

/// A checkerboard has strong local contrast, so filters show a clear effect.
fn checkerboard(size: u32) -> DynamicImage {
    let mut img = RgbaImage::new(size, size);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let value = if (x + y) % 2 == 0 { 20 } else { 235 };
        *pixel = Rgba([value, value, value, 255]);
    }
    DynamicImage::ImageRgba8(img)
}

#[test]
fn denoise_reduces_local_contrast() {
    let img = checkerboard(32);
    let before = local_contrast(&img);
    let after = local_contrast(&denoise(&img, &DenoiseSpec { radius: 1 }).unwrap());
    assert!(after < before, "denoise deveria suavizar: {after} < {before}");
}

#[test]
fn sharpen_increases_local_contrast_on_a_soft_image() {
    let soft = denoise(&checkerboard(32), &DenoiseSpec { radius: 2 }).unwrap();
    let before = local_contrast(&soft);
    let after = local_contrast(&sharpen(&soft, &SharpenSpec { amount: 1.5, radius: 1.0 }).unwrap());
    assert!(after > before, "sharpen deveria realçar: {after} > {before}");
}

#[test]
fn zero_amount_sharpen_returns_the_image_unchanged() {
    let img = common::gradient(16, 16);
    let out = sharpen(&img, &SharpenSpec { amount: 0.0, radius: 1.0 }).unwrap();
    assert_eq!(out.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn zero_radius_denoise_returns_the_image_unchanged() {
    let img = common::gradient(16, 16);
    let out = denoise(&img, &DenoiseSpec { radius: 0 }).unwrap();
    assert_eq!(out.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn brightness_raises_every_channel() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([100, 100, 100, 255])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.2, contrast: 0.0, saturation: 0.0 }).unwrap();
    assert!(out.to_rgba8().get_pixel(0, 0).0[0] > 100);
}

#[test]
fn saturation_of_minus_one_produces_gray() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([200, 40, 40, 255])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.0, contrast: 0.0, saturation: -1.0 }).unwrap();
    let pixel = out.to_rgba8().get_pixel(0, 0).0;
    assert!(
        pixel[0].abs_diff(pixel[1]) <= 2 && pixel[1].abs_diff(pixel[2]) <= 2,
        "esperava cinza, recebeu {pixel:?}"
    );
}

#[test]
fn adjust_preserves_alpha() {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([10, 20, 30, 77])));
    let out = adjust(&img, &AdjustSpec { brightness: 0.5, contrast: 0.5, saturation: 0.5 }).unwrap();
    assert_eq!(out.to_rgba8().get_pixel(0, 0).0[3], 77);
}

#[test]
fn rejects_out_of_range_adjustments() {
    let img = common::gradient(4, 4);
    let err = adjust(&img, &AdjustSpec { brightness: 5.0, contrast: 0.0, saturation: 0.0 }).unwrap_err();
    assert!(matches!(err, CoreError::InvalidParameter(_)));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core --test filters_test`
Expected: FAIL na compilação.

- [ ] **Step 4: Implementar os filtros**

`crates/core/src/filters.rs`:

```rust
use crate::error::CoreError;
use image::{DynamicImage, Rgba, RgbaImage};

#[derive(Debug, Clone, Copy)]
pub struct SharpenSpec {
    /// 0.0 disables the filter. Useful range goes up to 3.0.
    pub amount: f32,
    /// Blur radius of the unsharp mask, in pixels.
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DenoiseSpec {
    /// Median filter radius in pixels. 0 disables the filter.
    pub radius: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct AdjustSpec {
    /// -1.0 to 1.0.
    pub brightness: f32,
    /// -1.0 to 1.0.
    pub contrast: f32,
    /// -1.0 fully desaturates, 1.0 doubles saturation.
    pub saturation: f32,
}

/// Unsharp mask: the image plus a weighted copy of its own high frequencies.
pub fn sharpen(img: &DynamicImage, spec: &SharpenSpec) -> Result<DynamicImage, CoreError> {
    if !(0.0..=5.0).contains(&spec.amount) {
        return Err(CoreError::InvalidParameter(
            "sharpen amount deve ficar entre 0.0 e 5.0".into(),
        ));
    }
    if spec.amount == 0.0 {
        return Ok(img.clone());
    }

    let blurred = img.blur(spec.radius.max(0.1));
    let original = img.to_rgba8();
    let blurred = blurred.to_rgba8();
    let mut out = RgbaImage::new(img.width(), img.height());

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let o = original.get_pixel(x, y).0;
        let b = blurred.get_pixel(x, y).0;
        let mut channels = [0u8; 4];
        for c in 0..3 {
            let detail = o[c] as f32 - b[c] as f32;
            channels[c] = clamp_u8(o[c] as f32 + detail * spec.amount);
        }
        channels[3] = o[3];
        *pixel = Rgba(channels);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

/// Median filter. It removes speckle while keeping edges, which a blur would not.
pub fn denoise(img: &DynamicImage, spec: &DenoiseSpec) -> Result<DynamicImage, CoreError> {
    if spec.radius > 10 {
        return Err(CoreError::InvalidParameter(
            "denoise radius deve ficar entre 0 e 10".into(),
        ));
    }
    if spec.radius == 0 {
        return Ok(img.clone());
    }

    let filtered = imageproc::filter::median_filter(&img.to_rgba8(), spec.radius, spec.radius);
    Ok(DynamicImage::ImageRgba8(filtered))
}

pub fn adjust(img: &DynamicImage, spec: &AdjustSpec) -> Result<DynamicImage, CoreError> {
    for (name, value) in [
        ("brightness", spec.brightness),
        ("contrast", spec.contrast),
        ("saturation", spec.saturation),
    ] {
        if !(-1.0..=1.0).contains(&value) {
            return Err(CoreError::InvalidParameter(format!(
                "{name} deve ficar entre -1.0 e 1.0, recebido {value}"
            )));
        }
    }

    let source = img.to_rgba8();
    let mut out = RgbaImage::new(img.width(), img.height());
    let contrast_factor = 1.0 + spec.contrast;
    let brightness_offset = spec.brightness * 255.0;

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let p = source.get_pixel(x, y).0;
        let mut channels = [0f32; 3];
        for c in 0..3 {
            let value = p[c] as f32;
            // Contrast pivots around mid grey so the image does not drift dark.
            let with_contrast = (value - 127.5) * contrast_factor + 127.5;
            channels[c] = with_contrast + brightness_offset;
        }

        // Rec. 709 luminance, the same weighting browsers use for grayscale.
        let luma = 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
        let saturation_factor = 1.0 + spec.saturation;
        let final_channels = [
            clamp_u8(luma + (channels[0] - luma) * saturation_factor),
            clamp_u8(luma + (channels[1] - luma) * saturation_factor),
            clamp_u8(luma + (channels[2] - luma) * saturation_factor),
            p[3],
        ];
        *pixel = Rgba(final_channels);
    }

    Ok(DynamicImage::ImageRgba8(out))
}

fn clamp_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
```

Em `crates/core/src/lib.rs`, acrescentar:

```rust
mod filters;
pub use filters::{adjust, denoise, sharpen, AdjustSpec, DenoiseSpec, SharpenSpec};
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 28 testes.

- [ ] **Step 6: Atualizar licenças e commitar**

Acrescentar em `THIRD-PARTY.md`:

```markdown
| imageproc | MIT | filtro de mediana para denoise |
```

```bash
git add Cargo.toml THIRD-PARTY.md crates/core
git commit -m "feat: add sharpen, denoise and tone adjustment filters"
```

---

### Task 5: Rasterização de SVG

**Files:**
- Create: `crates/core/src/svg.rs`, `crates/core/tests/fixtures/circle.svg`
- Modify: `crates/core/src/lib.rs`, `crates/core/src/decode.rs`, `crates/core/Cargo.toml`, `Cargo.toml`, `THIRD-PARTY.md`
- Test: `crates/core/tests/svg_test.rs`

**Interfaces:**
- Consumes: `CoreError` da Tarefa 1.
- Produces: `is_svg(bytes: &[u8]) -> bool`, `rasterize_svg(bytes: &[u8], width: Option<u32>, height: Option<u32>) -> Result<DynamicImage, CoreError>`.

- [ ] **Step 1: Declarar a dependência e criar a fixture**

Em `Cargo.toml` da raiz:

```toml
resvg = "0.45"
```

Em `crates/core/Cargo.toml`:

```toml
resvg.workspace = true
```

`crates/core/tests/fixtures/circle.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50" width="100" height="50">
  <rect width="100" height="50" fill="#ffffff"/>
  <circle cx="50" cy="25" r="20" fill="#ff0000"/>
</svg>
```

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/svg_test.rs`:

```rust
use vdesigner_core::{is_svg, rasterize_svg, CoreError};

const CIRCLE: &[u8] = include_bytes!("fixtures/circle.svg");

#[test]
fn recognises_svg_bytes() {
    assert!(is_svg(CIRCLE));
    assert!(!is_svg(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn recognises_svg_with_leading_xml_declaration() {
    let with_prolog = b"<?xml version=\"1.0\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
    assert!(is_svg(with_prolog));
}

#[test]
fn rasterizes_at_the_intrinsic_size_when_no_dimension_is_given() {
    let img = rasterize_svg(CIRCLE, None, None).unwrap();
    assert_eq!((img.width(), img.height()), (100, 50));
}

#[test]
fn rasterizes_at_any_requested_size_keeping_the_ratio() {
    let img = rasterize_svg(CIRCLE, Some(400), None).unwrap();
    assert_eq!((img.width(), img.height()), (400, 200));
}

#[test]
fn renders_the_expected_colours() {
    let img = rasterize_svg(CIRCLE, None, None).unwrap();
    let rgba = img.to_rgba8();
    assert_eq!(rgba.get_pixel(50, 25).0[0..3], [255, 0, 0], "centro vermelho");
    assert_eq!(rgba.get_pixel(2, 2).0[0..3], [255, 255, 255], "canto branco");
}

#[test]
fn rejects_malformed_svg() {
    let err = rasterize_svg(b"<svg><unclosed>", None, None).unwrap_err();
    assert!(matches!(err, CoreError::Decode(_)));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core --test svg_test`
Expected: FAIL na compilação.

- [ ] **Step 4: Implementar a rasterização**

`crates/core/src/svg.rs`:

```rust
use crate::error::CoreError;
use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia;
use resvg::usvg;

/// Cheap sniff over the first bytes. SVG has no magic number, so the check looks
/// for the root tag, skipping an optional XML declaration and any whitespace.
pub fn is_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(512)];
    String::from_utf8_lossy(head).contains("<svg")
}

/// Renders vector input into pixels at the requested size. A missing dimension is
/// derived from the intrinsic ratio, so vector art never distorts.
pub fn rasterize_svg(
    bytes: &[u8],
    width: Option<u32>,
    height: Option<u32>,
) -> Result<DynamicImage, CoreError> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &options)
        .map_err(|e| CoreError::Decode(format!("SVG inválido: {e}")))?;

    let intrinsic = tree.size();
    let (target_w, target_h) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let ratio = intrinsic.height() / intrinsic.width();
            (w, ((w as f32 * ratio).round() as u32).max(1))
        }
        (None, Some(h)) => {
            let ratio = intrinsic.width() / intrinsic.height();
            (((h as f32 * ratio).round() as u32).max(1), h)
        }
        (None, None) => (
            intrinsic.width().round().max(1.0) as u32,
            intrinsic.height().round().max(1.0) as u32,
        ),
    };

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| CoreError::InvalidParameter("dimensões de SVG inválidas".into()))?;

    let transform = tiny_skia::Transform::from_scale(
        target_w as f32 / intrinsic.width(),
        target_h as f32 / intrinsic.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let buffer = RgbaImage::from_raw(target_w, target_h, pixmap.take())
        .ok_or_else(|| CoreError::Decode("buffer de SVG inválido".into()))?;
    Ok(DynamicImage::ImageRgba8(buffer))
}
```

Atenção ao detalhe: `tiny_skia` entrega pixels com alpha pré-multiplicado. Para o caso comum de SVG opaco o resultado é idêntico, e a primeira versão aceita isso. Se aparecer transparência parcial com cor escurecida, desfaça a pré-multiplicação antes de montar o `RgbaImage`.

Em `crates/core/src/lib.rs`, acrescentar:

```rust
mod svg;
pub use svg::{is_svg, rasterize_svg};
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 34 testes.

- [ ] **Step 6: Atualizar licenças e commitar**

Acrescentar em `THIRD-PARTY.md`:

```markdown
| resvg | MPL-2.0 | rasterização de SVG |
```

Nota para quem implementa: a MPL-2.0 é licença de arquivo, compatível com a distribuição de um aplicativo MIT, desde que o código da própria `resvg` continue disponível. Nada precisa ser feito além do registro aqui.

```bash
git add Cargo.toml THIRD-PARTY.md crates/core
git commit -m "feat: add svg rasterization at arbitrary sizes"
```

---

### Task 6: Pipeline de job e prévia

**Files:**
- Create: `crates/core/src/job.rs`
- Modify: `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, `Cargo.toml`, `THIRD-PARTY.md`
- Test: `crates/core/tests/job_test.rs`

**Interfaces:**
- Consumes: tudo das Tarefas 1 a 5.
- Produces: `Step`, `Job { steps, output }`, `JobOutput { bytes, width, height }`, `Progress { current, total, label }`, `run_job(input: &[u8], job: &Job, progress: &mut dyn FnMut(Progress)) -> Result<JobOutput, CoreError>`, `run_preview(input: &[u8], job: &Job, max_side: u32) -> Result<JobOutput, CoreError>`.

Este é o coração do plano. `Job` é serializável, porque a interface o envia como JSON e a Tarefa de presets vai gravá-lo em disco sem alteração.

- [ ] **Step 1: Declarar a dependência de serialização**

Em `Cargo.toml` da raiz:

```toml
serde = { version = "1", features = ["derive"] }
```

Em `crates/core/Cargo.toml`:

```toml
serde.workspace = true
```

Acrescentar `#[derive(serde::Serialize, serde::Deserialize)]` em `OutputFormat`, `EncodeSpec`, `FitMode`, `ResizeSpec`, `SharpenSpec`, `DenoiseSpec` e `AdjustSpec`, mantendo os derives que já existem.

- [ ] **Step 2: Escrever o teste que falha**

`crates/core/tests/job_test.rs`:

```rust
mod common;

use vdesigner_core::{
    run_job, run_preview, CoreError, DenoiseSpec, EncodeSpec, FitMode, Job, OutputFormat, Progress,
    ResizeSpec, SharpenSpec, Step,
};

fn png_input(width: u32, height: u32) -> Vec<u8> {
    common::as_png(&common::gradient(width, height))
}

fn webp_output() -> EncodeSpec {
    EncodeSpec { format: OutputFormat::WebP, quality: 82, lossless: false }
}

#[test]
fn runs_an_empty_job_as_a_pure_format_conversion() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_job(&png_input(40, 20), &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (40, 20));
    assert!(!out.bytes.is_empty());
}

#[test]
fn applies_steps_in_order() {
    let job = Job {
        steps: vec![
            Step::Resize(ResizeSpec {
                width: Some(100),
                height: None,
                fit: FitMode::Contain,
                pad_color: [0, 0, 0, 0],
            }),
            Step::Sharpen(SharpenSpec { amount: 0.5, radius: 1.0 }),
        ],
        output: webp_output(),
    };
    let out = run_job(&png_input(400, 200), &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (100, 50));
}

#[test]
fn reports_progress_once_per_step_plus_encoding() {
    let job = Job {
        steps: vec![
            Step::Denoise(DenoiseSpec { radius: 1 }),
            Step::Sharpen(SharpenSpec { amount: 0.5, radius: 1.0 }),
        ],
        output: webp_output(),
    };
    let mut seen: Vec<Progress> = Vec::new();
    run_job(&png_input(32, 32), &job, &mut |p| seen.push(p)).unwrap();

    assert_eq!(seen.len(), 3, "duas etapas mais a codificação");
    assert_eq!(seen.last().unwrap().current, 3);
    assert!(seen.iter().all(|p| p.total == 3));
}

#[test]
fn accepts_svg_input_and_rasterizes_it() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10" width="10" height="10"><rect width="10" height="10" fill="#00ff00"/></svg>"#;
    let job = Job {
        steps: vec![Step::Resize(ResizeSpec {
            width: Some(64),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        })],
        output: EncodeSpec { format: OutputFormat::Png, quality: 100, lossless: true },
    };
    let out = run_job(svg, &job, &mut |_| {}).unwrap();
    assert_eq!((out.width, out.height), (64, 64));
}

#[test]
fn preview_limits_the_longest_side() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_preview(&png_input(4000, 2000), &job, 512).unwrap();
    assert_eq!(out.width, 512);
    assert_eq!(out.height, 256);
}

#[test]
fn preview_does_not_upscale_small_images() {
    let job = Job { steps: vec![], output: webp_output() };
    let out = run_preview(&png_input(100, 50), &job, 512).unwrap();
    assert_eq!((out.width, out.height), (100, 50));
}

#[test]
fn preview_keeps_an_explicit_resize_proportional_to_the_reduction() {
    // A preview of a job that resizes to 2000px must not return 2000px of pixels.
    let job = Job {
        steps: vec![Step::Resize(ResizeSpec {
            width: Some(2000),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        })],
        output: webp_output(),
    };
    let out = run_preview(&png_input(4000, 2000), &job, 512).unwrap();
    assert!(out.width <= 512, "prévia não pode exceder o limite, veio {}", out.width);
}

#[test]
fn propagates_a_decode_failure() {
    let job = Job { steps: vec![], output: webp_output() };
    let err = run_job(b"garbage", &job, &mut |_| {}).unwrap_err();
    assert!(matches!(err, CoreError::UnsupportedFormat));
}

#[test]
fn job_round_trips_through_json() {
    let job = Job {
        steps: vec![Step::Denoise(DenoiseSpec { radius: 2 })],
        output: webp_output(),
    };
    let text = serde_json::to_string(&job).unwrap();
    let parsed: Job = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed.steps.len(), 1);
    assert_eq!(parsed.output.quality, 82);
}
```

Acrescentar `serde_json = "1"` em `[dev-dependencies]` de `crates/core/Cargo.toml`.

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner-core --test job_test`
Expected: FAIL na compilação.

- [ ] **Step 4: Implementar o pipeline**

`crates/core/src/job.rs`:

```rust
use crate::{
    adjust, decode, denoise, encode, error::CoreError, is_svg, rasterize_svg, resize, sharpen,
    AdjustSpec, DenoiseSpec, EncodeSpec, FitMode, ResizeSpec, SharpenSpec,
};
use image::DynamicImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Step {
    Resize(ResizeSpec),
    Sharpen(SharpenSpec),
    Denoise(DenoiseSpec),
    Adjust(AdjustSpec),
}

/// A complete operation, described as data. The Editor builds one; the batch
/// screen will build many from a preset. The engine cannot tell them apart.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub steps: Vec<Step>,
    pub output: EncodeSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// 1-based index of the stage that just finished.
    pub current: u32,
    pub total: u32,
    pub label: &'static str,
}

#[derive(Debug, Clone)]
pub struct JobOutput {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Runs the job at full resolution.
pub fn run_job(
    input: &[u8],
    job: &Job,
    progress: &mut dyn FnMut(Progress),
) -> Result<JobOutput, CoreError> {
    let image = load(input, job)?;
    execute(image, job, progress)
}

/// Runs the same job over a reduced copy, so the Editor stays interactive.
/// The reduction happens before the steps, and the final encode is unchanged,
/// which keeps a single code path between preview and export.
pub fn run_preview(input: &[u8], job: &Job, max_side: u32) -> Result<JobOutput, CoreError> {
    if max_side == 0 {
        return Err(CoreError::InvalidParameter(
            "max_side deve ser maior que zero".into(),
        ));
    }

    let image = load(input, job)?;
    let source_width = image.width().max(1);
    let longest = image.width().max(image.height());
    let reduced = if longest > max_side {
        let ratio = max_side as f64 / longest as f64;
        let spec = ResizeSpec {
            width: Some(((image.width() as f64 * ratio).round() as u32).max(1)),
            height: None,
            fit: FitMode::Contain,
            pad_color: [0, 0, 0, 0],
        };
        resize(&image, &spec)?
    } else {
        image
    };

    // A resize step inside the job would undo the reduction, so each one is
    // rescaled by the same factor the source was reduced by.
    let scale_ratio = reduced.width() as f64 / source_width as f64;
    let scaled_job = Job {
        steps: job.steps.iter().map(|s| scale_step(s, scale_ratio)).collect(),
        output: job.output,
    };

    execute(reduced, &scaled_job, &mut |_| {})
}

fn load(input: &[u8], job: &Job) -> Result<DynamicImage, CoreError> {
    if is_svg(input) {
        // Vector input is rasterized straight at the first resize target when
        // there is one, which avoids resampling a raster copy later.
        let target = job.steps.iter().find_map(|step| match step {
            Step::Resize(spec) => Some((spec.width, spec.height)),
            _ => None,
        });
        let (width, height) = target.unwrap_or((None, None));
        rasterize_svg(input, width, height)
    } else {
        decode(input)
    }
}

fn scale_step(step: &Step, ratio: f64) -> Step {
    match step {
        Step::Resize(spec) => Step::Resize(ResizeSpec {
            width: spec.width.map(|w| ((w as f64 * ratio).round() as u32).max(1)),
            height: spec.height.map(|h| ((h as f64 * ratio).round() as u32).max(1)),
            ..*spec
        }),
        other => other.clone(),
    }
}

fn execute(
    mut image: DynamicImage,
    job: &Job,
    progress: &mut dyn FnMut(Progress),
) -> Result<JobOutput, CoreError> {
    let total = job.steps.len() as u32 + 1;

    for (index, step) in job.steps.iter().enumerate() {
        let label = match step {
            Step::Resize(spec) => {
                image = resize(&image, spec)?;
                "redimensionando"
            }
            Step::Sharpen(spec) => {
                image = sharpen(&image, spec)?;
                "aplicando nitidez"
            }
            Step::Denoise(spec) => {
                image = denoise(&image, spec)?;
                "reduzindo ruído"
            }
            Step::Adjust(spec) => {
                image = adjust(&image, spec)?;
                "ajustando tom"
            }
        };
        progress(Progress { current: index as u32 + 1, total, label });
    }

    let bytes = encode(&image, &job.output)?;
    progress(Progress { current: total, total, label: "codificando" });

    Ok(JobOutput { bytes, width: image.width(), height: image.height() })
}
```

Em `crates/core/src/lib.rs`, acrescentar:

```rust
mod job;
pub use job::{run_job, run_preview, Job, JobOutput, Progress, Step};
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `cargo test -p vdesigner-core`
Expected: PASS, 43 testes.

- [ ] **Step 6: Commitar**

```bash
git add Cargo.toml crates/core
git commit -m "feat: add declarative job pipeline with preview path"
```

---

### Task 7: Camada Tauri e comandos

**Files:**
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/session.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/export.rs`
- Modify: `Cargo.toml` (membros do workspace), `THIRD-PARTY.md`
- Test: `src-tauri/src/export.rs` (testes em módulo), `src-tauri/tests/commands_test.rs`

**Interfaces:**
- Consumes: `Job`, `run_job`, `run_preview`, `JobOutput`, `CoreError` da Tarefa 6.
- Produces: comandos `open_image(path) -> ImageInfo`, `preview(job) -> PreviewResult`, `export(job, output_dir, file_stem, overwrite) -> ExportResult`; tipos `ImageInfo { width, height, format, preview_png_base64 }`, `PreviewResult { png_base64, width, height }`, `ExportResult { path, bytes_written }`.

- [ ] **Step 1: Criar a crate e registrar no workspace**

Em `Cargo.toml` da raiz, mudar os membros:

```toml
members = ["crates/core", "src-tauri"]
```

`src-tauri/Cargo.toml`:

```toml
[package]
name = "vdesigner"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
vdesigner-core = { path = "../crates/core" }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde.workspace = true
serde_json = "1"
base64 = "0.22"
thiserror.workspace = true

[dev-dependencies]
tempfile = "3"
```

`src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 2: Escrever o teste que falha, para a proteção de sobrescrita**

A regra de sobrescrita é a única lógica de verdade nesta camada, então é ela que recebe teste automatizado.

`src-tauri/tests/commands_test.rs`:

```rust
use std::fs;
use vdesigner::export::{resolve_output_path, ExportError};

#[test]
fn builds_the_path_from_the_stem_and_the_format_extension() {
    let dir = tempfile::tempdir().unwrap();
    let path = resolve_output_path(dir.path(), "hero", "webp", false).unwrap();
    assert_eq!(path.file_name().unwrap(), "hero.webp");
}

#[test]
fn refuses_to_overwrite_unless_explicitly_allowed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("hero.webp"), b"existing").unwrap();

    let err = resolve_output_path(dir.path(), "hero", "webp", false).unwrap_err();
    assert!(matches!(err, ExportError::WouldOverwrite(_)));
}

#[test]
fn overwrites_when_explicitly_allowed() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("hero.webp"), b"existing").unwrap();

    let path = resolve_output_path(dir.path(), "hero", "webp", true).unwrap();
    assert_eq!(path.file_name().unwrap(), "hero.webp");
}

#[test]
fn rejects_a_stem_that_escapes_the_output_directory() {
    let dir = tempfile::tempdir().unwrap();
    let err = resolve_output_path(dir.path(), "../outside", "webp", true).unwrap_err();
    assert!(matches!(err, ExportError::InvalidName(_)));
}

#[test]
fn rejects_an_empty_stem() {
    let dir = tempfile::tempdir().unwrap();
    let err = resolve_output_path(dir.path(), "   ", "webp", true).unwrap_err();
    assert!(matches!(err, ExportError::InvalidName(_)));
}
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `cargo test -p vdesigner`
Expected: FAIL na compilação, `unresolved import vdesigner::export`.

- [ ] **Step 4: Implementar a escrita de saída**

`src-tauri/src/export.rs`:

```rust
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("o arquivo já existe: {0}")]
    WouldOverwrite(String),

    #[error("nome de arquivo inválido: {0}")]
    InvalidName(String),

    #[error("falha ao gravar: {0}")]
    Io(String),
}

/// Builds the destination path and enforces the overwrite rule. Overwriting an
/// existing file is only ever allowed when the caller asks for it explicitly.
pub fn resolve_output_path(
    directory: &Path,
    stem: &str,
    extension: &str,
    allow_overwrite: bool,
) -> Result<PathBuf, ExportError> {
    let trimmed = stem.trim();
    if trimmed.is_empty() {
        return Err(ExportError::InvalidName("o nome não pode ficar vazio".into()));
    }
    if trimmed.contains(['/', '\\', ':']) || trimmed.contains("..") {
        return Err(ExportError::InvalidName(format!(
            "o nome não pode conter separador de caminho: {trimmed}"
        )));
    }

    let path = directory.join(format!("{trimmed}.{extension}"));
    if path.exists() && !allow_overwrite {
        return Err(ExportError::WouldOverwrite(path.display().to_string()));
    }
    Ok(path)
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<u64, ExportError> {
    std::fs::write(path, bytes).map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(bytes.len() as u64)
}
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

`src-tauri/src/main.rs` precisa expor o módulo como biblioteca para o teste alcançá-lo. Criar `src-tauri/src/lib.rs`:

```rust
pub mod commands;
pub mod export;
pub mod session;
```

E em `src-tauri/Cargo.toml`:

```toml
[lib]
name = "vdesigner"
path = "src/lib.rs"

[[bin]]
name = "vdesigner"
path = "src/main.rs"
```

Run: `cargo test -p vdesigner`
Expected: PASS, 5 testes.

- [ ] **Step 6: Implementar a sessão e os comandos**

`src-tauri/src/session.rs`:

```rust
use std::sync::Mutex;

/// The bytes of the image currently open in the Editor. Kept in memory so the
/// preview never re-reads the disk on every slider move.
#[derive(Default)]
pub struct Session {
    pub source: Mutex<Option<SourceImage>>,
}

pub struct SourceImage {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub file_stem: String,
}
```

`src-tauri/src/commands.rs`:

```rust
use crate::export::{resolve_output_path, write_bytes};
use crate::session::{Session, SourceImage};
use base64::Engine;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use vdesigner_core::{
    decode, is_svg, rasterize_svg, run_job, run_preview, EncodeSpec, Job, OutputFormat,
};

const PREVIEW_MAX_SIDE: u32 = 2048;

#[derive(Serialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub file_stem: String,
    pub preview_png_base64: String,
}

#[derive(Serialize)]
pub struct PreviewResult {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize)]
pub struct ExportResult {
    pub path: String,
    pub bytes_written: u64,
}

/// Every command returns a plain string on failure, because the UI only ever
/// displays the message. The engine keeps the typed errors.
type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn open_image(path: String, session: State<Session>) -> CommandResult<ImageInfo> {
    let bytes = std::fs::read(&path).map_err(|e| format!("não foi possível ler o arquivo: {e}"))?;

    let image = if is_svg(&bytes) {
        rasterize_svg(&bytes, None, None).map_err(|e| e.to_string())?
    } else {
        decode(&bytes).map_err(|e| e.to_string())?
    };

    let file_stem = PathBuf::from(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "imagem".to_string());

    let info = ImageInfo {
        width: image.width(),
        height: image.height(),
        file_stem: file_stem.clone(),
        preview_png_base64: preview_png(&bytes)?,
    };

    *session.source.lock().map_err(|_| "estado corrompido".to_string())? = Some(SourceImage {
        bytes,
        width: image.width(),
        height: image.height(),
        file_stem,
    });

    Ok(info)
}

#[tauri::command]
pub fn preview(job: Job, session: State<Session>) -> CommandResult<PreviewResult> {
    let guard = session.source.lock().map_err(|_| "estado corrompido".to_string())?;
    let source = guard.as_ref().ok_or("nenhuma imagem aberta")?;

    // The preview is always served as PNG, so the canvas shows exactly the
    // pixels the pipeline produced, without a second lossy pass.
    let png_job = Job {
        steps: job.steps.clone(),
        output: EncodeSpec { format: OutputFormat::Png, quality: 100, lossless: true },
    };
    let output =
        run_preview(&source.bytes, &png_job, PREVIEW_MAX_SIDE).map_err(|e| e.to_string())?;

    Ok(PreviewResult {
        png_base64: base64::engine::general_purpose::STANDARD.encode(&output.bytes),
        width: output.width,
        height: output.height,
    })
}

#[tauri::command]
pub fn export(
    job: Job,
    output_dir: String,
    file_stem: String,
    overwrite: bool,
    session: State<Session>,
) -> CommandResult<ExportResult> {
    let guard = session.source.lock().map_err(|_| "estado corrompido".to_string())?;
    let source = guard.as_ref().ok_or("nenhuma imagem aberta")?;

    let extension = extension_for(job.output.format);
    let path = resolve_output_path(std::path::Path::new(&output_dir), &file_stem, extension, overwrite)
        .map_err(|e| e.to_string())?;

    let output = run_job(&source.bytes, &job, &mut |_| {}).map_err(|e| e.to_string())?;
    let bytes_written = write_bytes(&path, &output.bytes).map_err(|e| e.to_string())?;

    Ok(ExportResult { path: path.display().to_string(), bytes_written })
}

fn extension_for(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::WebP => "webp",
        OutputFormat::Avif => "avif",
        OutputFormat::Png => "png",
        OutputFormat::Jpeg => "jpg",
        OutputFormat::Tiff => "tiff",
        OutputFormat::Ico => "ico",
    }
}

fn preview_png(bytes: &[u8]) -> CommandResult<String> {
    let job = Job {
        steps: vec![],
        output: EncodeSpec { format: OutputFormat::Png, quality: 100, lossless: true },
    };
    let output = run_preview(bytes, &job, PREVIEW_MAX_SIDE).map_err(|e| e.to_string())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&output.bytes))
}
```

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use vdesigner::commands;
use vdesigner::session::Session;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Session::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_image,
            commands::preview,
            commands::export
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Vdesigner");
}
```

`src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Vdesigner",
  "version": "0.1.0",
  "identifier": "br.com.vdesigner.app",
  "build": {
    "frontendDist": "../ui/dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm --prefix ../ui run dev",
    "beforeBuildCommand": "npm --prefix ../ui run build"
  },
  "app": {
    "windows": [
      {
        "title": "Vdesigner",
        "width": 1280,
        "height": 800,
        "minWidth": 960,
        "minHeight": 640
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["msi"],
    "icon": ["icons/icon.ico"]
  }
}
```

Nota: gere `src-tauri/icons/icon.ico` com `cargo tauri icon` a partir de um PNG quadrado de 1024px, ou use o ícone padrão do template do Tauri por enquanto.

- [ ] **Step 7: Rodar a verificação e commitar**

Run: `cargo test --all && cargo clippy --all-targets -- -D warnings`
Expected: PASS.

Acrescentar em `THIRD-PARTY.md`:

```markdown
| tauri | MIT OR Apache-2.0 | camada de aplicativo |
| tauri-plugin-dialog | MIT OR Apache-2.0 | seleção de arquivo e pasta |
| base64 | MIT OR Apache-2.0 | transporte da prévia para a interface |
| serde, serde_json | MIT OR Apache-2.0 | serialização de Job |
```

```bash
git add Cargo.toml THIRD-PARTY.md src-tauri
git commit -m "feat: add tauri commands for open, preview and export"
```

---

### Task 8: Interface do Editor

**Files:**
- Create: `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/index.html`, `ui/src/main.tsx`, `ui/src/App.tsx`, `ui/src/api.ts`, `ui/src/styles.css`
- Create: `ui/src/screens/Editor.tsx`, `ui/src/components/PreviewPane.tsx`, `ui/src/components/ResizeControls.tsx`, `ui/src/components/QualityControls.tsx`, `ui/src/components/ExportBar.tsx`
- Test: `ui/src/api.test.ts`, `ui/src/screens/Editor.test.tsx`

**Interfaces:**
- Consumes: os comandos `open_image`, `preview`, `export` da Tarefa 7.
- Produces: a aplicação completa. Nenhuma tarefa posterior deste plano depende dela.

- [ ] **Step 1: Criar o projeto da interface**

`ui/package.json`:

```json
{
  "name": "vdesigner-ui",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "test": "vitest run",
    "lint": "tsc --noEmit"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.5.0",
    "@testing-library/react": "^16.0.1",
    "@testing-library/user-event": "^14.5.2",
    "@types/react": "^18.3.5",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "jsdom": "^25.0.0",
    "typescript": "^5.6.2",
    "vite": "^5.4.0",
    "vitest": "^2.1.0"
  }
}
```

`ui/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  test: { environment: "jsdom", setupFiles: ["./src/test-setup.ts"] },
});
```

`ui/src/test-setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

`ui/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noEmit": true,
    "types": ["vitest/globals"]
  },
  "include": ["src"]
}
```

`ui/index.html`:

```html
<!doctype html>
<html lang="pt-BR">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Vdesigner</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Run: `npm --prefix ui install`

- [ ] **Step 2: Escrever o teste que falha, para a construção do Job**

A única lógica de verdade na interface é montar o `Job` a partir dos controles. É isso que o teste cobre.

`ui/src/api.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { buildJob, type EditorSettings } from "./api";

const base: EditorSettings = {
  width: null,
  height: null,
  lockRatio: true,
  fit: "Contain",
  sharpenAmount: 0,
  denoiseRadius: 0,
  brightness: 0,
  contrast: 0,
  saturation: 0,
  format: "WebP",
  quality: 82,
};

describe("buildJob", () => {
  it("produz um job sem etapas quando nada foi alterado", () => {
    const job = buildJob(base);
    expect(job.steps).toHaveLength(0);
    expect(job.output).toEqual({ format: "WebP", quality: 82, lossless: false });
  });

  it("inclui o resize quando há largura", () => {
    const job = buildJob({ ...base, width: 1920 });
    expect(job.steps).toEqual([
      { Resize: { width: 1920, height: null, fit: "Contain", pad_color: [0, 0, 0, 0] } },
    ]);
  });

  it("omite a altura enquanto a proporção está travada", () => {
    const job = buildJob({ ...base, width: 800, height: 600, lockRatio: true });
    expect(job.steps[0]).toEqual({
      Resize: { width: 800, height: null, fit: "Contain", pad_color: [0, 0, 0, 0] },
    });
  });

  it("envia as duas dimensões quando a proporção está destravada", () => {
    const job = buildJob({ ...base, width: 800, height: 600, lockRatio: false, fit: "Stretch" });
    expect(job.steps[0]).toEqual({
      Resize: { width: 800, height: 600, fit: "Stretch", pad_color: [0, 0, 0, 0] },
    });
  });

  it("ordena as etapas como denoise, sharpen e ajuste", () => {
    const job = buildJob({ ...base, sharpenAmount: 1, denoiseRadius: 2, brightness: 0.1 });
    expect(job.steps.map((s) => Object.keys(s)[0])).toEqual(["Denoise", "Sharpen", "Adjust"]);
  });

  it("marca png e tiff como lossless", () => {
    expect(buildJob({ ...base, format: "Png" }).output.lossless).toBe(true);
  });
});
```

- [ ] **Step 3: Rodar o teste e confirmar que falha**

Run: `npm --prefix ui run test`
Expected: FAIL, `Failed to resolve import "./api"`.

- [ ] **Step 4: Implementar a camada de API**

`ui/src/api.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";

export type OutputFormat = "WebP" | "Avif" | "Png" | "Jpeg" | "Tiff" | "Ico";
export type FitMode = "Contain" | "Cover" | "Stretch";

/** Mirrors vdesigner_core::Step, which serde serialises as an externally tagged enum. */
export type Step =
  | { Resize: { width: number | null; height: number | null; fit: FitMode; pad_color: [number, number, number, number] } }
  | { Sharpen: { amount: number; radius: number } }
  | { Denoise: { radius: number } }
  | { Adjust: { brightness: number; contrast: number; saturation: number } };

export interface Job {
  steps: Step[];
  output: { format: OutputFormat; quality: number; lossless: boolean };
}

export interface EditorSettings {
  width: number | null;
  height: number | null;
  lockRatio: boolean;
  fit: FitMode;
  sharpenAmount: number;
  denoiseRadius: number;
  brightness: number;
  contrast: number;
  saturation: number;
  format: OutputFormat;
  quality: number;
}

const LOSSLESS_FORMATS: OutputFormat[] = ["Png", "Tiff", "Ico"];

/** Turns the editor controls into the engine's job description. */
export function buildJob(settings: EditorSettings): Job {
  const steps: Step[] = [];

  if (settings.width !== null || settings.height !== null) {
    steps.push({
      Resize: {
        width: settings.width,
        // With the ratio locked, sending only one dimension lets the engine
        // derive the other, which is what keeps the image undistorted.
        height: settings.lockRatio ? null : settings.height,
        fit: settings.fit,
        pad_color: [0, 0, 0, 0],
      },
    });
  }

  if (settings.denoiseRadius > 0) {
    steps.push({ Denoise: { radius: settings.denoiseRadius } });
  }
  if (settings.sharpenAmount > 0) {
    steps.push({ Sharpen: { amount: settings.sharpenAmount, radius: 1.0 } });
  }
  if (settings.brightness !== 0 || settings.contrast !== 0 || settings.saturation !== 0) {
    steps.push({
      Adjust: {
        brightness: settings.brightness,
        contrast: settings.contrast,
        saturation: settings.saturation,
      },
    });
  }

  return {
    steps,
    output: {
      format: settings.format,
      quality: settings.quality,
      lossless: LOSSLESS_FORMATS.includes(settings.format),
    },
  };
}

export interface ImageInfo {
  width: number;
  height: number;
  file_stem: string;
  preview_png_base64: string;
}

export interface PreviewResult {
  png_base64: string;
  width: number;
  height: number;
}

export interface ExportResult {
  path: string;
  bytes_written: number;
}

export const api = {
  openImage: (path: string) => invoke<ImageInfo>("open_image", { path }),
  preview: (job: Job) => invoke<PreviewResult>("preview", { job }),
  export: (job: Job, outputDir: string, fileStem: string, overwrite: boolean) =>
    invoke<ExportResult>("export", { job, outputDir, fileStem, overwrite }),
};
```

- [ ] **Step 5: Rodar o teste e confirmar que passa**

Run: `npm --prefix ui run test`
Expected: PASS, 6 testes.

- [ ] **Step 6: Escrever o teste da tela**

`ui/src/screens/Editor.test.tsx`:

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Editor } from "./Editor";
import { api } from "../api";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    api: {
      openImage: vi.fn(),
      preview: vi.fn(),
      export: vi.fn(),
    },
  };
});

const openImage = vi.mocked(api.openImage);
const preview = vi.mocked(api.preview);

describe("Editor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    preview.mockResolvedValue({ png_base64: "AAA", width: 100, height: 50 });
  });

  it("mostra o estado vazio antes de abrir uma imagem", () => {
    render(<Editor onOpenFile={vi.fn()} />);
    expect(screen.getByText(/arraste uma imagem/i)).toBeInTheDocument();
  });

  it("mostra as dimensões depois de abrir uma imagem", async () => {
    openImage.mockResolvedValue({
      width: 1920,
      height: 1080,
      file_stem: "hero",
      preview_png_base64: "AAA",
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));

    await waitFor(() => expect(screen.getByText(/1920 × 1080/)).toBeInTheDocument());
  });

  it("desabilita o campo de altura enquanto a proporção está travada", async () => {
    openImage.mockResolvedValue({
      width: 800,
      height: 400,
      file_stem: "hero",
      preview_png_base64: "AAA",
    });

    render(<Editor onOpenFile={async () => "C:/temp/hero.jpg"} />);
    await userEvent.click(screen.getByRole("button", { name: /abrir imagem/i }));
    await waitFor(() => expect(screen.getByLabelText(/altura/i)).toBeDisabled());
  });
});
```

Run: `npm --prefix ui run test`
Expected: FAIL, `Failed to resolve import "./Editor"`.

- [ ] **Step 7: Implementar a tela e os componentes**

`ui/src/screens/Editor.tsx`:

```tsx
import { useCallback, useEffect, useRef, useState } from "react";
import { api, buildJob, type EditorSettings, type ImageInfo } from "../api";
import { PreviewPane } from "../components/PreviewPane";
import { ResizeControls } from "../components/ResizeControls";
import { QualityControls } from "../components/QualityControls";
import { ExportBar } from "../components/ExportBar";

const DEFAULT_SETTINGS: EditorSettings = {
  width: null,
  height: null,
  lockRatio: true,
  fit: "Contain",
  sharpenAmount: 0,
  denoiseRadius: 0,
  brightness: 0,
  contrast: 0,
  saturation: 0,
  format: "WebP",
  quality: 82,
};

/** Waiting this long after the last control change keeps the preview from
 *  running on every single slider pixel. */
const PREVIEW_DEBOUNCE_MS = 250;

interface EditorProps {
  /** Injected so tests do not need the Tauri dialog plugin. */
  onOpenFile: () => Promise<string | null>;
}

export function Editor({ onOpenFile }: EditorProps) {
  const [image, setImage] = useState<ImageInfo | null>(null);
  const [settings, setSettings] = useState<EditorSettings>(DEFAULT_SETTINGS);
  const [previewSrc, setPreviewSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const timer = useRef<number | null>(null);

  const open = useCallback(async () => {
    setError(null);
    const path = await onOpenFile();
    if (!path) return;
    try {
      const info = await api.openImage(path);
      setImage(info);
      setSettings(DEFAULT_SETTINGS);
      setPreviewSrc(`data:image/png;base64,${info.preview_png_base64}`);
    } catch (e) {
      setError(String(e));
    }
  }, [onOpenFile]);

  useEffect(() => {
    if (!image) return;
    if (timer.current !== null) window.clearTimeout(timer.current);

    timer.current = window.setTimeout(async () => {
      setBusy(true);
      try {
        const result = await api.preview(buildJob(settings));
        setPreviewSrc(`data:image/png;base64,${result.png_base64}`);
        setError(null);
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    }, PREVIEW_DEBOUNCE_MS);

    return () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    };
  }, [settings, image]);

  return (
    <main className="editor">
      <header className="editor__header">
        <button type="button" onClick={open}>
          Abrir imagem
        </button>
        {image && (
          <span className="editor__dimensions">
            {image.width} × {image.height}
          </span>
        )}
        {busy && <span role="status">processando…</span>}
      </header>

      {error && <p role="alert" className="editor__error">{error}</p>}

      {!image ? (
        <p className="editor__empty">Arraste uma imagem aqui ou clique em Abrir imagem.</p>
      ) : (
        <div className="editor__body">
          <PreviewPane src={previewSrc} />
          <aside className="editor__panel">
            <ResizeControls
              settings={settings}
              sourceWidth={image.width}
              sourceHeight={image.height}
              onChange={setSettings}
            />
            <QualityControls settings={settings} onChange={setSettings} />
            <ExportBar settings={settings} fileStem={image.file_stem} onError={setError} />
          </aside>
        </div>
      )}
    </main>
  );
}
```

`ui/src/components/PreviewPane.tsx`:

```tsx
interface PreviewPaneProps {
  src: string | null;
}

export function PreviewPane({ src }: PreviewPaneProps) {
  return (
    <section className="preview">
      {src ? <img src={src} alt="Prévia do resultado" /> : <p>Gerando prévia…</p>}
    </section>
  );
}
```

`ui/src/components/ResizeControls.tsx`:

```tsx
import type { EditorSettings, FitMode } from "../api";

interface ResizeControlsProps {
  settings: EditorSettings;
  sourceWidth: number;
  sourceHeight: number;
  onChange: (next: EditorSettings) => void;
}

export function ResizeControls({ settings, sourceWidth, sourceHeight, onChange }: ResizeControlsProps) {
  const ratio = sourceHeight / sourceWidth;

  const setWidth = (value: string) => {
    const width = value === "" ? null : Math.max(1, Number(value));
    onChange({
      ...settings,
      width,
      // The mirrored height is only a hint for the user; with the ratio locked
      // the engine is the one that computes the real value.
      height: settings.lockRatio && width !== null ? Math.round(width * ratio) : settings.height,
    });
  };

  return (
    <fieldset className="controls">
      <legend>Tamanho</legend>

      <label htmlFor="width">Largura</label>
      <input
        id="width"
        type="number"
        min={1}
        value={settings.width ?? ""}
        placeholder={String(sourceWidth)}
        onChange={(e) => setWidth(e.target.value)}
      />

      <label htmlFor="height">Altura</label>
      <input
        id="height"
        type="number"
        min={1}
        disabled={settings.lockRatio}
        value={settings.height ?? ""}
        placeholder={String(sourceHeight)}
        onChange={(e) =>
          onChange({ ...settings, height: e.target.value === "" ? null : Math.max(1, Number(e.target.value)) })
        }
      />

      <label htmlFor="lock">
        <input
          id="lock"
          type="checkbox"
          checked={settings.lockRatio}
          onChange={(e) =>
            onChange({
              ...settings,
              lockRatio: e.target.checked,
              fit: e.target.checked ? "Contain" : settings.fit,
            })
          }
        />
        Manter proporção
      </label>

      {!settings.lockRatio && (
        <>
          <label htmlFor="fit">Encaixe</label>
          <select
            id="fit"
            value={settings.fit}
            onChange={(e) => onChange({ ...settings, fit: e.target.value as FitMode })}
          >
            <option value="Contain">Caber dentro, com preenchimento</option>
            <option value="Cover">Preencher e recortar</option>
            <option value="Stretch">Esticar (distorce)</option>
          </select>
        </>
      )}
    </fieldset>
  );
}
```

`ui/src/components/QualityControls.tsx`:

```tsx
import type { EditorSettings, OutputFormat } from "../api";

interface QualityControlsProps {
  settings: EditorSettings;
  onChange: (next: EditorSettings) => void;
}

const FORMATS: { value: OutputFormat; label: string }[] = [
  { value: "WebP", label: "WebP" },
  { value: "Avif", label: "AVIF" },
  { value: "Png", label: "PNG" },
  { value: "Jpeg", label: "JPG" },
  { value: "Tiff", label: "TIFF" },
  { value: "Ico", label: "ICO (até 256px)" },
];

const LOSSY: OutputFormat[] = ["WebP", "Avif", "Jpeg"];

export function QualityControls({ settings, onChange }: QualityControlsProps) {
  const slider = (
    id: string,
    label: string,
    value: number,
    min: number,
    max: number,
    step: number,
    key: keyof EditorSettings,
  ) => (
    <>
      <label htmlFor={id}>
        {label}: {value}
      </label>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange({ ...settings, [key]: Number(e.target.value) })}
      />
    </>
  );

  return (
    <fieldset className="controls">
      <legend>Qualidade</legend>

      {slider("sharpen", "Nitidez", settings.sharpenAmount, 0, 3, 0.1, "sharpenAmount")}
      {slider("denoise", "Redução de ruído", settings.denoiseRadius, 0, 5, 1, "denoiseRadius")}
      {slider("brightness", "Brilho", settings.brightness, -1, 1, 0.05, "brightness")}
      {slider("contrast", "Contraste", settings.contrast, -1, 1, 0.05, "contrast")}
      {slider("saturation", "Saturação", settings.saturation, -1, 1, 0.05, "saturation")}

      <label htmlFor="format">Formato de saída</label>
      <select
        id="format"
        value={settings.format}
        onChange={(e) => onChange({ ...settings, format: e.target.value as OutputFormat })}
      >
        {FORMATS.map((f) => (
          <option key={f.value} value={f.value}>
            {f.label}
          </option>
        ))}
      </select>

      {LOSSY.includes(settings.format) &&
        slider("quality", "Compressão", settings.quality, 1, 100, 1, "quality")}
    </fieldset>
  );
}
```

`ui/src/components/ExportBar.tsx`:

```tsx
import { useState } from "react";
import { api, buildJob, type EditorSettings } from "../api";

interface ExportBarProps {
  settings: EditorSettings;
  fileStem: string;
  onError: (message: string | null) => void;
}

export function ExportBar({ settings, fileStem, onError }: ExportBarProps) {
  const [outputDir, setOutputDir] = useState("");
  const [stem, setStem] = useState(fileStem);
  const [overwrite, setOverwrite] = useState(false);
  const [saved, setSaved] = useState<string | null>(null);

  const run = async () => {
    onError(null);
    setSaved(null);
    try {
      const result = await api.export(buildJob(settings), outputDir, stem, overwrite);
      setSaved(result.path);
    } catch (e) {
      onError(String(e));
    }
  };

  return (
    <fieldset className="controls">
      <legend>Exportar</legend>

      <label htmlFor="outdir">Pasta de saída</label>
      <input id="outdir" value={outputDir} onChange={(e) => setOutputDir(e.target.value)} />

      <label htmlFor="stem">Nome do arquivo</label>
      <input id="stem" value={stem} onChange={(e) => setStem(e.target.value)} />

      <label htmlFor="overwrite">
        <input
          id="overwrite"
          type="checkbox"
          checked={overwrite}
          onChange={(e) => setOverwrite(e.target.checked)}
        />
        Sobrescrever se já existir
      </label>
      {overwrite && <small>O arquivo existente será substituído sem aviso.</small>}

      <button type="button" onClick={run} disabled={!outputDir || !stem}>
        Exportar
      </button>

      {saved && <p role="status">Salvo em {saved}</p>}
    </fieldset>
  );
}
```

`ui/src/App.tsx`:

```tsx
import { open } from "@tauri-apps/plugin-dialog";
import { Editor } from "./screens/Editor";

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg"];

export function App() {
  const pickFile = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Imagens", extensions: IMAGE_EXTENSIONS }],
    });
    return typeof selected === "string" ? selected : null;
  };

  return <Editor onOpenFile={pickFile} />;
}
```

`ui/src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

`ui/src/styles.css`:

```css
:root {
  color-scheme: dark;
  --bg: #14161a;
  --panel: #1d2026;
  --text: #e8eaed;
  --accent: #4c8dff;
}

* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg);
  color: var(--text);
  font: 14px/1.5 "Segoe UI", system-ui, sans-serif;
}

.editor { display: flex; flex-direction: column; height: 100vh; }
.editor__header { display: flex; gap: 16px; align-items: center; padding: 12px 16px; background: var(--panel); }
.editor__body { display: grid; grid-template-columns: 1fr 320px; flex: 1; min-height: 0; }
.editor__panel { overflow-y: auto; padding: 16px; background: var(--panel); display: flex; flex-direction: column; gap: 20px; }
.editor__empty { margin: auto; opacity: 0.7; }
.editor__error { margin: 8px 16px; padding: 8px 12px; background: #4a1f1f; border-radius: 4px; }

.preview { display: grid; place-items: center; padding: 24px; overflow: auto; }
.preview img { max-width: 100%; max-height: 100%; object-fit: contain; }

.controls { display: flex; flex-direction: column; gap: 6px; border: 1px solid #2c3038; border-radius: 6px; padding: 12px; }
.controls legend { padding: 0 6px; font-weight: 600; }
.controls input[type="number"], .controls input[type="text"], .controls select { background: #14161a; color: var(--text); border: 1px solid #2c3038; border-radius: 4px; padding: 6px; }
.controls button { background: var(--accent); color: #fff; border: 0; border-radius: 4px; padding: 8px; cursor: pointer; }
.controls button:disabled { opacity: 0.4; cursor: not-allowed; }
```

- [ ] **Step 8: Rodar os testes e confirmar que passam**

Run: `npm --prefix ui run test && npm --prefix ui run lint`
Expected: PASS, 9 testes, sem erro de tipo.

- [ ] **Step 9: Rodar o aplicativo de verdade**

Run: `npm --prefix ui install && cargo tauri dev`

Roteiro de verificação manual, porque canvas e diálogo de arquivo não se automatizam bem:

1. Abrir um JPG grande. A prévia aparece em menos de dois segundos.
2. Digitar 800 na largura. A altura acompanha e a prévia encolhe.
3. Destravar a proporção, escolher Cover, pôr 400 × 400. A prévia recorta sem esticar.
4. Subir a nitidez até 2. A diferença é visível.
5. Trocar o formato para AVIF, exportar para uma pasta vazia. O arquivo aparece.
6. Exportar de novo com o mesmo nome, sem marcar sobrescrever. Aparece o erro de arquivo existente.
7. Marcar sobrescrever e exportar. Funciona.
8. Abrir um SVG e exportar como PNG de 1024px. Sai nítido, sem serrilhado de ampliação.

- [ ] **Step 10: Acrescentar a integração contínua da interface e commitar**

Em `.github/workflows/ci.yml`, acrescentar o job:

```yaml
  ui:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: ui/package-lock.json
      - run: npm --prefix ui ci
      - run: npm --prefix ui run lint
      - run: npm --prefix ui run test
```

```bash
git add ui .github/workflows/ci.yml
git commit -m "feat: add editor screen with live preview and export"
```

---

### Task 9: Empacotamento e documentação de instalação

**Files:**
- Create: `README.md`, `LICENSE`, `.github/workflows/release.yml`
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: o aplicativo completo da Tarefa 8.
- Produces: instalador MSI e pacote portátil em `.zip`.

- [ ] **Step 1: Acrescentar a licença**

`LICENSE`: o texto padrão da MIT, com `Copyright (c) 2026 Vandyck`.

- [ ] **Step 2: Escrever o README**

`README.md`:

```markdown
# Vdesigner

Ferramenta de imagem e cor para designers e desenvolvedores front end. Converte,
redimensiona sem distorção e melhora a qualidade de imagens, com prévia ao vivo.

## Instalação

Baixe o instalador `.msi` mais recente em Releases.

O Windows SmartScreen exibe um aviso na primeira execução, porque o aplicativo
ainda não tem assinatura de código. Clique em "Mais informações" e depois em
"Executar assim mesmo". O checksum SHA-256 de cada release é publicado junto do
arquivo, e permite conferir que o download não foi alterado.

Quem não pode instalar nada na máquina pode usar o pacote `.zip`, que roda
direto da pasta.

## Formatos

Entrada: JPG, PNG, WebP, GIF, BMP, TIFF e SVG.
Saída: WebP, AVIF, PNG, JPG, TIFF e ICO.

## Desenvolvimento

Requer Rust 1.82 e Node 20.

    npm --prefix ui install
    cargo tauri dev

Testes:

    cargo test --all
    npm --prefix ui run test

## Licença

MIT. As licenças das dependências estão em THIRD-PARTY.md.
```

- [ ] **Step 3: Escrever o fluxo de release**

`.github/workflows/release.yml`:

```yaml
name: release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.82
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: npm --prefix ui ci
      - run: cargo install tauri-cli --version "^2" --locked
      - run: cargo tauri build

      - name: Compute checksums
        shell: pwsh
        run: |
          $msi = Get-ChildItem -Recurse -Filter *.msi src-tauri/target/release/bundle
          Get-FileHash $msi.FullName -Algorithm SHA256 | Format-List | Out-File checksums.txt

      - uses: softprops/action-gh-release@v2
        with:
          files: |
            src-tauri/target/release/bundle/msi/*.msi
            checksums.txt
```

- [ ] **Step 4: Construir o instalador localmente e verificar**

Run: `cargo tauri build`
Expected: um `.msi` em `src-tauri/target/release/bundle/msi/`.

Verificação manual: instalar o MSI em uma máquina Windows limpa, abrir o
aplicativo pelo menu Iniciar, repetir os passos 1, 5 e 6 do roteiro da Tarefa 8,
e desinstalar pelo painel de controle sem deixar resíduo.

- [ ] **Step 5: Commitar**

```bash
git add README.md LICENSE .github/workflows/release.yml src-tauri/tauri.conf.json
git commit -m "chore: add msi packaging, release workflow and install docs"
```

---

## Cobertura da spec neste plano

| Requisito da spec | Tarefa |
|---|---|
| Workspace com crate de motor pura | 1 |
| Decodificação JPG, PNG, WebP, GIF, BMP, TIFF | 1 |
| Codificação WebP, AVIF, PNG, JPG, TIFF, ICO | 2 |
| Lanczos3, proporção travada, contain, cover, stretch | 3 |
| Sharpening, denoise, ajuste de contraste e cor | 4 |
| SVG como entrada, rasterizado em qualquer tamanho | 5 |
| Pipeline declarativo `Job` | 6 |
| Prévia limitada a 2048px pelo mesmo caminho de código | 6 |
| Job serializável, base dos presets | 6 |
| Camada Tauri fina, sem lógica de imagem | 7 |
| Proteção contra sobrescrita | 7 |
| Erros esperados com mensagem clara | 1, 7, 8 |
| Tela do Editor com prévia ao vivo | 8 |
| MSI, portátil, MIT, THIRD-PARTY, checksum, aviso de SmartScreen | 9 |

## Fora deste plano

- **Plano 2 — Lote e presets:** fila de workers, relatório de falhas, presets em
  disco e compartilháveis.
- **Plano 3 — Cor:** conta-gotas global, histórico, gerador de paletas,
  contraste WCAG e exportação.
- **Plano 4 — Módulos nativos opcionais:** decodificação de HEIC e de AVIF pela
  feature `heif` com `libheif`, e upscale com IA. Ficam juntos porque os dois
  dependem de binário externo, download sob demanda e verificação de licença.
