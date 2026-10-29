//! Native image import/paste/drop, non-destructive crop and bounded PNG export.
use super::*;
use diagcraft::{
    images::{ImageAsset, ImageContent},
    raster::{self, PngOptions},
};
fn message(error: String) -> String {
    if let Some(detail) = error.strip_prefix("Image illisible : ") {
        i18n::render("Image illisible : {0}", &[("0", detail.to_owned())])
    } else {
        t(error)
    }
}
#[derive(Default)]
pub(super) struct State {
    pub png_open: bool,
    pub options: PngOptions,
    pub selection: bool,
    pub lock_ratio: bool,
}
impl DrawApp {
    pub(super) fn insert_image_assets(
        &mut self,
        assets: Vec<(ImageAsset, String)>,
        at: Option<Point>,
    ) -> Result<Vec<u64>, String> {
        if assets.is_empty() {
            return Ok(vec![]);
        }
        if assets.len() > 16 {
            return Err("Maximum de 16 images par import.".into());
        }
        let center = at.unwrap_or_else(|| self.world(self.canvas.center()));
        let mut next = self.doc.clone();
        let mut ids = vec![];
        for (index, (asset, alt)) in assets.into_iter().enumerate() {
            let size = ImageContent::new(asset.clone()).display_size();
            let offset = index as f32 * 20.;
            let position = [
                center[0] - size[0] / 2. + offset,
                center[1] - size[1] / 2. + offset,
            ]
            .map(|p| p.clamp(-100_000., 100_000.));
            let id = next.add_image(asset, position)?;
            next.node_mut(id).unwrap().image.as_mut().unwrap().alt = alt;
            ids.push(id);
        }
        next.validate()?;
        self.doc = next;
        self.selected = ids.clone();
        self.commit();
        self.show_inspector = true;
        self.status = "Images embarquées dans le document.".into();
        Ok(ids)
    }
    pub(super) fn import_image(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .set_title(t("Importer des images"))
            .add_filter(t("Images PNG et JPEG"), &["png", "jpg", "jpeg"])
            .pick_files()
        else {
            return;
        };
        if paths.len() > 16 {
            self.error = Some(t("Maximum de 16 images par import."));
            return;
        }
        let assets = paths
            .iter()
            .map(|p| {
                ImageAsset::from_path(p).map(|a| {
                    (
                        a,
                        p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>();
        if let Err(e) = assets.and_then(|assets| self.insert_image_assets(assets, None).map(|_| ()))
        {
            self.error = Some(i18n::render(
                "Import d’image impossible : {0}",
                &[("0", message(e))],
            ));
        }
    }
    /// Tests/captures never read or write the user's system clipboard.
    pub(super) fn paste_image(&mut self) -> bool {
        #[cfg(test)]
        {
            false
        }
        #[cfg(not(test))]
        {
            if self.screenshot.is_some() {
                return false;
            }
            let Ok(mut clipboard) = arboard::Clipboard::new() else {
                return false;
            };
            let Ok(data) = clipboard.get_image() else {
                return false;
            };
            let position = self.paste_position.take();
            let result = u32::try_from(data.width)
                .ok()
                .zip(u32::try_from(data.height).ok())
                .ok_or_else(|| "Pixels d’image invalides.".to_string())
                .and_then(|(w, h)| ImageAsset::from_rgba(w, h, &data.bytes))
                .and_then(|asset| {
                    self.insert_image_assets(vec![(asset, String::new())], position)
                        .map(|_| ())
                });
            if let Err(e) = result {
                self.error = Some(i18n::render(
                    "Import d’image impossible : {0}",
                    &[("0", message(e))],
                ));
            }
            true
        }
    }
    pub(super) fn drop_images(&mut self, files: Vec<egui::DroppedFile>, position: Option<Point>) {
        if self.command_dialog_blocked() || self.drag.is_some() {
            self.status = "Terminez l’édition en cours avant d’importer une image.".into();
            return;
        }
        if files.len() > 16 {
            self.error = Some(t("Maximum de 16 images par import."));
            return;
        }
        let assets = files
            .into_iter()
            .map(|f| {
                let alt = f
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or(f.name);
                let asset = if let Some(path) = f.path {
                    ImageAsset::from_path(&path)
                } else if let Some(bytes) = f.bytes {
                    ImageAsset::from_bytes(&bytes)
                } else {
                    Err("Image déposée introuvable.".into())
                };
                asset.map(|a| (a, alt))
            })
            .collect::<Result<Vec<_>, _>>();
        if let Err(e) =
            assets.and_then(|assets| self.insert_image_assets(assets, position).map(|_| ()))
        {
            self.error = Some(i18n::render(
                "Import d’image impossible : {0}",
                &[("0", message(e))],
            ));
        }
    }
    pub(super) fn image_inspector(&mut self, ui: &mut egui::Ui, id: u64) {
        let Some(n) = self.doc.node_mut(id) else {
            return;
        };
        let Some(image) = n.image.as_mut() else {
            return;
        };
        ui.label(RichText::new(t("Image embarquée")).strong());
        ui.small(format!(
            "{} × {} px",
            image.asset.width(),
            image.asset.height()
        ));
        ui.small(t(
            "Les grands imports sont réduits ; le fichier source reste intact.",
        ));
        ui.label(t("Description de l’image"));
        ui.add(
            egui::TextEdit::singleline(&mut image.alt)
                .char_limit(1000)
                .desired_width(f32::INFINITY),
        );
        ui.checkbox(
            &mut image.keep_aspect,
            t("Conserver les proportions de l’image"),
        );
        egui::CollapsingHeader::new(t("Recadrage de l’image"))
            .id_salt(("image-crop", id))
            .default_open(true)
            .show(ui, |ui| {
                let [x, y, w, h] = image.crop;
                let mut sides = [
                    x * 100.,
                    y * 100.,
                    (1. - x - w).max(0.) * 100.,
                    (1. - y - h).max(0.) * 100.,
                ];
                let mut edited = false;
                for (i, label) in [(0, "Gauche"), (1, "Haut"), (2, "Droite"), (3, "Bas")] {
                    let opposite = (i + 2) % 4;
                    let max = (99.9 - sides[opposite]).max(0.);
                    edited |= ui
                        .add(
                            egui::Slider::new(&mut sides[i], 0. ..=max)
                                .text(t(label))
                                .suffix(" %"),
                        )
                        .changed();
                }
                if edited {
                    image.crop = [
                        sides[0] / 100.,
                        sides[1] / 100.,
                        (100. - sides[0] - sides[2]) / 100.,
                        (100. - sides[1] - sides[3]) / 100.,
                    ];
                }
                if ui.button(t("Réinitialiser le recadrage")).clicked() {
                    image.crop = [0., 0., 1., 1.];
                }
                if ui.button(t("Ajuster le cadre à l’image")).clicked() {
                    n.size = image.display_size();
                    n.text_style.auto_fit = false;
                }
            });
        let replace = ui.button(t("Remplacer l’image…")).clicked();
        ui.separator();
        if replace
            && let Some(path) = rfd::FileDialog::new()
                .add_filter(t("Images PNG et JPEG"), &["png", "jpg", "jpeg"])
                .pick_file()
        {
            match ImageAsset::from_path(&path)
                .and_then(|a| self.doc.update_image(id, Some(a), None, None))
            {
                Ok(()) => {}
                Err(e) => self.error = Some(e),
            }
        }
    }
    pub(super) fn open_png_export(&mut self) {
        self.clear_connection();
        self.context_open = false;
        self.image_ui.selection = false;
        self.image_ui.lock_ratio = true;
        self.image_ui.options = PngOptions {
            page: self.doc.page_index,
            ..Default::default()
        };
        self.reset_png_size();
        self.image_ui.png_open = true;
    }
    fn png_region(&self) -> Result<[f32; 4], String> {
        raster::region(
            &self.doc,
            if self.image_ui.selection {
                &self.selected
            } else {
                &[]
            },
        )
    }
    fn reset_png_size(&mut self) {
        if let Ok(b) = self.png_region() {
            let scale = self.image_ui.options.dpi / 96.;
            self.image_ui.options.width = (b[2] * scale).round().clamp(1., u32::MAX as f32) as u32;
            self.image_ui.options.height = (b[3] * scale).round().clamp(1., u32::MAX as f32) as u32;
        }
    }
    pub(super) fn fit_png_size(&mut self) {
        let w = self.image_ui.options.width.max(1) as f64;
        let h = self.image_ui.options.height.max(1) as f64;
        let factor = 1f64
            .min(8192. / w)
            .min(8192. / h)
            .min((raster::MAX_OUTPUT_PIXELS as f64 / (w * h)).sqrt());
        self.image_ui.options.width = (w * factor).floor().max(1.) as u32;
        self.image_ui.options.height = (h * factor).floor().max(1.) as u32;
    }
    pub(super) fn png_dialog(&mut self, ctx: &egui::Context) {
        if !self.image_ui.png_open {
            return;
        }
        let modal = egui::Modal::new("png-export".into()).show(ctx, |ui| self.png_controls(ui));
        if modal.should_close() {
            self.image_ui.png_open = false;
        }
        if !modal.inner {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title(t("Exporter en PNG"))
            .add_filter("PNG", &["png"])
            .set_file_name("diagramme.png")
            .save_file()
        else {
            return;
        };
        let mut options = self.image_ui.options.clone();
        options.selection = if self.image_ui.selection {
            self.selected.clone()
        } else {
            vec![]
        };
        match raster::export(&self.doc, &options).and_then(|b| document::atomic_write(&path, &b)) {
            Ok(()) => {
                self.image_ui.png_open = false;
                self.status = "PNG exporté.".into();
            }
            Err(e) => {
                self.error = Some(i18n::render(
                    "Impossible d’exporter : {0}",
                    &[("0", message(e))],
                ))
            }
        }
    }
    fn png_controls(&mut self, ui: &mut egui::Ui) -> bool {
        ui.set_width((ui.ctx().content_rect().width() - 64.).clamp(280., 440.));
        ui.heading(t("Exporter en PNG"));
        let old = self.image_ui.selection;
        ui.radio_value(&mut self.image_ui.selection, false, t("Page actuelle"));
        ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
            ui.radio_value(&mut self.image_ui.selection, true, t("Sélection actuelle"));
        });
        if old != self.image_ui.selection {
            self.reset_png_size();
        }
        ui.checkbox(
            &mut self.image_ui.lock_ratio,
            t("Lier les dimensions de sortie"),
        );
        let previous = [self.image_ui.options.width, self.image_ui.options.height];
        egui::Grid::new("png-size").show(ui, |ui| {
            ui.label(t("Largeur en pixels"));
            ui.add(
                egui::DragValue::new(&mut self.image_ui.options.width)
                    .clamp_existing_to_range(false)
                    .range(1..=8192),
            );
            ui.end_row();
            ui.label(t("Hauteur en pixels"));
            ui.add(
                egui::DragValue::new(&mut self.image_ui.options.height)
                    .clamp_existing_to_range(false)
                    .range(1..=8192),
            );
            ui.end_row();
        });
        if self.image_ui.lock_ratio
            && let Ok(b) = self.png_region()
        {
            if previous[0] != self.image_ui.options.width {
                self.image_ui.options.height = (self.image_ui.options.width as f32 * b[3] / b[2])
                    .round()
                    .clamp(1., u32::MAX as f32)
                    as u32;
            } else if previous[1] != self.image_ui.options.height {
                self.image_ui.options.width = (self.image_ui.options.height as f32 * b[2] / b[3])
                    .round()
                    .clamp(1., u32::MAX as f32)
                    as u32;
            }
        }
        if ui
            .add(
                egui::Slider::new(&mut self.image_ui.options.dpi, 36. ..=1200.)
                    .text(t("Résolution (ppp)")),
            )
            .changed()
        {
            self.reset_png_size();
        }
        if ui.button(t("Adapter aux limites PNG")).clicked() {
            self.fit_png_size();
        }
        ui.checkbox(
            &mut self.image_ui.options.transparent,
            t("Fond transparent"),
        );
        ui.small(t(
            "Maximum 16 millions de pixels. Le PNG exporte une seule page ou la sélection visible.",
        ));
        let validation = self
            .image_ui
            .options
            .validate()
            .and_then(|_| self.png_region().map(|_| ()));
        if let Err(e) = &validation {
            ui.colored_label(ERROR_TEXT, t(e));
        }
        ui.horizontal(|ui| {
            if ui.button(t("Annuler")).clicked() {
                self.image_ui.png_open = false;
            }
            ui.add_enabled(validation.is_ok(), egui::Button::new(t("Exporter…")))
                .clicked()
        })
        .inner
    }
    pub(super) fn prepare_images_capture(&mut self, png: bool) {
        self.doc = diagcraft::images::demo();
        let id = self.doc.nodes[0].id;
        self.selected = vec![id];
        self.history = History::new(&self.doc);
        self.show_library = false;
        self.show_inspector = true;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        self.fit_next = true;
        if png {
            self.open_png_export();
            self.image_ui.selection = true;
            self.reset_png_size();
            self.image_ui.options.transparent = true;
        }
    }
}
