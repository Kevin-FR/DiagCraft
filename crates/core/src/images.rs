//! Portable, bounded image content. Import normalizes PNG/JPEG to embedded PNG.
//! Immutable payloads are shared by document/undo clones; crop is non-destructive.
use crate::document::{Document, Node, Point, ShapeKind};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read},
    path::Path,
    sync::{Arc, OnceLock},
};

pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PNG_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PIXELS: u64 = 4_000_000;
pub const MAX_INPUT_PIXELS: u64 = 16_000_000;
pub const MAX_DOCUMENT_PIXELS: u64 = 16_000_000;
pub const MAX_DOCUMENT_BASE64: usize = 8 * 1024 * 1024;
const MAX_SIDE: u32 = 8192;
#[derive(Clone, Debug, PartialEq)]
pub struct ImageAsset(Arc<Payload>);
#[derive(Debug)]
struct Payload {
    png: String,
    width: u32,
    height: u32,
    sha256: String,
    validated: OnceLock<Result<(), String>>,
}
impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || (self.png == other.png && self.width == other.width && self.height == other.height)
    }
}
#[derive(Serialize)]
struct AssetRef<'a> {
    png: &'a str,
    width: u32,
    height: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAsset {
    png: String,
    width: u32,
    height: u32,
}
impl Serialize for ImageAsset {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        AssetRef {
            png: self.png_base64(),
            width: self.width(),
            height: self.height(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ImageAsset {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = RawAsset::deserialize(d)?;
        let asset = Self::embedded(&raw.png, false).map_err(serde::de::Error::custom)?;
        if [raw.width, raw.height] != [asset.width(), asset.height()] {
            return Err(serde::de::Error::custom("Dimensions d’image incohérentes."));
        }
        Ok(asset)
    }
}
fn dimensions(w: u32, h: u32, limit: u64) -> Result<(), String> {
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE || u64::from(w) * u64::from(h) > limit {
        return Err(if limit == MAX_INPUT_PIXELS {
            "Image source trop grande : maximum 16 millions de pixels et 8192 px par côté."
        } else {
            "Image trop grande : maximum 4 millions de pixels et 8192 px par côté."
        }
        .into());
    }
    Ok(())
}
fn decode(bytes: &[u8], format: ImageFormat, orient: bool) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(if orient {
        128 * 1024 * 1024
    } else {
        32 * 1024 * 1024
    });
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("Image illisible : {e}"))?;
    let (w, h) = decoder.dimensions();
    dimensions(w, h, if orient { MAX_INPUT_PIXELS } else { MAX_PIXELS })?;
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|e| format!("Image illisible : {e}"))?;
    if orient {
        image.apply_orientation(orientation);
    }
    Ok(image)
}
fn resize_alpha(mut image: image::RgbaImage, width: u32, height: u32) -> image::RgbaImage {
    // Filter premultiplied pixels so transparent borders do not form dark halos.
    for p in image.pixels_mut() {
        for c in 0..3 {
            p[c] = ((u16::from(p[c]) * u16::from(p[3]) + 127) / 255) as u8;
        }
    }
    let mut out =
        image::imageops::resize(&image, width, height, image::imageops::FilterType::Triangle);
    for p in out.pixels_mut() {
        if p[3] > 0 {
            for c in 0..3 {
                p[c] = ((u32::from(p[c]) * 255 + u32::from(p[3]) / 2) / u32::from(p[3])).min(255)
                    as u8;
            }
        }
    }
    out
}
impl ImageAsset {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_INPUT_BYTES {
            return Err("Image trop volumineuse : maximum 8 Mio à l’import.".into());
        }
        let format = image::guess_format(bytes)
            .map_err(|_| "Format non pris en charge : PNG ou JPEG requis.")?;
        if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
            return Err("Format non pris en charge : PNG ou JPEG requis.".into());
        }
        Self::normalize(decode(bytes, format, true)?)
    }
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_INPUT_BYTES as u64 {
            return Err("Image trop volumineuse : maximum 8 Mio à l’import.".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_INPUT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::from_bytes(&bytes)
    }
    pub fn from_rgba(width: u32, height: u32, bytes: &[u8]) -> Result<Self, String> {
        dimensions(width, height, MAX_INPUT_PIXELS)?;
        if bytes.len() != width as usize * height as usize * 4 {
            return Err("Pixels d’image invalides.".into());
        }
        let rgba = image::RgbaImage::from_raw(width, height, bytes.to_vec())
            .ok_or("Pixels d’image invalides.")?;
        Self::normalize(DynamicImage::ImageRgba8(rgba))
    }
    fn normalize(image: DynamicImage) -> Result<Self, String> {
        let mut image = image.into_rgba8();
        let scale = (MAX_PIXELS as f64
            / (u64::from(image.width()) * u64::from(image.height())) as f64)
            .sqrt()
            .min(1.);
        if scale < 1. {
            let w = (f64::from(image.width()) * scale).floor().max(1.) as u32;
            let h = (f64::from(image.height()) * scale).floor().max(1.) as u32;
            image = resize_alpha(image, w, h);
        }
        for _ in 0..5 {
            let (width, height) = image.dimensions();
            let mut png = Cursor::new(Vec::new());
            image::ImageEncoder::write_image(
                image::codecs::png::PngEncoder::new(&mut png),
                image.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
            if png.get_ref().len() <= MAX_PNG_BYTES {
                let asset = Self::payload(png.into_inner(), width, height)?;
                let _ = asset.0.validated.set(Ok(()));
                return Ok(asset);
            }
            image = resize_alpha(image, (width * 3 / 4).max(1), (height * 3 / 4).max(1));
        }
        Err("Image PNG normalisée trop volumineuse : maximum 4 Mio.".into())
    }
    fn payload(bytes: Vec<u8>, width: u32, height: u32) -> Result<Self, String> {
        if bytes.len() > MAX_PNG_BYTES {
            return Err("Image PNG normalisée trop volumineuse : maximum 4 Mio.".into());
        }
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok(Self(Arc::new(Payload {
            png: STANDARD.encode(bytes),
            width,
            height,
            sha256,
            validated: OnceLock::new(),
        })))
    }
    /// Embedded data is always PNG, never a path, URL, SVG or alternate codec.
    pub fn from_png_base64(encoded: &str) -> Result<Self, String> {
        Self::embedded(encoded, true)
    }
    fn embedded(encoded: &str, validate: bool) -> Result<Self, String> {
        if encoded.len() > MAX_PNG_BYTES.div_ceil(3) * 4 {
            return Err("Image PNG normalisée trop volumineuse : maximum 4 Mio.".into());
        }
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| "Image base64 invalide.")?;
        // Read only the mandatory IHDR first. Whole-document pixel/byte budgets
        // are checked before decompressing any embedded payload from a file.
        if bytes.len() < 33
            || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
            || bytes[8..12] != 13u32.to_be_bytes()
            || &bytes[12..16] != b"IHDR"
        {
            return Err("Les images embarquées doivent être au format PNG.".into());
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
        dimensions(width, height, MAX_PIXELS)?;
        let asset = Self::payload(bytes, width, height)?;
        if validate {
            asset.validate()?;
        }
        Ok(asset)
    }
    fn validate(&self) -> Result<(), String> {
        self.0
            .validated
            .get_or_init(|| {
                let bytes = STANDARD
                    .decode(self.png_base64())
                    .map_err(|_| "Image base64 invalide.".to_string())?;
                let image = decode(&bytes, ImageFormat::Png, false)?;
                if [image.width(), image.height()] != [self.width(), self.height()] {
                    return Err("Dimensions d’image incohérentes.".into());
                }
                Ok(())
            })
            .clone()
    }
    pub fn import_png_base64(encoded: &str) -> Result<Self, String> {
        if encoded.len() > MAX_PNG_BYTES.div_ceil(3) * 4 {
            return Err("Image PNG normalisée trop volumineuse : maximum 4 Mio.".into());
        }
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| "Image base64 invalide.")?;
        if image::guess_format(&bytes).ok() != Some(ImageFormat::Png) {
            return Err("Les images embarquées doivent être au format PNG.".into());
        }
        Self::from_bytes(&bytes)
    }
    pub fn width(&self) -> u32 {
        self.0.width
    }
    pub fn height(&self) -> u32 {
        self.0.height
    }
    pub fn png_base64(&self) -> &str {
        &self.0.png
    }
    pub fn sha256(&self) -> &str {
        &self.0.sha256
    }
}
fn full_crop() -> [f32; 4] {
    [0., 0., 1., 1.]
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageContent {
    pub asset: ImageAsset,
    #[serde(default = "full_crop")]
    pub crop: [f32; 4],
    #[serde(default = "yes")]
    pub keep_aspect: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub alt: String,
}
impl ImageContent {
    pub fn new(asset: ImageAsset) -> Self {
        Self {
            asset,
            crop: full_crop(),
            keep_aspect: true,
            alt: String::new(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.alt.len() > 4096 {
            return Err("Description d’image trop longue.".into());
        }
        let [x, y, w, h] = self.crop;
        if self.crop.iter().any(|v| !v.is_finite())
            || x < 0.
            || y < 0.
            || w < 0.000_1
            || h < 0.000_1
            || x + w > 1.00001
            || y + h > 1.00001
        {
            return Err(
                "Recadrage invalide : rectangle normalisé inclus dans l’image requis.".into(),
            );
        }
        self.asset.validate()
    }
    pub fn ratio(&self) -> f32 {
        self.asset.width() as f32 * self.crop[2] / (self.asset.height() as f32 * self.crop[3])
    }
    pub fn display_size(&self) -> Point {
        let ratio = self.ratio();
        let w = (self.asset.width() as f32 * self.crop[2]).clamp(96., 480.);
        [w, (w / ratio).clamp(24., 480.)]
    }
    /// Nested viewport clips the normalized crop and shares SVG/PDF/canvas math.
    pub(crate) fn svg(&self, size: Point) -> String {
        let [x, y, w, h] = self.crop;
        let iw = self.asset.width() as f32;
        let ih = self.asset.height() as f32;
        // Clip to the crop itself, including when a proportional image is
        // letterboxed. A larger meet viewport would reveal discarded margins.
        let visible = if self.keep_aspect {
            let scale = (size[0] / (w * iw)).min(size[1] / (h * ih));
            [w * iw * scale, h * ih * scale]
        } else {
            size
        };
        format!(
            "<svg x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\" preserveAspectRatio=\"none\" overflow=\"hidden\"><image width=\"{iw}\" height=\"{ih}\" href=\"data:image/png;base64,{}\"/></svg>",
            (size[0] - visible[0]) / 2.,
            (size[1] - visible[1]) / 2.,
            visible[0],
            visible[1],
            x * iw,
            y * ih,
            w * iw,
            h * ih,
            self.asset.png_base64()
        )
    }
}
impl Document {
    pub fn add_image(&mut self, asset: ImageAsset, position: Point) -> Result<u64, String> {
        let mut next = self.clone();
        let content = ImageContent::new(asset);
        let size = content.display_size();
        let id = next.add_node(ShapeKind::Rectangle, position);
        let n = next.node_mut(id).unwrap();
        n.position = position;
        n.size = size;
        n.text.clear();
        n.stroke_width = 0.;
        n.appearance.fill.mode = crate::appearance::PaintMode::None;
        n.image = Some(content);
        next.validate()?;
        self.check_locked_changes(&next)?;
        *self = next;
        Ok(id)
    }
    pub fn update_image(
        &mut self,
        id: u64,
        asset: Option<ImageAsset>,
        crop: Option<[f32; 4]>,
        keep_aspect: Option<bool>,
    ) -> Result<(), String> {
        if !self.can_edit_object(id) {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let mut next = self.clone();
        let image = next
            .node_mut(id)
            .and_then(|n| n.image.as_mut())
            .ok_or("Image introuvable.")?;
        if let Some(asset) = asset {
            image.asset = asset;
            image.crop = full_crop();
        }
        if let Some(crop) = crop {
            image.crop = crop;
        }
        if let Some(keep) = keep_aspect {
            image.keep_aspect = keep;
        }
        next.validate()?;
        self.check_locked_changes(&next)?;
        *self = next;
        Ok(())
    }
    pub(crate) fn validate_image_budget(&self) -> Result<(), String> {
        let mut pixels = 0u64;
        let mut encoded = 0usize;
        for n in self
            .nodes
            .iter()
            .chain(self.pages.iter().flat_map(|p| &p.nodes))
        {
            if let Some(image) = &n.image {
                pixels = pixels.saturating_add(
                    u64::from(image.asset.width()) * u64::from(image.asset.height()),
                );
                encoded = encoded.saturating_add(image.asset.png_base64().len());
            }
        }
        if pixels > MAX_DOCUMENT_PIXELS || encoded > MAX_DOCUMENT_BASE64 {
            return Err("Trop d’images : maximum 16 millions de pixels et 8 Mio de données base64 par document.".into());
        }
        Ok(())
    }
}
impl Node {
    pub fn keeps_image_aspect(&self) -> bool {
        self.image.as_ref().is_some_and(|i| i.keep_aspect)
    }
}

/// Original synthetic image fixture; no external raster assets are distributed.
pub fn demo() -> Document {
    let mut bytes = vec![0u8; 320 * 180 * 4];
    for y in 0..180 {
        for x in 0..320 {
            let i = (y * 320 + x) * 4;
            bytes[i..i + 4].copy_from_slice(&if x < 106 {
                [21, 151, 111, 255]
            } else if x < 213 {
                [0, 125, 250, 255]
            } else {
                [247, 183, 51, 255]
            });
        }
    }
    let mut d = Document {
        title: "Illustrer les idées".into(),
        page_size: [1100., 760.],
        ..Default::default()
    };
    let asset = ImageAsset::from_rgba(320, 180, &bytes).unwrap();
    let id = d.add_image(asset, [130., 220.]).unwrap();
    let n = d.node_mut(id).unwrap();
    n.size = [640., 360.];
    n.image.as_mut().unwrap().crop = [0.1, 0., 0.8, 1.];
    n.image.as_mut().unwrap().alt = "Trois couleurs originales".into();
    n.kind = ShapeKind::Rounded;
    n.stroke_width = 2.;
    d
}
